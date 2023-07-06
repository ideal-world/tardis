use std::sync::Arc;

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::TardisFuns;
#[cfg(feature = "future")]
use futures::stream::SplitSink;
#[cfg(feature = "future")]
use futures::{Future, SinkExt, StreamExt};
use native_tls::TlsConnector;
use serde::Serialize;
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::tungstenite::{self, Error, Message};
use tokio_tungstenite::{Connector, MaybeTlsStream, WebSocketStream};
use tracing::info;
use tracing::{trace, warn};
use url::Url;

pub struct TardisWSClient<F, T>
where
    F: Fn(Message) -> T + Send + Sync + 'static,
    T: Future<Output = Option<Message>> + Send + 'static,
{
    str_url: String,
    fun: F,
    write: Mutex<Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>>,
}

impl<F, T> TardisWSClient<F, T>
where
    F: Fn(Message) -> T + Send + Sync + Copy + 'static,
    T: Future<Output = Option<Message>> + Send + 'static,
{
    pub async fn init(str_url: &str, fun: F) -> TardisResult<TardisWSClient<F, T>> {
        Self::do_init(str_url, fun, false).await
    }

    async fn do_init(str_url: &str, fun: F, retry: bool) -> TardisResult<TardisWSClient<F, T>> {
        let url = Url::parse(str_url).map_err(|_| TardisError::format_error(&format!("[Tardis.WSClient] Invalid url {str_url}"), "406-tardis-ws-url-error"))?;
        info!("[Tardis.WSClient] Initializing, host:{}, port:{}", url.host_str().unwrap_or(""), url.port().unwrap_or(0));
        let connect = if !str_url.starts_with("wss") {
            tokio_tungstenite::connect_async(url.clone()).await
        } else {
            tokio_tungstenite::connect_async_tls_with_config(
                url.clone(),
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
        let (client, _) = connect.map_err(|error| {
            if !retry {
                TardisError::format_error(&format!("[Tardis.WSClient] Failed to connect {str_url} {error}"), "500-tardis-ws-client-connect-error")
            } else {
                TardisError::format_error(&format!("[Tardis.WSClient] Failed to reconnect {str_url} {error}"), "500-tardis-ws-client-reconnect-error")
            }
        })?;
        info!("[Tardis.WSClient] Initialized, host:{}, port:{}", url.host_str().unwrap_or(""), url.port().unwrap_or(0));
        let (write, mut read) = client.split();
        let write = Arc::new(Mutex::new(write));
        let reply = write.clone();
        tokio::spawn(async move {
            while let Some(Ok(message)) = read.next().await {
                trace!("[Tardis.WSClient] WS receive: {}", message);
                if let Some(resp) = fun(message).await {
                    trace!("[Tardis.WSClient] WS send: {}", resp);
                    if let Err(error) = reply.lock().await.send(resp).await {
                        warn!("[Tardis.WSClient] Failed to send message : {error}");
                        break;
                    }
                }
            }
        });
        Ok(TardisWSClient {
            str_url: str_url.to_string(),
            fun,
            write: Mutex::new(write),
        })
    }

    pub async fn send_obj<E: ?Sized + Serialize>(&self, msg: &E) -> TardisResult<()> {
        let message = TardisFuns::json.obj_to_string(msg)?;
        self.send_text(message).await
    }

    pub async fn send_text(&self, message: String) -> TardisResult<()> {
        let message = Message::Text(message.clone());
        self.send_with_retry(message).await
    }

    pub async fn send_with_retry(&self, message: Message) -> TardisResult<()> {
        if let Err(error) = self.do_send(message.clone()).await {
            warn!("[Tardis.WSClient] Failed to send message {}: {}", message.clone(), error);
            match error {
                Error::AlreadyClosed | Error::Io(_) => {
                    if let Err(error) = self.reconnect().await {
                        Err(error)
                    } else {
                        self.do_send(message.clone())
                            .await
                            .map_err(|error| TardisError::format_error(&format!("[Tardis.WSClient] Failed to send message {message}: {error}"), "500-tardis-ws-client-send-error"))
                    }
                }
                _ => Err(TardisError::format_error(
                    &format!("[Tardis.WSClient] Failed to send message {message}: {error}"),
                    "500-tardis-ws-client-send-error",
                )),
            }
        } else {
            Ok(())
        }
    }

    pub async fn do_send(&self, message: Message) -> Result<(), tungstenite::Error> {
        self.write.lock().await.lock().await.send(message).await
    }

    async fn reconnect(&self) -> TardisResult<()> {
        let new_client = Self::do_init(&self.str_url, self.fun, true).await?;
        *self.write.lock().await = new_client.write.lock().await.clone();
        Ok(())
    }
}
