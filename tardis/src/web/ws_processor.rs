#[cfg(feature = "cluster")]
pub mod cluster_protocol;
use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
#[cfg(feature = "cluster")]
use crate::cluster::cluster_hashmap::ClusterStaticHashMap;

use std::sync::Arc;
use std::{collections::HashMap, num::NonZeroUsize};

use futures::{Future, SinkExt, StreamExt};
use lru::LruCache;
use poem::web::websocket::{BoxWebSocketUpgraded, CloseCode, Message, WebSocket};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tracing::warn;
use tracing::{debug, trace};

use crate::{tardis_static, TardisFuns};

pub const WS_SYSTEM_EVENT_INFO: &str = "__sys_info__";
pub const WS_SYSTEM_EVENT_AVATAR_ADD: &str = "__sys_avatar_add__";
pub const WS_SYSTEM_EVENT_AVATAR_DEL: &str = "__sys_avatar_del__";
pub const WS_SYSTEM_EVENT_ERROR: &str = "__sys_error__";
#[derive(Debug, Clone, Copy)]
enum MessageSendState {
    Success,
    Sending,
    Fail,
}

/// # Safety:
/// It's safe for we set the cache size manually
#[allow(clippy::undocumented_unsafe_blocks)]
pub const WS_SENDER_CACHE_SIZE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(1000000) };

tardis_static! {
    // Websocket instance Id -> Avatars
    #[cfg(not(feature = "cluster"))]
    pub ws_insts_mapping_avatars: Arc<tokio::sync::RwLock<HashMap<String, Vec<String>>>>;
    #[cfg(feature = "cluster")]
    pub ws_insts_mapping_avatars: ClusterStaticHashMap<String, Vec<String>> = ClusterStaticHashMap::<String, Vec<String>>::builder("tardis/avatar")
        .modify_handler("del_avatar", |v, modify| {
            if let Some(del) = modify.as_str() {
                v.retain(|value| *value != del);
            }
        })
        .modify_handler("add_avatar", |v, modify| {
            if let Some(add) = modify.as_str() {
                v.push(add.to_string() );
            }
        })
        .build();
}
lazy_static! {
    // Single instance reply guard
    static ref REPLY_ONCE_GUARD: Arc<Mutex<LruCache<String, MessageSendState>>> = Arc::new(Mutex::new(LruCache::new(WS_SENDER_CACHE_SIZE)));
}

pub fn ws_echo<PF, PT, CF, CT>(avatars: String, ext: HashMap<String, String>, websocket: WebSocket, process_fun: PF, close_fun: CF) -> BoxWebSocketUpgraded
where
    PF: Fn(String, String, HashMap<String, String>) -> PT + Send + Sync + 'static,
    PT: Future<Output = Option<String>> + Send + 'static,
    CF: Fn(Option<(CloseCode, String)>, HashMap<String, String>) -> CT + Send + Sync + 'static,
    CT: Future<Output = ()> + Send + 'static,
{
    websocket
        .on_upgrade(|mut socket| async move {
            while let Some(Ok(message)) = socket.next().await {
                match message {
                    Message::Text(text) => {
                        trace!("[Tardis.WebServer] WS message receive: {} by {}", text, &avatars);
                        if let Some(msg) = process_fun(avatars.clone(), text, ext.clone()).await {
                            trace!("[Tardis.WebServer] WS message send: {} to {}", msg, &avatars);
                            if let Err(error) = socket.send(Message::Text(msg.clone())).await {
                                warn!("[Tardis.WebServer] WS message send failed, message {msg} to {}: {error}", &avatars);
                                break;
                            }
                        }
                    }
                    Message::Close(msg) => {
                        trace!("[Tardis.WebServer] WS message receive: close {:?}", msg);
                        close_fun(msg, ext.clone()).await
                    }
                    Message::Binary(_) => {
                        warn!("[Tardis.WebServer] WS message receive: the binary type is not implemented");
                    }
                    Message::Ping(_) => {
                        warn!("[Tardis.WebServer] WS message receive: the ping type is not implemented");
                    }
                    Message::Pong(_) => {
                        warn!("[Tardis.WebServer] WS message receive: the pong type is not implemented");
                    }
                }
            }
        })
        .boxed()
}

