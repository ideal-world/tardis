use std::sync::Arc;
use std::{collections::HashMap, num::NonZeroUsize};

use futures::{Future, SinkExt, StreamExt};
use log::trace;
use log::warn;
use lru::LruCache;
use poem::web::websocket::{BoxWebSocketUpgraded, CloseCode, Message, WebSocket};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tokio::sync::{broadcast::Sender, Mutex};

use crate::TardisFuns;

pub const WS_SYSTEM_EVENT_INFO: &str = "__sys_info__";
pub const WS_SYSTEM_EVENT_AVATAR_ADD: &str = "__sys_avatar_add__";
pub const WS_SYSTEM_EVENT_AVATAR_DEL: &str = "__sys_avatar_del__";
pub const WS_SYSTEM_EVENT_ERROR: &str = "__sys_error__";
pub const WS_CACHE_SIZE: u32 = 1000000;

lazy_static! {
    static ref CACHES: Arc<Mutex<LruCache<String, bool>>> = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(WS_CACHE_SIZE as usize).unwrap())));
    // Instance Id -> Avatars
    static ref INSTS: Arc<RwLock<HashMap<String, Vec<String>>>> = Arc::new(RwLock::new(HashMap::new()));
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
                        trace!("[Tardis.WebServer] WS message receive: clone {:?}", msg);
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

fn ws_send_to_channel(send_msg: TardisWebsocketMgrMessage, sender: &Sender<String>) -> bool {
    if let Err(error) = sender.send(TardisFuns::json.obj_to_string(&send_msg).unwrap()) {
        warn!(
            "[Tardis.WebServer] WS message send to channel: {} to {:?} ignore {:?} failed: {error}",
            send_msg.msg, send_msg.to_avatars, send_msg.ignore_avatars
        );
        return false;
    }
    true
}

pub fn ws_send_error_to_channel(req_message: &str, error_message: &str, from_avatar: &str, from_inst_id: &str, sender: &Sender<String>) -> bool {
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
    ws_send_to_channel(send_msg, sender)
}

