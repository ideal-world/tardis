use std::pin::Pin;
use std::sync::Arc;

#[cfg(feature = "future")]
use futures::{Future, SinkExt, StreamExt};
use native_tls::TlsConnector;
use serde::de::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::{mpsc, OwnedSemaphorePermit, RwLock, Semaphore};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::Connector;
use tracing::{debug, info};
use tracing::{trace, warn};
use url::Url;

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::TardisFuns;

type OnMsgCbk = Arc<dyn Fn(Message) -> Pin<Box<dyn Future<Output = Option<Message>> + Send + Sync>> + Send + Sync>;
// with a callback function to handle inbound messages, but never handle inbound messages positively.
// and then, we should also send messages through this client.

#[derive(Clone)]
pub struct TardisWSClient {
    pub(crate) url: Url,
    on_message: OnMsgCbk,
    sender: Arc<RwLock<mpsc::UnboundedSender<Message>>>,
    connection_semaphore: Arc<Semaphore>,
}

impl std::fmt::Debug for TardisWSClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TardisWSClient").field("url", &self.url.to_string()).field("connected", &self.is_connected()).finish()
    }
}

impl TardisWSClient {
    pub async fn connect<F, T>(str_url: &str, on_message: F) -> TardisResult<TardisWSClient>
    where
        F: Fn(Message) -> T + Send + Sync + Clone + 'static,
        T: Future<Output = Option<Message>> + Send + Sync + 'static,
    {
        let url = Url::parse(str_url).map_err(|_| TardisError::format_error(&format!("[Tardis.WSClient] Invalid url {str_url}"), "406-tardis-ws-url-error"))?;

        let connection_semaphore = Arc::new(Semaphore::const_new(1));
        let permit = connection_semaphore.clone().acquire_owned().await.expect("newly created semaphore should not fail");
        let tx = {
            let on_message = on_message.clone();
            Self::do_connect(&url, Arc::new(move |m| Box::pin(on_message(m))), false, permit).await?
        };
        let sender = Arc::new(RwLock::new(tx));
        Ok(TardisWSClient {
            url,
            on_message: Arc::new(move |m| Box::pin(on_message(m))),
            sender,
            connection_semaphore,
        })
    }

    pub fn is_connected(&self) -> bool {
        self.connection_semaphore.available_permits() == 0
    }