fn ws_send_to_channel<TX>(send_msg: TardisWebsocketMgrMessage, inner_sender: Arc<TX>, hook: Arc<impl WsHooks>)
where
    TX: WsBroadcastSender,
{
    let task = async move {
        let id = send_msg.msg_id.clone();
        if let Err(e) = inner_sender.send(send_msg).await {
            warn!("[Tardis.Ws] send message encounter an error");
            hook.on_fail(id, e).await;
        }
    };
    tokio::spawn(task);
}

pub fn ws_send_error_to_channel<TX>( error_message: &str, msg_id: &str, from_avatar: &str, from_inst_id: &str, inner_sender: Arc<TX>, hook: Arc<impl WsHooks>)
where
    TX: WsBroadcastSender,
{
    let send_msg = TardisWebsocketMgrMessage {
        msg_id: msg_id.to_string(),
        msg: json!(error_message),
        from_avatar: from_avatar.to_string(),
        to_avatars: vec![from_avatar.to_string()],
        event: Some(WS_SYSTEM_EVENT_ERROR.to_string()),
        ignore_self: false,
        ignore_avatars: vec![],
        from_inst_id: from_inst_id.to_string(),
        echo: true,
    };
    warn!("[Tardis.WebServer] WS message receive by {:?} failed: {error_message}", from_avatar);
    ws_send_to_channel(send_msg, inner_sender, hook)
}

pub trait WsBroadcastSender: Send + Sync + 'static {
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TardisWebsocketMgrMessage>;
    fn send(&self, msg: TardisWebsocketMgrMessage) -> impl Future<Output = TardisResult<()>> + Send;
}

impl WsBroadcastSender for tokio::sync::broadcast::Sender<TardisWebsocketMgrMessage> {
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TardisWebsocketMgrMessage> {
        self.subscribe()
    }

    async fn send(&self, msg: TardisWebsocketMgrMessage) -> TardisResult<()> {
        let _ = self.send(msg).map_err(|_| TardisError::internal_error("tokio channel send error", ""))?;
        Ok(())
    }
}

pub trait WsHooks: Sync + Send + 'static {
    fn on_process(&self, req: TardisWebsocketReq) -> impl Future<Output = Option<TardisWebsocketResp>> + Send;
    fn on_close(&self, message: Option<(CloseCode, String)>) -> impl Future<Output = ()> + Send {
        if let Some((code, reason)) = message {
            tracing::debug!("[Tardis.Ws] connection closed {code:?} for reason: {reason}");
        }
        async {}
    }
    fn on_fail(&self, id: String, error: TardisError) -> impl Future<Output = ()> + Send {
        tracing::warn!("[Tardis.Ws] fail to send out message [{id}], reason: {error}");
        async {}
    }
    fn on_success(&self, id: String) -> impl Future<Output = ()> + Send {
        tracing::debug!("[Tardis.Ws] success to send out message [{id}]");
        async {}
    }
}

pub struct WsBroadcastContext<S, H> {
    sender: S,
    hooks: H,
    avatar_self: String,
}

