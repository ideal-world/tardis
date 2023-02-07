use std::{collections::HashMap, sync::Arc, time::Duration};

use futures::{Future, SinkExt, StreamExt};
use log::trace;
use moka::future::Cache;
use poem::web::websocket::{BoxWebSocketUpgraded, CloseCode, Message, WebSocket};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{broadcast::Sender, RwLock};
use tracing::warn;

use crate::TardisFuns;

lazy_static! {
    static ref CACHES: Arc<RwLock<HashMap<String, Cache<String, bool>>>> = Arc::new(RwLock::new(HashMap::new()));
}

const WS_CACHE_TTL_SEC: u64 = 30;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TardisWebsocketResp {
    pub msg: String,
    pub from_seesion: String,
    pub to_seesions: Vec<String>,
    pub ignore_self: bool,
}

pub fn ws_echo<PF, PT, CF, CT>(websocket: WebSocket, current_seesion: String, ext: HashMap<String, String>, process_fun: PF, close_fun: CF) -> BoxWebSocketUpgraded
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
                        trace!("[Tardis.WebServer] WS echo receive: {} by {}", text, &current_seesion);
                        if let Some(msg) = process_fun(current_seesion.clone(), text, ext.clone()).await {
                            trace!("[Tardis.WebServer] WS echo send: {} to {}", msg, &current_seesion);
                            if let Err(error) = socket.send(Message::Text(msg.clone())).await {
                                warn!("[Tardis.WebServer] WS echo send failed, message {msg} to {}: {error}", &current_seesion);
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
    topic: String,
    websocket: WebSocket,
    sender: Sender<String>,
    current_seesion: String,
    subscribe_mode: bool,
    ext: HashMap<String, String>,
    process_fun: PF,
    close_fun: CF,
) -> BoxWebSocketUpgraded
where
    PF: Fn(String, String, HashMap<String, String>) -> PT + Send + Sync + 'static,
    PT: Future<Output = Option<TardisWebsocketResp>> + Send + 'static,
    CF: Fn(Option<(CloseCode, String)>, HashMap<String, String>) -> CT + Send + Sync + 'static,
    CT: Future<Output = ()> + Send + 'static,
{
    let mut receiver = sender.subscribe();
    websocket
        .on_upgrade(move |socket| async move {
            let current_seesion_clone = current_seesion.clone();
            let (mut sink, mut stream) = socket.split();

            tokio::spawn(async move {
                while let Some(Ok(message)) = stream.next().await {
                    match message {
                        Message::Text(text) => {
                            trace!("[Tardis.WebServer] WS broadcast receive: {} by {}", text, current_seesion);
                            if let Some(resp) = process_fun(current_seesion.clone(), text, ext.clone()).await {
                                trace!("[Tardis.WebServer] WS broadcast send: {} to {:?}", resp.msg, resp.to_seesions);
                                let send_msg = if !subscribe_mode {
                                    let mut value = TardisFuns::json.obj_to_json(&resp).unwrap();
                                    value.as_object_mut().unwrap().insert("id".to_string(), Value::String(TardisFuns::field.nanoid()));
                                    value.to_string()
                                } else {
                                    TardisFuns::json.obj_to_string(&resp).unwrap()
                                };
                                if let Err(error) = sender.send(send_msg) {
                                    warn!(
                                        "[Tardis.WebServer] WS broadcast send to channel failed, message {} to {:?}: {error}",
                                        resp.msg, resp.to_seesions
                                    );
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

            if !subscribe_mode && !CACHES.read().await.contains_key(&topic) {
                CACHES.write().await.insert(topic.clone(), Cache::builder().time_to_live(Duration::from_secs(WS_CACHE_TTL_SEC)).build());
            }

            tokio::spawn(async move {
                while let Ok(resp_str) = receiver.recv().await {
                    let resp = TardisFuns::json.str_to_obj::<TardisWebsocketResp>(&resp_str).unwrap();
                    if (resp.to_seesions.is_empty() && (!resp.ignore_self || resp.from_seesion != current_seesion_clone)) || resp.to_seesions.contains(&current_seesion_clone) {
                        if !subscribe_mode {
                            let resp = TardisFuns::json.str_to_json(&resp_str).unwrap();
                            let id = resp.get("id").unwrap().as_str().unwrap();
                            let id = format!("{id}{}", &current_seesion_clone);
                            let cache = CACHES.read().await;
                            let cache = cache.get(&topic.clone()).unwrap();
                            if cache.contains_key(&id) {
                                continue;
                            }
                            cache.insert(id.clone(), true).await;
                        }
                        if let Err(error) = sink.send(Message::Text(resp.msg.clone())).await {
                            if error.to_string() != "Connection closed normally" {
                                warn!("[Tardis.WebServer] WS broadcast send failed, message {} to {:?}: {error}", resp.msg, resp.to_seesions);
                            }
                            break;
                        }
                    }
                }
            });
        })
        .boxed()
}