    async fn do_connect(url: &Url, on_message: OnMsgCbk, retry: bool, permit: OwnedSemaphorePermit) -> TardisResult<mpsc::UnboundedSender<Message>> {
        info!(
            "[Tardis.WSClient] {}, host:{}, port:{}",
            if retry { "Re-initializing" } else { "Initializing" },
            url.host_str().unwrap_or(""),
            url.port().unwrap_or(0)
        );
        let connect = if url.scheme() != "wss" {
            tokio_tungstenite::connect_async(url.to_string()).await
        } else {
            tokio_tungstenite::connect_async_tls_with_config(
                url.to_string(),
                None,
                false,
                Some(Connector::NativeTls(TlsConnector::builder().danger_accept_invalid_certs(true).build().map_err(|e| {
                    TardisError::format_error(
                        &format!("[Tardis.WSClient] Failed to build tls connector: {e}"),
                        "500-tardis-ws-client-build-connector-error",
                    )
                })?)),
            )
            .await
        };
        let (stream, _) = connect.map_err(|error| {
            if !retry {
                TardisError::format_error(&format!("[Tardis.WSClient] Failed to connect {url} {error}"), "500-tardis-ws-client-connect-error")
            } else {
                TardisError::format_error(&format!("[Tardis.WSClient] Failed to reconnect {url} {error}"), "500-tardis-ws-client-reconnect-error")
            }
        })?;
        info!(
            "[Tardis.WSClient] {}, host:{}, port:{}",
            if retry { "Re-initialized" } else { "Initialized" },
            url.host_str().unwrap_or(""),
            url.port().unwrap_or(0)
        );
        let (mut ws_tx, mut ws_rx) = stream.split();
        // let ws_tx = Arc::new(Mutex::new(ws_tx));
        // let reply = ws_tx.clone();
        // let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<Message>();

        let (outbound_queue_tx, mut outbound_queue_rx) = mpsc::unbounded_channel::<Message>();

        // there should be two queue:
        // 1. out to client queue
        // 2. client to remote queue

        // outbound side
        let ob_handle = {
            let url = url.clone();
            tokio::spawn(async move {
                while let Some(message) = outbound_queue_rx.recv().await {
                    if let Err(e) = ws_tx.send(message).await {
                        warn!("[Tardis.WSClient] client: {url} error when send to websocket: {e}");
                        match e {
                            tokio_tungstenite::tungstenite::Error::ConnectionClosed | tokio_tungstenite::tungstenite::Error::AlreadyClosed => break,
                            _ => {}
                        }
                        // websocket was closed
                    }
                }
            })
        };

        // inbound side
        let ib_handle = {
            let on_message = on_message.clone();

            let outbound_queue_tx = outbound_queue_tx.clone();
            let url = url.clone();
            tokio::spawn(async move {
                // stream would be owned by one single task and
                // 1. outbound messages would be sent by the task, and can be forwarded from other tasks
                // 2. inbound messages would be received by the task, and be dropped in this task.
                while let Some(message) = ws_rx.next().await {
                    match message {
                        Ok(message) => {
                            if message.is_text() {
                                trace!("[Tardis.WSClient] WS receive text: {}", message);
                            }
                            let fut_response = on_message(message);
                            let outbound_queue_tx = outbound_queue_tx.clone();
                            let url = url.clone();
                            tokio::spawn(async move {
                                if let Some(resp) = fut_response.await {
                                    trace!("[Tardis.WSClient] WS send: {}", resp);
                                    if let Err(e) = outbound_queue_tx.send(resp) {
                                        debug!("[Tardis.WSClient] client: {url} error when send to outbound message queue: {e}")
                                        // outbound channel was closed
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            warn!("[Tardis.WSClient] client: {url} error when receive from websocket: {e}")
                        }
                    }
                }
            })
        };
        tokio::spawn(async move {
            let permit = permit;
            tokio::select! {
                _ = ib_handle => {},
                _ = ob_handle => {}
            }
            drop(permit)
        });

        Ok(outbound_queue_tx)
    }

    pub async fn send_obj<E: ?Sized + Serialize>(&self, msg: &E) -> TardisResult<()> {
        let message = TardisFuns::json.obj_to_string(msg)?;
        self.send_text(message).await
    }

    pub async fn send_text(&self, message: String) -> TardisResult<()> {
        let message = Message::Text(message.clone());
        self.send_raw_with_retry(message).await
    }

    pub async fn send_raw_with_retry(&self, message: Message) -> TardisResult<()> {
        // wait until the client is ready
        const MAX_RETRY_TIME: usize = 1;
        let mut retry_time = 0;
        while retry_time < MAX_RETRY_TIME {
            let connected = self.send_raw(message.clone()).await?;
            if !connected {
                self.reconnect().await?;
                retry_time += 1;
                continue;
            } else {
                return Ok(());
            }
        }
        Err(TardisError::format_error(
            &format!("[Tardis.WSClient] Failed to send message {message}: exceed max retry time {MAX_RETRY_TIME}"),
            "500-tardis-ws-client-send-error",
        ))
    }

    /// Send a message to the websocket server.
    /// if the client is not ready or disconnected, a `Ok(false)` value would be returned.
    pub async fn send_raw(&self, message: Message) -> TardisResult<bool> {
        if !self.is_connected() {
            return Ok(false);
        }
        match self.sender.read().await.send(message) {
            Ok(_) => Ok(true),
            Err(_) => Err(TardisError::format_error(
                &format!("[Tardis.WSClient] Client {url} failed to send message", url = self.url),
                "500-tardis-ws-client-send-error",
            )),
        }
    }

    pub async fn reconnect(&self) -> TardisResult<()> {
        if let Ok(permit) = self.connection_semaphore.clone().try_acquire_owned() {
            info!("[Tardis.WSClient] trying to reconnect {url}", url = self.url);
            let sender = Self::do_connect(&self.url, self.on_message.clone(), true, permit).await?;
            *self.sender.write().await = sender;
        }
        Ok(())
    }
}

pub trait TardisWebSocketMessageExt {
    fn str_to_obj<T: for<'de> Deserialize<'de>>(&self) -> TardisResult<T>;
    fn str_to_json(&self) -> TardisResult<Value>;
}

impl TardisWebSocketMessageExt for Message {
    fn str_to_obj<T: for<'de> Deserialize<'de>>(&self) -> TardisResult<T> {
        if let Message::Text(msg) = self {
            TardisFuns::json.str_to_obj(msg).map_err(|_| {
                TardisError::format_error(
                    &format!("[Tardis.WSClient] Message {self} parse to object error"),
                    "400-tardis-ws-client-message-parse-error",
                )
            })
        } else {
            Err(TardisError::format_error(
                &format!("[Tardis.WSClient] Message {self} isn't a text type"),
                "400-tardis-ws-client-message-not-text",
            ))
        }
    }

    fn str_to_json(&self) -> TardisResult<Value> {
        if let Message::Text(msg) = self {
            TardisFuns::json
                .str_to_json(msg)
                .map_err(|_| TardisError::format_error(&format!("[Tardis.WSClient] Message {self} parse to json error"), "400-tardis-ws-client-message-parse-error"))
        } else {
            Err(TardisError::format_error(
                &format!("[Tardis.WSClient] Message {self} isn't a text type"),
                "400-tardis-ws-client-message-not-text",
            ))
        }
    }
}