pub async fn handle_req(req_msg: TardisWebsocketReq, mgr_node: bool, ) {
    {
        let msg_id = req_msg.msg_id.as_ref().unwrap_or(&TardisFuns::field.nanoid()).to_string();
        // Security check
        if !mgr_node && req_msg.spec_inst_id.is_some() {
            ws_send_error_to_channel(
                
                "spec_inst_id can only be specified on the management node",
                &msg_id,
                &avatar_self,
                &inst_id,
                inner_sender.clone(),
                hooks.clone(),
            );
            continue;
        }
        if !mgr_node && !current_avatars.contains(&req_msg.from_avatar) {
            ws_send_error_to_channel( "from_avatar is illegal", &msg_id, &avatar_self, &inst_id, inner_sender.clone(), hooks.clone());
            continue;
        }
        // System process
        if req_msg.event == Some(WS_SYSTEM_EVENT_INFO.to_string()) {
            let Ok(msg) = TardisFuns::json
                .obj_to_json(&TardisWebsocketInstInfo {
                    inst_id: inst_id.clone(),
                    avatars: current_avatars,
                    mgr_node,
                    subscribe_mode,
                })
                .map_err(|error| {
                    crate::log::error!(
                        "[Tardis.WebServer] can't serialize {struct_name}, error: {error}",
                        struct_name = stringify!(TardisWebsocketInstInfo)
                    );
                    ws_send_error_to_channel( "message illegal", &msg_id, &avatar_self, &inst_id, inner_sender.clone(), hooks.clone());
                })
            else {
                continue;
            };
            let send_msg = TardisWebsocketMgrMessage {
                msg_id,
                msg,
                from_avatar: req_msg.from_avatar.clone(),
                to_avatars: vec![req_msg.from_avatar],
                event: req_msg.event,
                ignore_self: false,
                ignore_avatars: vec![],
                from_inst_id: if let Some(spec_inst_id) = req_msg.spec_inst_id { spec_inst_id } else { inst_id.clone() },
                echo: true,
            };
            ws_send_to_channel(send_msg, inner_sender.clone(), hooks.clone());
            continue;
            // For security reasons, adding an avatar needs to be handled by the management node
        } else if mgr_node && req_msg.event == Some(WS_SYSTEM_EVENT_AVATAR_ADD.to_string()) {
            let Some(new_avatar) = req_msg.msg.as_str() else {
                ws_send_error_to_channel( "msg is not a string", &msg_id, &avatar_self, &inst_id, inner_sender.clone(), hooks.clone());
                continue;
            };
            let Some(ref spec_inst_id) = req_msg.spec_inst_id else {
                ws_send_error_to_channel( "spec_inst_id is not specified", &msg_id, &avatar_self, &inst_id, inner_sender.clone(), hooks.clone());
                continue;
            };
            #[cfg(feature = "cluster")]
            {
                let Ok(Some(_)) = insts_in_send.get(spec_inst_id.clone()).await else {
                    ws_send_error_to_channel( "spec_inst_id not found", &msg_id, &avatar_self, &inst_id, inner_sender.clone(), hooks.clone());
                    continue;
                };
                trace!("[Tardis.WebServer] WS message add avatar {}:{} to {}", &msg_id, &new_avatar, &spec_inst_id);
                let _ = insts_in_send.modify(spec_inst_id.clone(), "add_avatar", json!(new_avatar)).await;
                continue;
            }
            #[cfg(not(feature = "cluster"))]
            {
                let mut write_locked = insts_in_send.write().await;
                let Some(inst) = write_locked.get_mut(spec_inst_id) else {
                    ws_send_error_to_channel( "spec_inst_id not found", &msg_id, &avatar_self, &inst_id, &inner_sender);
                    continue;
                };
                inst.push(new_avatar.to_string());
                drop(write_locked);
                trace!("[Tardis.WebServer] WS message add avatar {}:{} to {}", msg_id, new_avatar, spec_inst_id);
            }
        } else if req_msg.event == Some(WS_SYSTEM_EVENT_AVATAR_DEL.to_string()) {
            #[cfg(feature = "cluster")]
            {
                let Ok(Some(_)) = insts_in_send.get(inst_id.clone()).await else {
                    ws_send_error_to_channel( "spec_inst_id not found", &msg_id, &avatar_self, &inst_id, inner_sender.clone(), hooks.clone());
                    continue;
                };
                let Some(del_avatar) = req_msg.msg.as_str() else {
                    ws_send_error_to_channel( "msg is not a string", &msg_id, &avatar_self, &inst_id, inner_sender.clone(), hooks.clone());
                    continue;
                };
                let _ = insts_in_send.modify(inst_id.clone(), "del_avatar", json!(del_avatar)).await;
                trace!("[Tardis.WebServer] WS message delete avatar {},{} to {}", msg_id, del_avatar, &inst_id);
            }
            #[cfg(not(feature = "cluster"))]
            {
                let Some(del_avatar) = req_msg.msg.as_str() else {
                    ws_send_error_to_channel( "msg is not a string", &msg_id, &avatar_self, &inst_id, &inner_sender);
                    continue;
                };
                let mut write_locked = insts_in_send.write().await;
                let Some(inst) = write_locked.get_mut(&inst_id) else {
                    ws_send_error_to_channel( "spec_inst_id not found", &msg_id, &avatar_self, &inst_id, &inner_sender);
                    continue;
                };
                inst.retain(|value| *value != del_avatar);
                drop(write_locked);
                trace!("[Tardis.WebServer] WS message delete avatar {},{} to {}", msg_id, del_avatar, &inst_id);
            }
            continue;
        }
        if let Some(resp_msg) = hooks.on_process(req_msg.clone()).await {
            trace!(
                "[Tardis.WebServer] WS message send to channel: {},{} to {:?} ignore {:?}",
                msg_id,
                resp_msg.msg,
                resp_msg.to_avatars,
                resp_msg.ignore_avatars
            );
            let send_msg = TardisWebsocketMgrMessage {
                msg_id,
                msg: resp_msg.msg,
                from_avatar: req_msg.from_avatar,
                to_avatars: resp_msg.to_avatars,
                event: req_msg.event,
                ignore_self: req_msg.ignore_self.unwrap_or(true),
                ignore_avatars: resp_msg.ignore_avatars,
                from_inst_id: if let Some(spec_inst_id) = req_msg.spec_inst_id { spec_inst_id } else { inst_id.clone() },
                echo: false,
            };
            let hooks = hooks.clone();
            ws_send_to_channel(send_msg, inner_sender.clone(), hooks.clone());
        };
    }
}
#[tracing::instrument(skip(websocket, inner_sender, hooks))]
pub async fn ws_broadcast(
    avatars: Vec<String>,
    mgr_node: bool,
    subscribe_mode: bool,
    websocket: WebSocket,
    inner_sender: impl WsBroadcastSender,
    hooks: impl WsHooks,
) -> BoxWebSocketUpgraded {
    let arc_hooks = Arc::new(hooks);
    let arc_inner_sender = Arc::new(inner_sender);
    websocket
        .on_upgrade(move |socket| async move {
            let mut inner_receiver = arc_inner_sender.subscribe();
            // corresponded to the current ws connection
            let inst_id = TardisFuns::field.nanoid();
            let current_receive_inst_id = inst_id.clone();
            #[cfg(feature = "cluster")]
            let _ = ws_insts_mapping_avatars().insert(inst_id.clone(), avatars).await;
            #[cfg(feature = "cluster")]
            let insts_in_send = ws_insts_mapping_avatars().clone();
            #[cfg(not(feature = "cluster"))]
            let _ = ws_insts_mapping_avatars().write().await.insert(inst_id.clone(), avatars);
            #[cfg(not(feature = "cluster"))]
            let insts_in_send = ws_insts_mapping_avatars().clone();
            let (mut ws_sink, mut ws_stream) = socket.split();
            let ws_closed = tokio_util::sync::CancellationToken::new();
            let ws_closed_notifier = ws_closed.clone();
            debug!("[Tardis.WebServer] WS new connection {inst_id}");
            let hooks = arc_hooks.clone();
            let inner_sender = arc_inner_sender.clone();
            tokio::spawn(async move {
                // message inbound
                while let Some(Ok(message)) = ws_stream.next().await {
                    match message {
                        Message::Text(text) => {
                            #[cfg(feature = "cluster")]
                            let Ok(Some(current_avatars)) = insts_in_send.get(inst_id.clone()).await
                            else {
                                warn!("[Tardis.WebServer] insts_in_send of inst_id {inst_id} not found");
                                continue;
                            };
                            #[cfg(not(feature = "cluster"))]
                            let Some(current_avatars) = insts_in_send.read().await.get(&inst_id).cloned() else {
                                warn!("[Tardis.WebServer] insts_in_send of inst_id {inst_id} not found");
                                continue;
                            };
                            trace!(
                                "[Tardis.WebServer] WS message receive text: {} by {:?} {}",
                                text,
                                current_avatars,
                                if mgr_node { "[MGR]" } else { "" }
                            );
                            let Some(avatar_self) = current_avatars.first().cloned() else {
                                warn!("[Tardis.WebServer] current_avatars is empty");
                                continue;
                            };
                            match TardisFuns::json.str_to_obj::<TardisWebsocketReq>(&text) {
                                Err(_) => {
                                    ws_send_error_to_channel( "message illegal", "", &avatar_self, &inst_id, inner_sender.clone(), hooks.clone());
                                    continue;
                                }
                                Ok(req_msg) => {
                                    let msg_id = req_msg.msg_id.as_ref().unwrap_or(&TardisFuns::field.nanoid()).to_string();
                                    // Security check
                                    if !mgr_node && req_msg.spec_inst_id.is_some() {
                                        ws_send_error_to_channel(
                                            "spec_inst_id can only be specified on the management node",
                                            &msg_id,
                                            &avatar_self,
                                            &inst_id,
                                            inner_sender.clone(),
                                            hooks.clone(),
                                        );
                                        continue;
                                    }
                                    if !mgr_node && !current_avatars.contains(&req_msg.from_avatar) {
                                        ws_send_error_to_channel( "from_avatar is illegal", &msg_id, &avatar_self, &inst_id, inner_sender.clone(), hooks.clone());
                                        continue;
                                    }
                                    // System process
                                    if req_msg.event == Some(WS_SYSTEM_EVENT_INFO.to_string()) {
                                        let Ok(msg) = TardisFuns::json
                                            .obj_to_json(&TardisWebsocketInstInfo {
                                                inst_id: inst_id.clone(),
                                                avatars: current_avatars,
                                                mgr_node,
                                                subscribe_mode,
                                            })
                                            .map_err(|error| {
                                                crate::log::error!(
                                                    "[Tardis.WebServer] can't serialize {struct_name}, error: {error}",
                                                    struct_name = stringify!(TardisWebsocketInstInfo)
                                                );
                                                ws_send_error_to_channel( "message illegal", &msg_id, &avatar_self, &inst_id, inner_sender.clone(), hooks.clone());
                                            })
                                        else {
                                            continue;
                                        };
                                        let send_msg = TardisWebsocketMgrMessage {
                                            msg_id,
                                            msg,
                                            from_avatar: req_msg.from_avatar.clone(),
                                            to_avatars: vec![req_msg.from_avatar],
                                            event: req_msg.event,
                                            ignore_self: false,
                                            ignore_avatars: vec![],
                                            from_inst_id: if let Some(spec_inst_id) = req_msg.spec_inst_id { spec_inst_id } else { inst_id.clone() },
                                            echo: true,
                                        };
                                        ws_send_to_channel(send_msg, inner_sender.clone(), hooks.clone());
                                        continue;
                                        // For security reasons, adding an avatar needs to be handled by the management node
                                    } else if mgr_node && req_msg.event == Some(WS_SYSTEM_EVENT_AVATAR_ADD.to_string()) {
                                        let Some(new_avatar) = req_msg.msg.as_str() else {
                                            ws_send_error_to_channel( "msg is not a string", &msg_id, &avatar_self, &inst_id, inner_sender.clone(), hooks.clone());
                                            continue;
                                        };
                                        let Some(ref spec_inst_id) = req_msg.spec_inst_id else {
                                            ws_send_error_to_channel( "spec_inst_id is not specified", &msg_id, &avatar_self, &inst_id, inner_sender.clone(), hooks.clone());
                                            continue;
                                        };
                                        #[cfg(feature = "cluster")]
                                        {
                                            let Ok(Some(_)) = insts_in_send.get(spec_inst_id.clone()).await else {
                                                ws_send_error_to_channel( "spec_inst_id not found", &msg_id, &avatar_self, &inst_id, inner_sender.clone(), hooks.clone());
                                                continue;
                                            };
                                            trace!("[Tardis.WebServer] WS message add avatar {}:{} to {}", &msg_id, &new_avatar, &spec_inst_id);
                                            let _ = insts_in_send.modify(spec_inst_id.clone(), "add_avatar", json!(new_avatar)).await;
                                            continue;
                                        }
                                        #[cfg(not(feature = "cluster"))]
                                        {
                                            let mut write_locked = insts_in_send.write().await;
                                            let Some(inst) = write_locked.get_mut(spec_inst_id) else {
                                                ws_send_error_to_channel( "spec_inst_id not found", &msg_id, &avatar_self, &inst_id, &inner_sender);
                                                continue;
                                            };
                                            inst.push(new_avatar.to_string());
                                            drop(write_locked);
                                            trace!("[Tardis.WebServer] WS message add avatar {}:{} to {}", msg_id, new_avatar, spec_inst_id);
                                        }
                                    } else if req_msg.event == Some(WS_SYSTEM_EVENT_AVATAR_DEL.to_string()) {
                                        #[cfg(feature = "cluster")]
                                        {
                                            let Ok(Some(_)) = insts_in_send.get(inst_id.clone()).await else {
                                                ws_send_error_to_channel( "spec_inst_id not found", &msg_id, &avatar_self, &inst_id, inner_sender.clone(), hooks.clone());
                                                continue;
                                            };
                                            let Some(del_avatar) = req_msg.msg.as_str() else {
                                                ws_send_error_to_channel( "msg is not a string", &msg_id, &avatar_self, &inst_id, inner_sender.clone(), hooks.clone());
                                                continue;
                                            };
                                            let _ = insts_in_send.modify(inst_id.clone(), "del_avatar", json!(del_avatar)).await;
                                            trace!("[Tardis.WebServer] WS message delete avatar {},{} to {}", msg_id, del_avatar, &inst_id);
                                        }
                                        #[cfg(not(feature = "cluster"))]
                                        {
                                            let Some(del_avatar) = req_msg.msg.as_str() else {
                                                ws_send_error_to_channel( "msg is not a string", &msg_id, &avatar_self, &inst_id, &inner_sender);
                                                continue;
                                            };
                                            let mut write_locked = insts_in_send.write().await;
                                            let Some(inst) = write_locked.get_mut(&inst_id) else {
                                                ws_send_error_to_channel( "spec_inst_id not found", &msg_id, &avatar_self, &inst_id, &inner_sender);
                                                continue;
                                            };
                                            inst.retain(|value| *value != del_avatar);
                                            drop(write_locked);
                                            trace!("[Tardis.WebServer] WS message delete avatar {},{} to {}", msg_id, del_avatar, &inst_id);
                                        }
                                        continue;
                                    }
                                    if let Some(resp_msg) = hooks.on_process(req_msg.clone()).await {
                                        trace!(
                                            "[Tardis.WebServer] WS message send to channel: {},{} to {:?} ignore {:?}",
                                            msg_id,
                                            resp_msg.msg,
                                            resp_msg.to_avatars,
                                            resp_msg.ignore_avatars
                                        );
                                        let send_msg = TardisWebsocketMgrMessage {
                                            msg_id,
                                            msg: resp_msg.msg,
                                            from_avatar: req_msg.from_avatar,
                                            to_avatars: resp_msg.to_avatars,
                                            event: req_msg.event,
                                            ignore_self: req_msg.ignore_self.unwrap_or(true),
                                            ignore_avatars: resp_msg.ignore_avatars,
                                            from_inst_id: if let Some(spec_inst_id) = req_msg.spec_inst_id { spec_inst_id } else { inst_id.clone() },
                                            echo: false,
                                        };
                                        let hooks = hooks.clone();
                                        ws_send_to_channel(send_msg, inner_sender.clone(), hooks.clone());
                                    };
                                }
                            }
                        }
                        Message::Close(msg) => {
                            trace!("[Tardis.WebServer] WS message receive: close {:?}", msg);
                            hooks.on_close(msg).await
                        }
                        Message::Binary(_) => {
                            warn!("[Tardis.WebServer] WS message receive: the binary type is not implemented");
                        }
                        Message::Ping(_) => {
                            warn!("[Tardis.WebServer] WS message receive: the ping type is not implemented");
                        }
                        Message::Pong(_) => {
                            warn!("[Tardis.WebServer] WS message receive: the pong type is not implemented");
                        }
                    }
                }
                ws_closed_notifier.cancel();
            });

            let reply_once_guard = REPLY_ONCE_GUARD.clone();

            tokio::spawn(async move {
                debug!("[Tardis.WebServer] WS tx side: new connection {current_receive_inst_id}");
                'poll_next_message: loop {
                    let mgr_message = tokio::select! {
                        _ = ws_closed.cancelled() => {
                            trace!("[Tardis.WebServer] WS message receive: connection closed");
                            return
                        }
                        next = inner_receiver.recv() => {
                            match next {
                                Ok(message) => message,
                                Err(e) => {
                                    warn!("[Tardis.WebServer] WS message receive from channel failed: {e}");
                                    return
                                }
                            }
                        }
                    };

                    tracing::trace!(inst_id = current_receive_inst_id, "[Tardis.WebServer] inner receiver receive message {mgr_message:?}");

                    #[cfg(feature = "cluster")]
                    let Ok(Some(current_avatars)) = ({ ws_insts_mapping_avatars().get(current_receive_inst_id.clone()).await }) else {
                        warn!("[Tardis.WebServer] Instance id {current_receive_inst_id} not found");
                        continue;
                    };
                    #[cfg(not(feature = "cluster"))]
                    let Some(current_avatars) = ({ ws_insts_mapping_avatars().read().await.get(&current_receive_inst_id).cloned() }) else {
                        warn!("[Tardis.WebServer] Instance id {current_receive_inst_id} not found");
                        continue;
                    };
                    // only self
                    if mgr_message.echo && current_receive_inst_id != mgr_message.from_inst_id {
                        continue;
                    }
                    // except self
                    if mgr_message.ignore_self && current_receive_inst_id == mgr_message.from_inst_id {
                        continue;
                    }

                    tracing::trace!("[Tardis.WebServer] inner receiver receive message {mgr_message:?}");
                    if
                    // send to all
                    mgr_message.to_avatars.is_empty() && mgr_message.ignore_avatars.is_empty()
                             // send to targets that match the current avatars
                           || !mgr_message.to_avatars.is_empty() && mgr_message.to_avatars.iter().any(|avatar| current_avatars.contains(avatar))
                        // send to targets that NOT match the current avatars
                        || !mgr_message.ignore_avatars.is_empty() && mgr_message.ignore_avatars.iter().all(|avatar| current_avatars.contains(avatar))
                    {
                        let Ok(resp_msg) = (if mgr_node {
                            TardisFuns::json.obj_to_string(&mgr_message)
                        } else {
                            TardisFuns::json.obj_to_string(&TardisWebsocketMessage {
                                msg_id: mgr_message.msg_id.clone(),
                                msg: mgr_message.msg.clone(),
                                event: mgr_message.event.clone(),
                            })
                        }) else {
                            warn!("[Tardis.WebServer] Cannot serialize {:?} into json", mgr_message);
                            continue;
                        };
                        let cache_id = if !subscribe_mode {
                            let id = format!("{}{:?}", mgr_message.msg_id, &current_avatars);
                            'poll_sending_state: loop {
                                let mut lock = reply_once_guard.lock().await;
                                match lock.get(&id) {
                                    Some(MessageSendState::Success) => continue 'poll_next_message,
                                    Some(MessageSendState::Sending) => tokio::task::yield_now().await,
                                    _ => {
                                        lock.put(id.clone(), MessageSendState::Sending);
                                        break 'poll_sending_state;
                                    }
                                }
                            }
                            Some(id)
                        } else {
                            None
                        };
                        if let Err(error) = ws_sink.send(Message::Text(resp_msg)).await {
                            if let Some(cache_id) = cache_id {
                                let mut lock = reply_once_guard.lock().await;
                                lock.put(cache_id, MessageSendState::Fail);
                            }
                            if error.to_string() != "Connection closed normally" {
                                warn!(
                                    "[Tardis.WebServer] WS message send: {}:{} to {:?} ignore {:?} failed: {error}",
                                    mgr_message.msg_id, mgr_message.msg, mgr_message.to_avatars, mgr_message.ignore_avatars
                                );
                            }
                            break;
                        } else if let Some(cache_id) = cache_id {
                            let mut lock = reply_once_guard.lock().await;
                            lock.put(cache_id, MessageSendState::Success);
                        }
                    }
                }
            });
        })
        .boxed()
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct TardisWebsocketReq {
    pub msg_id: Option<String>,
    pub msg: Value,
    pub from_avatar: String,
    pub to_avatars: Option<Vec<String>>,
    pub event: Option<String>,
    pub ignore_self: Option<bool>,
    pub spec_inst_id: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TardisWebsocketResp {
    pub msg: Value,
    pub to_avatars: Vec<String>,
    pub ignore_avatars: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TardisWebsocketMgrMessage {
    pub msg_id: String,
    pub msg: Value,
    pub from_inst_id: String,
    pub from_avatar: String,
    pub to_avatars: Vec<String>,
    pub event: Option<String>,
    pub ignore_self: bool,
    pub echo: bool,
    pub ignore_avatars: Vec<String>,
}

impl TardisWebsocketMgrMessage {
    pub fn into_req(self, msg_id: String, msg: Value, current_avatar: String, to_avatars: Option<Vec<String>>) -> TardisWebsocketReq {
        TardisWebsocketReq {
            msg_id: Some(msg_id),
            msg,
            from_avatar: current_avatar,
            to_avatars,
            event: self.event,
            ignore_self: Some(self.ignore_self),
            spec_inst_id: Some(self.from_inst_id),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TardisWebsocketMessage {
    pub msg_id: String,
    pub msg: Value,
    pub event: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct TardisWebsocketInstInfo {
    pub inst_id: String,
    pub avatars: Vec<String>,
    pub mgr_node: bool,
    pub subscribe_mode: bool,
}