pub fn ws_broadcast<PF, PT, CF, CT>(
    avatars: Vec<String>,
    mgr_node: bool,
    subscribe_mode: bool,
    ext: HashMap<String, String>,
    websocket: WebSocket,
    sender: Sender<String>,
    process_fun: PF,
    close_fun: CF,
) -> BoxWebSocketUpgraded
where
    PF: Fn(TardisWebsocketReq, HashMap<String, String>) -> PT + Send + Sync + 'static,
    PT: Future<Output = Option<TardisWebsocketResp>> + Send + 'static,
    CF: Fn(Option<(CloseCode, String)>, HashMap<String, String>) -> CT + Send + Sync + 'static,
    CT: Future<Output = ()> + Send + 'static,
{
    let mut receiver = sender.subscribe();
    websocket
        .on_upgrade(move |socket| async move {
            let inst_id = TardisFuns::field.nanoid();
            let current_receive_inst_id = inst_id.clone();
            INSTS.write().await.insert(inst_id.clone(), avatars);
            let (mut sink, mut stream) = socket.split();

            let insts_in_send = INSTS.clone();
            tokio::spawn(async move {
                while let Some(Ok(message)) = stream.next().await {
                    match message {
                        Message::Text(text) => {
                            let msg_id = TardisFuns::field.nanoid();
                            let current_avatars = insts_in_send.read().await.get(&inst_id).unwrap().clone();
                            trace!(
                                "[Tardis.WebServer] WS message receive: {}:{} by {:?} {}",
                                msg_id,
                                text,
                                current_avatars,
                                if mgr_node { "[MGR]" } else { "" }
                            );
                            match TardisFuns::json.str_to_obj::<TardisWebsocketReq>(&text) {
                                Err(_) => {
                                    ws_send_error_to_channel(&text, "message not illegal", current_avatars.get(0).unwrap(), &inst_id, &sender);
                                    break;
                                }
                                Ok(req_msg) => {
                                    // Security check
                                    if !mgr_node && req_msg.spec_inst_id.is_some() {
                                        ws_send_error_to_channel(
                                            &text,
                                            "spec_inst_id can only be specified on the management node",
                                            current_avatars.get(0).unwrap(),
                                            &inst_id,
                                            &sender,
                                        );
                                        break;
                                    }
                                    if !mgr_node && !current_avatars.contains(&req_msg.from_avatar) {
                                        ws_send_error_to_channel(&text, "from_avatar is not illegal", current_avatars.get(0).unwrap(), &inst_id, &sender);
                                        break;
                                    }
                                    // System process
                                    if req_msg.event == Some(WS_SYSTEM_EVENT_INFO.to_string()) {
                                        let send_msg = TardisWebsocketMgrMessage {
                                            id: TardisFuns::field.nanoid(),
                                            msg: TardisFuns::json
                                                .obj_to_json(&TardisWebsocketInstInfo {
                                                    inst_id: inst_id.clone(),
                                                    avatars: current_avatars,
                                                    mgr_node,
                                                    subscribe_mode,
                                                })
                                                .unwrap(),
                                            from_avatar: req_msg.from_avatar.clone(),
                                            to_avatars: vec![req_msg.from_avatar],
                                            event: req_msg.event,
                                            ignore_self: false,
                                            ignore_avatars: vec![],
                                            from_inst_id: if let Some(spec_inst_id) = req_msg.spec_inst_id { spec_inst_id } else { inst_id.clone() },
                                            echo: true,
                                        };
                                        if !ws_send_to_channel(send_msg, &sender) {
                                            break;
                                        }
                                        continue;
                                        // For security reasons, adding an avatar needs to be handled by the management node
                                    } else if mgr_node && req_msg.event == Some(WS_SYSTEM_EVENT_AVATAR_ADD.to_string()) {
                                        let new_avatar = req_msg.msg.as_str().unwrap();
                                        insts_in_send.write().await.get_mut(&req_msg.spec_inst_id.clone().unwrap()).unwrap().push(new_avatar.to_string());
                                        trace!("[Tardis.WebServer] WS message add avatar {}:{} to {}", msg_id, new_avatar, req_msg.spec_inst_id.unwrap());
                                        continue;
                                    } else if req_msg.event == Some(WS_SYSTEM_EVENT_AVATAR_DEL.to_string()) {
                                        let del_avatar = req_msg.msg.as_str().unwrap();
                                        insts_in_send.write().await.get_mut(&inst_id).unwrap().retain(|value| *value != del_avatar);
                                        trace!("[Tardis.WebServer] WS message delete avatar {},{} to {}", msg_id, del_avatar, &inst_id);
                                        continue;
                                    }

                                    // Normal process
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
                                        if !ws_send_to_channel(send_msg, &sender) {
                                            break;
                                        }
                                    }
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

            let cache = CACHES.clone();
            let insts_in_receive = INSTS.clone();

            tokio::spawn(async move {
                while let Ok(resp_msg) = receiver.recv().await {
                    let resp = TardisFuns::json.str_to_obj::<TardisWebsocketMgrMessage>(&resp_msg).unwrap();
                    let current_avatars = insts_in_receive.read().await.get(&current_receive_inst_id).unwrap().clone();
                    // // only self
                    if resp.echo && current_receive_inst_id != resp.from_inst_id {
                        continue;
                    }
                    // except self
                    if resp.ignore_self && current_receive_inst_id == resp.from_inst_id {
                        continue;
                    }
                    if
                    // send to all
                    resp.to_avatars.is_empty() && resp.ignore_avatars.is_empty()
                             // send to targets that match the current avatars
                           || !resp.to_avatars.is_empty() && resp.to_avatars.iter().any(|avatar| current_avatars.contains(avatar))
                        // send to targets that NOT match the current avatars
                        || !resp.ignore_avatars.is_empty() && resp.ignore_avatars.iter().all(|avatar| current_avatars.contains(avatar))
                    {
                        if !subscribe_mode {
                            let id = format!("{}{:?}", resp.id, &current_avatars);
                            let mut lock = cache.lock().await;
                            if lock.put(id.clone(), true).is_some() {
                                continue;
                            }
                        }
                        let resp_msg = if mgr_node {
                            TardisFuns::json.obj_to_string(&resp).unwrap()
                        } else {
                            TardisFuns::json
                                .obj_to_string(&TardisWebsocketMessage {
                                    msg: resp.msg.clone(),
                                    event: resp.event.clone(),
                                })
                                .unwrap()
                        };
                        if let Err(error) = sink.send(Message::Text(resp_msg)).await {
                            if error.to_string() != "Connection closed normally" {
                                warn!(
                                    "[Tardis.WebServer] WS message send: {} to {:?} ignore {:?} failed: {error}",
                                    resp.msg, resp.to_avatars, resp.ignore_avatars
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

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TardisWebsocketMessage {
    pub msg: Value,
    pub event: Option<String>,
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

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct TardisWebsocketInstInfo {
    pub inst_id: String,
    pub avatars: Vec<String>,
    pub mgr_node: bool,
    pub subscribe_mode: bool,
}
