use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use super::cluster_processor::{TardisClusterMessageReq, TardisClusterMessageResp};
use super::cluster_receive::{listen::*, listen_reply};
use crate::cluster::cluster_processor::{cache_nodes, local_node_id, ClusterEventTarget, ClusterRemoteNodeKey};
use crate::{
    basic::{error::TardisError, result::TardisResult},
    web::ws_client::TardisWSClient,
    TardisFuns,
};
use futures::future::join_all;
use serde::Serialize;
use serde_json::Value;
use tracing::{error, trace};

/// Cluster-wide event
///
/// `<L>` is the listener type, default is [`Once`], which implies that the response message will be received only once.
///
/// # Example
/// ```
/// # use tardis::cluster::cluster_publish::ClusterEvent;
/// let event = ClusterEvent::new("hello").no_response().message(&("hello", "world"));
/// ```
#[derive(Debug, Clone)]
pub struct ClusterEvent<L = Once> {
    event: Cow<'static, str>,
    message: Value,
    target: ClusterEventTarget,
    listener: L,
}

impl ClusterEvent {
    pub fn new(event: impl Into<Cow<'static, str>>) -> Self {
        Self {
            event: event.into(),
            message: Value::Null,
            target: ClusterEventTarget::Broadcast,
            listener: Once { timeout: None },
        }
    }
}

impl<L> ClusterEvent<L> {
    pub fn listener<L2: Listener>(self, listener: L2) -> ClusterEvent<L2> {
        ClusterEvent {
            event: self.event,
            message: self.message,
            target: self.target,
            listener,
        }
    }
    /// Set the listener to receive only one response.
    pub fn one_response(self, timeout: Option<Duration>) -> ClusterEvent<Once> {
        ClusterEvent {
            event: self.event,
            message: self.message,
            target: self.target,
            listener: Once { timeout },
        }
    }
    /// Don't expect any response.
    pub fn no_response(self) -> ClusterEvent<Never> {
        ClusterEvent {
            event: self.event,
            message: self.message,
            target: self.target,
            listener: Never,
        }
    }
    /// Set the message of the event.
    pub fn message<T: Serialize>(self, message: &T) -> TardisResult<Self> {
        Ok(Self {
            message: crate::TardisFuns::json.obj_to_json(message)?,
            ..self
        })
    }
    pub fn json_message(self, message: Value) -> Self {
        Self { message, ..self }
    }
    /// Set the target of the event.
    ///
    /// see [`ClusterEventTarget`]
    pub fn target(self, target: impl Into<ClusterEventTarget>) -> Self {
        Self { target: target.into(), ..self }
    }
}

impl ClusterEvent<Once> {
    /// Publish the event and receive only one response.
    pub async fn publish_one_response(self) -> TardisResult<TardisClusterMessageResp> {
        publish_event_with_listener(self.event, self.message, self.target, self.listener).await?.await.map_err(|e| {
            let error_info = format!("[Tardis.Cluster] [Client] Oneshot receive error: {e}, this may caused by timeout");
            tracing::error!("{error_info}");
            TardisError::wrap(&error_info, "-1-tardis-cluster-receive-message-error")
        })
    }
}

impl<L: Listener> ClusterEvent<L> {
    /// Publish the event.
    pub async fn publish(self) -> TardisResult<L::Reply> {
        publish_event_with_listener(self.event, self.message, self.target, self.listener).await
    }
}

/// Publish an event with no response.
pub async fn publish_event_no_response(event: impl Into<Cow<'static, str>>, message: Value, target: impl Into<ClusterEventTarget>) -> TardisResult<String> {
    publish_event_with_listener(event, message, target, Never).await
}

/// Publish an event and receive only one response.
pub async fn publish_event_one_response(
    event: impl Into<Cow<'static, str>>,
    message: Value,
    target: impl Into<ClusterEventTarget>,
    timeout: Option<Duration>,
) -> TardisResult<TardisClusterMessageResp> {
    publish_event_with_listener(event, message, target, Once { timeout }).await?.await.map_err(|e| {
        let error_info = format!("[Tardis.Cluster] [Client] Oneshot receive error: {e}, this may caused by timeout");
        tracing::error!("{error_info}");
        TardisError::wrap(&error_info, "-1-tardis-cluster-receive-message-error")
    })
}

/// Publish an event
pub async fn publish_event_with_listener<S: Listener>(
    event: impl Into<Cow<'static, str>>,
    message: Value,
    target: impl Into<ClusterEventTarget>,
    listener: S,
) -> TardisResult<S::Reply> {
    let node_id = local_node_id().await.to_string();
    let event = event.into();
    let target = target.into();
    let target_debug = format!("{target:?}");
    trace!("[Tardis.Cluster] [Client] publish event {event} , message {message} , to {target_debug}");

    let nodes: Vec<_> = match target {
        ClusterEventTarget::Broadcast => cache_nodes()
            .read()
            .await
            .iter()
            .filter(|(key, _)| match key {
                // just filter out my self
                ClusterRemoteNodeKey::NodeId(peer_node_id) => peer_node_id != &node_id,
                _ => false,
            })
            .map(|(_, val)| val.client.clone())
            .collect(),
        ClusterEventTarget::Single(ref addr) => cache_nodes().read().await.get(addr).map(|node| node.client.clone()).into_iter().collect(),
        ClusterEventTarget::Multi(ref multi) => {
            let cache_nodes = cache_nodes().read().await;
            multi.iter().filter_map(|addr| cache_nodes.get(addr).map(|node| node.client.clone())).collect()
        }
        ClusterEventTarget::Client(client) => vec![client],
    };
    if nodes.is_empty() {
        return Err(TardisError::wrap(
            &format!(
                "[Tardis.Cluster] [Client] publish event {event} , message {message} , to {target} error: can't find any target node",
                event = event,
                message = message,
                target = target_debug
            ),
            "-1-tardis-cluster-publish-message-error",
        ));
    }
    let message_req = TardisClusterMessageReq::new(message.clone(), event.to_string(), node_id);
    let message_id = message_req.msg_id.clone();
    let reply = listen_reply(listener, message_id).await;
    do_publish_event(message_req, nodes).await?;
    Ok(reply)
}

pub(crate) async fn do_publish_event(message_req: TardisClusterMessageReq, clients: impl IntoIterator<Item = Arc<TardisWSClient>>) -> TardisResult<()> {
    let ws_message = tokio_tungstenite::tungstenite::Message::Text(TardisFuns::json.obj_to_string(&message_req)?);
    let publish_result = join_all(clients.into_iter().map(|client| {
        let ws_message = ws_message.clone();
        async move { client.send_raw_with_retry(ws_message).await }
    }))
    .await;
    if publish_result
        .iter()
        .filter(|result| {
            if let Err(error) = result {
                error!(
                    "[Tardis.Cluster] [Client] publish event {event} , message {message}: {error}",
                    event = message_req.event,
                    message = message_req.msg
                );
                true
            } else {
                false
            }
        })
        .count()
        != 0
    {
        Err(TardisError::wrap(
            &format!(
                "[Tardis.Cluster] [Client] publish event {event} , message {message} error",
                event = message_req.event,
                message = message_req.msg
            ),
            "-1-tardis-cluster-publish-message-error",
        ))
    } else {
        Ok(())
    }
}
