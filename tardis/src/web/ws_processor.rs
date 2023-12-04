#[cfg(feature = "cluster")]
pub mod cluster_protocol;

use std::sync::Arc;
use std::{collections::HashMap, num::NonZeroUsize};

use futures::{Future, SinkExt, StreamExt};
use lru::LruCache;
use poem::web::websocket::{BoxWebSocketUpgraded, CloseCode, Message, WebSocket};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tracing::trace;
use tracing::warn;

use crate::cluster::cluster_hashmap::ClusterStaticHashMap;
use crate::{tardis_static, TardisFuns};

pub const WS_SYSTEM_EVENT_INFO: &str = "__sys_info__";
pub const WS_SYSTEM_EVENT_AVATAR_ADD: &str = "__sys_avatar_add__";
pub const WS_SYSTEM_EVENT_AVATAR_DEL: &str = "__sys_avatar_del__";
pub const WS_SYSTEM_EVENT_ERROR: &str = "__sys_error__";

/// # Safety:
/// It's safe for we set the cache size manually
#[allow(clippy::undocumented_unsafe_blocks)]
pub const WS_SENDER_CACHE_SIZE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(1000000) };

tardis_static! {
    // Websocket instance Id -> Avatars
    // ws_insts_mapping_avatars: Arc<RwLock<HashMap<String, Vec<String>>>>;
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
    static ref REPLY_ONCE_GUARD: Arc<Mutex<LruCache<String, bool>>> = Arc::new(Mutex::new(LruCache::new(WS_SENDER_CACHE_SIZE)));
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

fn ws_send_to_channel<TX>(send_msg: TardisWebsocketMgrMessage, inner_sender: &TX)
where
    TX: WsBroadcastSender,
{
    inner_sender.send(send_msg.clone());
}

pub fn ws_send_error_to_channel<TX>(req_message: &str, error_message: &str, from_avatar: &str, from_inst_id: &str, inner_sender: &TX)
where
    TX: WsBroadcastSender,
{
    let send_msg = TardisWebsocketMgrMessage {
        id: TardisFuns::field.nanoid(),
        msg: json!(error_message),
        from_avatar: from_avatar.to_string(),
        to_avatars: vec![from_avatar.to_string()],
        event: Some(WS_SYSTEM_EVENT_ERROR.to_string()),
        ignore_self: false,
        ignore_avatars: vec![],
        from_inst_id: from_inst_id.to_string(),
        echo: true,
    };
    warn!("[Tardis.WebServer] WS message receive: {} by {:?} failed: {error_message}", req_message, from_avatar);
    ws_send_to_channel(send_msg, inner_sender)
}

pub trait WsBroadcastSender: Send + Sync + 'static {
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TardisWebsocketMgrMessage>;
    // irresponsable to return error
    fn send(&self, msg: TardisWebsocketMgrMessage);
}

impl WsBroadcastSender for tokio::sync::broadcast::Sender<TardisWebsocketMgrMessage> {
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TardisWebsocketMgrMessage> {
        self.subscribe()
    }

    fn send(&self, msg: TardisWebsocketMgrMessage) {
        if let Err(err) = self.send(msg) {
            let msg = err.0;
            warn!(
                "[Tardis.WebServer] WS message send to channel: {} to {:?} ignore {:?} failed",
                msg.msg, msg.to_avatars, msg.ignore_avatars
            );
        }
    }
}

