#[cfg(feature = "cluster")]
pub mod cluster_protocol;
// pub mod connection_avatar_router;
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
    fn on_process(&self, req: TardisWebsocketReq, context: &WsBroadcastContext) -> impl Future<Output = Option<TardisWebsocketResp>> + Send;
    fn on_close(&self, message: Option<(CloseCode, String)>, _context: &WsBroadcastContext) -> impl Future<Output = ()> + Send {
        if let Some((code, reason)) = message {
            tracing::debug!("[Tardis.Ws] connection closed {code:?} for reason: {reason}");
        }
        async {}
    }
    fn on_fail(&self, id: String, error: TardisError, _context: &WsBroadcastContext) -> impl Future<Output = ()> + Send {
        tracing::warn!("[Tardis.Ws] fail to send out message [{id}], reason: {error}");
        async {}
    }
    fn on_success(&self, id: String, _context: &WsBroadcastContext) -> impl Future<Output = ()> + Send {
        tracing::debug!("[Tardis.Ws] success to send out message [{id}]");
        async {}
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WsBroadcastContext {
    pub inst_id: String,
    pub mgr_node: bool,
    pub subscribe_mode: bool,
}

impl WsBroadcastContext {
    pub fn new(mgr_node: bool, subscribe_mode: bool) -> Self {
        Self {
            inst_id: TardisFuns::field.nanoid(),
            mgr_node,
            subscribe_mode,
        }
    }
}

pub struct WsBroadcast<S, H> {
    inner_sender: Arc<S>,
    hooks: Arc<H>,
    context: Arc<WsBroadcastContext>,
}

impl<S, H> std::fmt::Debug for WsBroadcast<S, H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WsBroadcast").field("context", &self.context).finish_non_exhaustive()
    }
}

