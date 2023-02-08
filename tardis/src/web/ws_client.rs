use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::log::info;
use crate::TardisFuns;
use futures::{Future, SinkExt, StreamExt};
use log::{trace, warn};
use serde::Serialize;
use tokio::sync::broadcast::{self, Sender};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

pub struct TardisWSClient {
    tx: Sender<std::string::String>,
}

impl TardisWSClient {
    pub async fn init<F, T>(str_url: &str, fun: F) -> TardisResult<TardisWSClient>
    where
        F: Fn(String) -> T + Send + Sync + 'static,
        T: Future<Output = Option<String>> + Send + 'static,
    {
        let (tx, mut rx) = broadcast::channel(1024);
        let tx_clone = tx.clone();
        let url = Url::parse(str_url).map_err(|_| TardisError::format_error(&format!("[Tardis.WSClient] Invalid url {str_url}"), "406-tardis-ws-url-error"))?;
        info!("[Tardis.WSClient] Initializing, host:{}, port:{}", url.host_str().unwrap_or(""), url.port().unwrap_or(0));
        let (client, _) = connect_async(url.clone()).await.unwrap_or_else(|_| panic!("[Tardis.WSClient] Failed to connect {str_url}"));
        info!("[Tardis.WSClient] Initialized, host:{}, port:{}", url.host_str().unwrap_or(""), url.port().unwrap_or(0));
        let (mut write, mut read) = client.split();

        tokio::spawn(async move {
            while let Some(Ok(text)) = read.next().await {
                match text {
                    Message::Text(text) => {
                        trace!("[Tardis.WSClient] WS receive: {}", text);
                        if let Some(resp) = fun(text).await {
                            trace!("[Tardis.WSClient] WS send: {}", resp);
                            if let Err(error) = tx.send(resp.clone()) {
                                warn!("[Tardis.WSClient] Failed to send message {resp}: {error}");
                                break;
                            }
                        }
                    }
                    Message::Binary(_) => {
                        trace!("[Tardis.WSClient] WS receive: the binary type is not implemented");
                    }
                    Message::Ping(_) => {
                        trace!("[Tardis.WSClient] WS receive: the ping type is not implemented");
                    }
                    Message::Pong(_) => {
                        trace!("[Tardis.WSClient] WS receive: the pong type is not implemented");
                    }
                    Message::Close(_) => {
                        trace!("[Tardis.WSClient] WS receive: the close type is not implemented");
                    }
                    Message::Frame(_) => {
                        trace!("[Tardis.WSClient] WS receive: the frame type is not implemented");
                    }
                }
            }
        });
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(resp) => {
                        if let Err(error) = write.send(Message::Text(resp.clone())).await {
                            warn!("[Tardis.WSClient] Failed to send message {resp}: {error}");
                            break;
                        }
                    }
                    Err(error) => {
                        warn!("[Tardis.WSClient] Failed to send message: {error}");
                        break;
                    }
                }
            }
        });
        Ok(TardisWSClient { tx: tx_clone })
    }

    pub async fn send_raw(&self, msg: String) -> TardisResult<()> {
        self.tx
            .send(msg.clone())
            .map(|_| {})
            .map_err(|error| TardisError::format_error(&format!("[Tardis.WSClient] Failed to send message {msg}: {error}"), "500-tardis-ws-client-send-error"))
    }

    pub async fn send_obj<T: ?Sized + Serialize>(&self, msg: &T) -> TardisResult<()> {
        let msg = TardisFuns::json.obj_to_string(msg).unwrap();
        self.send_raw(msg).await
    }
}
