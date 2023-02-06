use futures::{Future, SinkExt, StreamExt};
use log::{info, trace};
use poem::web::{
    websocket::{BoxWebSocketUpgraded, CloseCode, Message, WebSocket},
    Data,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Sender;
use tracing::warn;

use crate::TardisFuns;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TardisWebsocketResp {
    pub msg: String,
    pub from_seesion: String,
    pub to_seesions: Vec<String>,
    pub ignore_self: bool,
}

pub fn ws_echo<PF, PT, CF, CT>(websocket: WebSocket, current_seesion: String, process_fun: PF, close_fun: CF) -> BoxWebSocketUpgraded
where
    PF: Fn(String, String) -> PT + Send + Sync + 'static,
    PT: Future<Output = Option<String>> + Send + 'static,
    CF: Fn(Option<(CloseCode, String)>) -> CT + Send + Sync + 'static,
    CT: Future<Output = ()> + Send + 'static,
{
    websocket
        .on_upgrade(|mut socket| async move {
            while let Some(Ok(message)) = socket.next().await {
                match message {
                    Message::Text(text) => {
                        trace!("[Tardis.WebServer] WS echo receive: {} by {}", text, &current_seesion);
                        if let Some(msg) = process_fun(current_seesion.clone(), text).await {
                            trace!("[Tardis.WebServer] WS echo send: {} to {}", msg, &current_seesion);
                            if let Err(error) = socket.send(Message::Text(msg.clone())).await {
                                warn!("[Tardis.WebServer] WS echo send failed, message {msg} to {}: {error}", &current_seesion);
                                break;
                            }
                        }
                    }
                    Message::Close(msg) => {
                        trace!("[Tardis.WebServer] WS echo receive: clone {:?}", msg);
                        close_fun(msg).await
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

pub fn ws_broadcast<PF, PT, CF, CT>(websocket: WebSocket, sender: Data<&Sender<String>>, current_seesion: String, process_fun: PF, close_fun: CF) -> BoxWebSocketUpgraded
where
    PF: Fn(String, String) -> PT + Send + Sync + 'static,
    PT: Future<Output = Option<TardisWebsocketResp>> + Send + 'static,
    CF: Fn(Option<(CloseCode, String)>) -> CT + Send + Sync + 'static,
    CT: Future<Output = ()> + Send + 'static,
{
    let sender = sender.clone();
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
                            if let Some(resp) = process_fun(current_seesion.clone(), text).await {
                                trace!("[Tardis.WebServer] WS broadcast send: {} to {:?}", resp.msg, resp.to_seesions);
                                if let Err(error) = sender.send(TardisFuns::json.obj_to_string(&resp).unwrap()) {
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
                            close_fun(msg).await
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

            tokio::spawn(async move {
                while let Ok(resp) = receiver.recv().await {
                    let resp = TardisFuns::json.str_to_obj::<TardisWebsocketResp>(&resp).unwrap();
                    if (resp.to_seesions.is_empty() && (!resp.ignore_self || resp.from_seesion != current_seesion_clone)) || resp.to_seesions.contains(&current_seesion_clone) {
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