pub async fn ws_broadcast<PF, PT, CF, CT>(
    avatars: Vec<String>,
    mgr_node: bool,
    subscribe_mode: bool,
    ext: HashMap<String, String>,
    websocket: WebSocket,
    inner_sender: impl WsBroadcastSender,
    process_fun: PF,
    close_fun: CF,
) -> BoxWebSocketUpgraded
where
    PF: Fn(TardisWebsocketReq, HashMap<String, String>) -> PT + Send + Sync + 'static,
    PT: Future<Output = Option<TardisWebsocketResp>> + Send + 'static,
    CF: Fn(Option<(CloseCode, String)>, HashMap<String, String>) -> CT + Send + Sync + 'static,
    CT: Future<Output = ()> + Send + 'static,
{
    let mut inner_receiver = inner_sender.subscribe();
    websocket
        .on_upgrade(move |socket| async move {
            // corresponded to the current ws connection
            let inst_id = TardisFuns::field.nanoid();
            let current_receive_inst_id = inst_id.clone();
            let _ = ws_insts_mapping_avatars().insert(inst_id.clone(), avatars).await;
            let (mut ws_sink, mut ws_stream) = socket.split();

            let insts_in_send = ws_insts_mapping_avatars().clone();
            tokio::spawn(async move {
                // message inbound
                while let Some(Ok(message)) = ws_stream.next().await {
                    match message {
                        Message::Text(text) => {
                            let msg_id = TardisFuns::field.nanoid();
                            let Ok(Some(current_avatars)) = insts_in_send.get(inst_id.clone()).await else {
                                warn!("[Tardis.WebServer] insts_in_send of inst_id {inst_id} not found");
                                continue;
                            };
                            trace!(
                                "[Tardis.WebServer] WS message receive: {}:{} by {:?} {}",
                                msg_id,
                                text,
                                current_avatars,
                                if mgr_node { "[MGR]" } else { "" }
                            );
                            let Some(avatar_self) = current_avatars.get(0).cloned() else {
                                warn!("[Tardis.WebServer] current_avatars is empty");
                                continue;
                            };
                            match TardisFuns::json.str_to_obj::<TardisWebsocketReq>(&text) {
                                Err(_) => {
                                    ws_send_error_to_channel(&text, "message not illegal", &avatar_self, &inst_id, &inner_sender);
                                    break;
                                }
                                Ok(req_msg) => {
                                    // Security check
                                    if !mgr_node && req_msg.spec_inst_id.is_some() {
                                        ws_send_error_to_channel(&text, "spec_inst_id can only be specified on the management node", &avatar_self, &inst_id, &inner_sender);
                                        break;
                                    }
                                    if !mgr_node && !current_avatars.contains(&req_msg.from_avatar) {
                                        ws_send_error_to_channel(&text, "from_avatar is not illegal", &avatar_self, &inst_id, &inner_sender);
                                        break;
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
                                                ws_send_error_to_channel(&text, "message not illegal", &avatar_self, &inst_id, &inner_sender);
                                            })
                                        else {
                                            break;
                                        };
                                        let send_msg = TardisWebsocketMgrMessage {
                                            id: TardisFuns::field.nanoid(),
                                            msg,
                                            from_avatar: req_msg.from_avatar.clone(),
                                            to_avatars: vec![req_msg.from_avatar],
                                            event: req_msg.event,
                                            ignore_self: false,
                                            ignore_avatars: vec![],
                                            from_inst_id: if let Some(spec_inst_id) = req_msg.spec_inst_id { spec_inst_id } else { inst_id.clone() },
                                            echo: true,
                                        };
                                        ws_send_to_channel(send_msg, &inner_sender);
                                        continue;
                                        // For security reasons, adding an avatar needs to be handled by the management node
                                    } else if mgr_node && req_msg.event == Some(WS_SYSTEM_EVENT_AVATAR_ADD.to_string()) {
                                        let Some(new_avatar) = req_msg.msg.as_str() else {
                                            ws_send_error_to_channel(&text, "msg is not a string", &avatar_self, &inst_id, &inner_sender);
                                            continue;
                                        };
                                        let Some(spec_inst_id) = req_msg.spec_inst_id else {
                                            ws_send_error_to_channel(&text, "spec_inst_id is not specified", &avatar_self, &inst_id, &inner_sender);
                                            continue;
                                        };
                                        trace!("[Tardis.WebServer] WS message add avatar {}:{} to {}", msg_id, &new_avatar, &spec_inst_id);
                                        let _ = insts_in_send.modify(spec_inst_id, "add_avatar", json!(new_avatar)).await;
                                        continue;
                                    } else if req_msg.event == Some(WS_SYSTEM_EVENT_AVATAR_DEL.to_string()) {
                                        let Some(del_avatar) = req_msg.msg.as_str() else {
                                            ws_send_error_to_channel(&text, "msg is not a string", &avatar_self, &inst_id, &inner_sender);
                                            continue;
                                        };
                                        let _ = insts_in_send.modify(inst_id.clone(), "del_avatar", json!(del_avatar)).await;
                                        trace!("[Tardis.WebServer] WS message delete avatar {},{} to {}", msg_id, del_avatar, &inst_id);
                                        continue;
                                    }
                                    if let Some(resp_msg) = process_fun(req_msg.clone(), ext.clone()).await {
                                        trace!(
                                            "[Tardis.WebServer] WS message send to channel: {},{} to {:?} ignore {:?}",
                                            msg_id,
                                            resp_msg.msg,
                                            resp_msg.to_avatars,
                                            resp_msg.ignore_avatars
                                        );
                                        let send_msg = TardisWebsocketMgrMessage {
                                            id: msg_id.clone(),
                                            msg: resp_msg.msg,
                                            from_avatar: req_msg.from_avatar,
                                            to_avatars: resp_msg.to_avatars,
                                            event: req_msg.event,
                                            ignore_self: req_msg.ignore_self.unwrap_or(true),
                                            ignore_avatars: resp_msg.ignore_avatars,
                                            from_inst_id: if let Some(spec_inst_id) = req_msg.spec_inst_id { spec_inst_id } else { inst_id.clone() },
                                            echo: false,
                                        };
                                        ws_send_to_channel(send_msg, &inner_sender);
                                    };
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
            });

            let reply_once_guard = REPLY_ONCE_GUARD.clone();

            tokio::spawn(async move {
                while let Ok(mgr_message) = inner_receiver.recv().await {
                    let Ok(Some(current_avatars)) = ({ ws_insts_mapping_avatars().get(current_receive_inst_id.clone()).await }) else {
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
                    if
                    // send to all
                    mgr_message.to_avatars.is_empty() && mgr_message.ignore_avatars.is_empty()
                             // send to targets that match the current avatars
                           || !mgr_message.to_avatars.is_empty() && mgr_message.to_avatars.iter().any(|avatar| current_avatars.contains(avatar))
                        // send to targets that NOT match the current avatars
                        || !mgr_message.ignore_avatars.is_empty() && mgr_message.ignore_avatars.iter().all(|avatar| current_avatars.contains(avatar))
                    {
                        if !subscribe_mode {
                            let id = format!("{}{:?}", mgr_message.id, &current_avatars);
                            let mut lock = reply_once_guard.lock().await;
                            if lock.put(id.clone(), true).is_some() {
                                continue;
                            }
                        }
                        let Ok(resp_msg) = (if mgr_node {
                            TardisFuns::json.obj_to_string(&mgr_message)
                        } else {
                            TardisFuns::json.obj_to_string(&TardisWebsocketMessage {
                                msg: mgr_message.msg.clone(),
                                event: mgr_message.event.clone(),
                            })
                        }) else {
                            warn!("[Tardis.WebServer] Cannot serialize {:?} into json", mgr_message);
                            continue;
                        };
                        if let Err(error) = ws_sink.send(Message::Text(resp_msg)).await {
                            if error.to_string() != "Connection closed normally" {
                                warn!(
                                    "[Tardis.WebServer] WS message send: {} to {:?} ignore {:?} failed: {error}",
                                    mgr_message.msg, mgr_message.to_avatars, mgr_message.ignore_avatars
                                );
                            }
                            break;
                        }
                    }
                }
            });
        })
        .boxed()
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct TardisWebsocketReq {
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
    pub id: String,
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
    pub fn into_req(self, msg: Value, current_avatar: String, to_avatars: Option<Vec<String>>) -> TardisWebsocketReq {
        TardisWebsocketReq {
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