impl<S, H> WsBroadcast<S, H>
where
    S: WsBroadcastSender,
    H: WsHooks,
{
    pub fn new(sender: S, hooks: H, context: WsBroadcastContext) -> Self {
        Self {
            inner_sender: Arc::new(sender),
            hooks: Arc::new(hooks),
            context: Arc::new(context),
        }
    }
    pub fn send_to_channel(&self, send_msg: TardisWebsocketMgrMessage) {
        let inner_sender = self.inner_sender.clone();
        let hook = self.hooks.clone();
        let context = self.context.clone();
        let task = async move {
            let id = send_msg.msg_id.clone();
            if let Err(e) = inner_sender.send(send_msg).await {
                warn!("[Tardis.Ws] send message encounter an error");
                hook.on_fail(id, e, &context).await;
            } else {
                hook.on_success(id, &context).await;
            }
        };
        tokio::spawn(task);
    }
    pub fn send_error_to_channel(&self, error_message: &str, from_avatar: &str, msg_id: Option<String>) {
        let send_msg = TardisWebsocketMgrMessage {
            msg_id: msg_id.unwrap_or_default(),
            msg: json!(error_message),
            from_avatar: from_avatar.to_string(),
            to_avatars: vec![from_avatar.to_string()],
            event: Some(WS_SYSTEM_EVENT_ERROR.to_string()),
            ignore_self: false,
            ignore_avatars: vec![],
            from_inst_id: self.context.inst_id.clone(),
            echo: true,
        };
        warn!("[Tardis.WebServer] WS message receive by {:?} failed: {error_message}", from_avatar);
        self.send_to_channel(send_msg);
    }
    pub async fn handle_req(&self, req_msg: TardisWebsocketReq) -> Result<(), String> {
        let insts_in_send = ws_insts_mapping_avatars().clone();
        let inst_id = self.context.inst_id.clone();
        #[cfg(feature = "cluster")]
        let Ok(Some(current_avatars)) = insts_in_send.get(inst_id.clone()).await
        else {
            warn!("[Tardis.WebServer] insts_in_send of inst_id {inst_id} not found");
            return Ok(());
        };
        #[cfg(not(feature = "cluster"))]
        let Some(current_avatars) = insts_in_send.read().await.get(&inst_id).cloned() else {
            warn!("[Tardis.WebServer] insts_in_send of inst_id {inst_id} not found");
            return Ok(());
        };
        let msg_id = req_msg.msg_id.as_ref().unwrap_or(&TardisFuns::field.nanoid()).to_string();
        // Security check
        if !self.context.mgr_node && req_msg.spec_inst_id.is_some() {
            return Err("spec_inst_id can only be specified on the management node".to_string());
        }
        if !self.context.mgr_node && !current_avatars.contains(&req_msg.from_avatar) {
            return Err("from_avatar is illegal".to_string());
        }
        // System process
        if req_msg.event == Some(WS_SYSTEM_EVENT_INFO.to_string()) {
            let msg = TardisFuns::json
                .obj_to_json(&TardisWebsocketInstInfo {
                    inst_id: self.context.inst_id.clone(),
                    avatars: current_avatars.clone(),
                    mgr_node: self.context.mgr_node,
                    subscribe_mode: self.context.subscribe_mode,
                })
                .map_err(|error| {
                    crate::log::error!(
                        "[Tardis.WebServer] can't serialize {struct_name}, error: {error}",
                        struct_name = stringify!(TardisWebsocketInstInfo)
                    );
                    "message illegal"
                })?;
            let send_msg = TardisWebsocketMgrMessage {
                msg_id,
                msg,
                from_avatar: req_msg.from_avatar.clone(),
                to_avatars: vec![req_msg.from_avatar],
                event: req_msg.event,
                ignore_self: false,
                ignore_avatars: vec![],
                from_inst_id: if let Some(spec_inst_id) = req_msg.spec_inst_id {
                    spec_inst_id
                } else {
                    self.context.inst_id.clone()
                },
                echo: true,
            };
            self.send_to_channel(send_msg);
            return Ok(());
            // For security reasons, adding an avatar needs to be handled by the management node
        } else if self.context.mgr_node && req_msg.event == Some(WS_SYSTEM_EVENT_AVATAR_ADD.to_string()) {
            let Some(new_avatar) = req_msg.msg.as_str() else {
                return Err("msg is not a string".to_string());
            };
            let Some(ref spec_inst_id) = req_msg.spec_inst_id else {
                return Err("spec_inst_id is not specified".to_string());
            };
            #[cfg(feature = "cluster")]
            {
                let Ok(Some(_)) = insts_in_send.get(spec_inst_id.clone()).await else {
                    return Err("spec_inst_id not found".to_string());
                };
                trace!("[Tardis.WebServer] WS message add avatar {}:{} to {}", &msg_id, &new_avatar, &spec_inst_id);
                let _ = insts_in_send.modify(spec_inst_id.clone(), "add_avatar", json!(new_avatar)).await;
                return Ok(());
            }
            #[cfg(not(feature = "cluster"))]
            {
                let mut write_locked = insts_in_send.write().await;
                let Some(inst) = write_locked.get_mut(spec_inst_id) else {
                    return Err("spec_inst_id not found".to_string());
                };
                inst.push(new_avatar.to_string());
                drop(write_locked);
                trace!("[Tardis.WebServer] WS message add avatar {}:{} to {}", msg_id, new_avatar, spec_inst_id);
            }
        } else if req_msg.event == Some(WS_SYSTEM_EVENT_AVATAR_DEL.to_string()) {
            #[cfg(feature = "cluster")]
            {
                let Ok(Some(_)) = insts_in_send.get(self.context.inst_id.clone()).await else {
                    return Err("spec_inst_id not found".to_string());
                };
                let Some(del_avatar) = req_msg.msg.as_str() else {
                    return Err("msg is not a string".to_string());
                };
                let _ = insts_in_send.modify(self.context.inst_id.clone(), "del_avatar", json!(del_avatar)).await;
                trace!("[Tardis.WebServer] WS message delete avatar {},{} to {}", msg_id, del_avatar, &self.context.inst_id);
            }
            #[cfg(not(feature = "cluster"))]
            {
                let Some(del_avatar) = req_msg.msg.as_str() else {
                    return Err("msg is not a string".to_string());
                };
                let mut write_locked = insts_in_send.write().await;
                let Some(inst) = write_locked.get_mut(&inst_id) else {
                    return Err("spec_inst_id not found".to_string());
                };
                inst.retain(|value| *value != del_avatar);
                drop(write_locked);
                trace!("[Tardis.WebServer] WS message delete avatar {},{} to {}", msg_id, del_avatar, &inst_id);
            }
            return Ok(());
        }
        if let Some(resp_msg) = self.hooks.on_process(req_msg.clone(), &self.context).await {
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
                from_inst_id: if let Some(spec_inst_id) = req_msg.spec_inst_id {
                    spec_inst_id
                } else {
                    self.context.inst_id.clone()
                },
                echo: false,
            };
            self.send_to_channel(send_msg);
        };
        Ok(())
    }
    #[tracing::instrument(skip(websocket, self))]
    pub async fn run(self, avatars: Vec<String>, websocket: WebSocket) -> BoxWebSocketUpgraded {
        websocket
            .on_upgrade(move |socket| async move {
                let mut inner_receiver = self.inner_sender.subscribe();
                // corresponded to the current ws connection
                let inst_id = self.context.inst_id.clone();
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
                let context = self.context.clone();
                debug!("[Tardis.WebServer] WS new connection {inst_id}");
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
                                    if self.context.mgr_node { "[MGR]" } else { "" }
                                );
                                let Some(avatar_self) = current_avatars.first().cloned() else {
                                    warn!("[Tardis.WebServer] current_avatars is empty");
                                    continue;
                                };
                                match TardisFuns::json.str_to_obj::<TardisWebsocketReq>(&text) {
                                    Err(_) => {
                                        self.send_error_to_channel("message illegal", &avatar_self, None);
                                        continue;
                                    }
                                    Ok(req_msg) => {
                                        let msg_id = req_msg.msg_id.clone();
                                        if let Err(e) = self.handle_req(req_msg).await {
                                            self.send_error_to_channel(&e, &avatar_self, msg_id);
                                        }
                                    }
                                }
                            }
                            Message::Close(msg) => {
                                trace!("[Tardis.WebServer] WS message receive: close {:?}", msg);
                                self.hooks.on_close(msg, &self.context).await
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

                        tracing::trace!("[Tardis.WebServer] inner receiver receive message {mgr_message:?}, current_avatars: {current_avatars:?}");
                        if
                        // send to all
                        mgr_message.to_avatars.is_empty() && mgr_message.ignore_avatars.is_empty()
                             // send to targets that match the current avatars
                           || !mgr_message.to_avatars.is_empty() && mgr_message.to_avatars.iter().any(|avatar| current_avatars.contains(avatar))
                        // send to targets that NOT match the current avatars
                        || !mgr_message.ignore_avatars.is_empty() && mgr_message.ignore_avatars.iter().all(|avatar| current_avatars.contains(avatar))
                        {
                            let Ok(resp_msg) = (if context.mgr_node {
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
                            let cache_id = if !context.subscribe_mode {
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
