use std::sync::Arc;
use std::{collections::HashMap, num::NonZeroUsize};

use futures::{Future, SinkExt, StreamExt};
use log::trace;
use log::warn;
use lru::LruCache;
use poem::web::websocket::{BoxWebSocketUpgraded, CloseCode, Message, WebSocket};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{broadcast::Sender, Mutex};

use crate::TardisFuns;

const WS_CACHE_SIZE: u32 = 1000000;

lazy_static! {
    static ref CACHES: Arc<Mutex<LruCache<String, bool>>> = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(WS_CACHE_SIZE as usize).unwrap())));
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
                        trace!("[Tardis.WebServer] WS echo receive: {} by {}", text, &avatars);
                        if let Some(msg) = process_fun(avatars.clone(), text, ext.clone()).await {
                            trace!("[Tardis.WebServer] WS echo send: {} to {}", msg, &avatars);
                            if let Err(error) = socket.send(Message::Text(msg.clone())).await {
                                warn!("[Tardis.WebServer] WS echo send failed, message {msg} to {}: {error}", &avatars);
                                break;
                            }
                        }
                    }
                    Message::Close(msg) => {
                        trace!("[Tardis.WebServer] WS echo receive: clone {:?}", msg);
                        close_fun(msg, ext.clone()).await
                    }
                    Message::Binary(_) => {
                        warn!("[Tardis.WebServer] WS echo receive: the binary type is not implemented");
                    }
                    Message::Ping(_) => {
                        warn!("[Tardis.WebServer] WS echo receive: the ping type is not implemented");
                    }
                    Message::Pong(_) => {
                        warn!("[Tardis.WebServer] WS echo receive: the pong type is not implemented");
                    }
                }
            }
        })
        .boxed()
}

pub fn ws_broadcast<PF, PT, CF, CT>(
    avatars: Vec<String>,
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
            let current_avatars = avatars.clone();
            let (mut sink, mut stream) = socket.split();

            tokio::spawn(async move {
                while let Some(Ok(message)) = stream.next().await {
                    match message {
                        Message::Text(text) => {
                            trace!("[Tardis.WebServer] WS broadcast receive: {} by {:?}", text, avatars);
                            match TardisFuns::json.str_to_obj::<TardisWebsocketReq>(&text) {
                                Ok(req_msg) => {
                                    if let Some(resp_msg) = process_fun(req_msg.clone(), ext.clone()).await {
                                        trace!(
                                            "[Tardis.WebServer] WS broadcast send to channel: {} to {:?} ignore {:?}",
                                            resp_msg.msg,
                                            resp_msg.to_avatars,
                                            resp_msg.ignore_avatars
                                        );
                                        let send_msg = TardisWebsocketInnerResp {
                                            id: TardisFuns::field.nanoid(),
                                            msg: resp_msg.msg,
                                            from_avatar: req_msg.from_avatar,
                                            to_avatars: resp_msg.to_avatars,
                                            event: req_msg.event,
                                            ignore_self: req_msg.ignore_self.unwrap_or(true),
                                            ignore_avatars: resp_msg.ignore_avatars,
                                        };
                                        if let Err(error) = sender.send(TardisFuns::json.obj_to_string(&send_msg).unwrap()) {
                                            warn!(
                                                "[Tardis.WebServer] WS broadcast send to channel: {} to {:?} ignore {:?} failed: {error}",
                                                send_msg.msg, send_msg.to_avatars, send_msg.ignore_avatars
                                            );
                                            break;
                                        }
                                    }
                                }
                                Err(_) => {
                                    warn!("[Tardis.WebServer] WS broadcast receive: {} by {:?} error: message not illegal", text, avatars);
                                    break;
                                }
                            }
                        }
                        Message::Close(msg) => {
                            trace!("[Tardis.WebServer] WS broadcast receive: close {:?}", msg);
                            close_fun(msg, ext.clone()).await
                        }
                        Message::Binary(_) => {
                            warn!("[Tardis.WebServer] WS broadcast receive: the binary type is not implemented");
                        }
                        Message::Ping(_) => {
                            warn!("[Tardis.WebServer] WS broadcast receive: the ping type is not implemented");
                        }
                        Message::Pong(_) => {
                            warn!("[Tardis.WebServer] WS broadcast receive: the pong type is not implemented");
                        }
                    }
                }
            });

            let cache = CACHES.clone();

            tokio::spawn(async move {
                while let Ok(resp_msg) = receiver.recv().await {
                    let resp = TardisFuns::json.str_to_obj::<TardisWebsocketInnerResp>(&resp_msg).unwrap();
                    if
                    // send to all avatars or except self
                    resp.to_avatars.is_empty() &&  resp.ignore_avatars.is_empty() && (!resp.ignore_self || !current_avatars.contains(&resp.from_avatar))
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
                        if let Err(error) = sink.send(Message::Text(resp.msg.to_string())).await {
                            if error.to_string() != "Connection closed normally" {
                                warn!(
                                    "[Tardis.WebServer] WS broadcast send: {} to {:?} ignore {:?} failed: {error}",
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
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TardisWebsocketResp {
    pub msg: Value,
    pub to_avatars: Vec<String>,
    pub ignore_avatars: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct TardisWebsocketInnerResp {
    pub id: String,
    pub msg: Value,
    pub from_avatar: String,
    pub to_avatars: Vec<String>,
    pub event: Option<String>,
    pub ignore_self: bool,
    pub ignore_avatars: Vec<String>,
}
