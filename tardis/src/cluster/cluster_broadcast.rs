use std::sync::{Arc, Weak};

use serde_json::Value;
use tokio::sync::broadcast;

use crate::basic::{error::TardisError, result::TardisResult};

use super::{
    cluster_processor::{peer_count, subscribe_if_not_exist, unsubscribe, ClusterEventTarget, ClusterHandler, TardisClusterMessageReq},
    cluster_publish::publish_event_no_response,
};

pub struct ClusterBroadcastChannel<T>
where
    T: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    pub ident: String,
    pub local_broadcast_channel: broadcast::Sender<Arc<T>>,
}

impl<T> ClusterBroadcastChannel<T>
where
    T: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    pub fn event_name(&self) -> String {
        format!("tardis/broadcast/{}", self.ident)
    }
    pub async fn send(&self, message: T) -> TardisResult<()> {
        match self.local_broadcast_channel.send(message.clone().into()) {
            Ok(size) => {
                tracing::trace!("[Tardis.Cluster] broadcast channel send to {size} local subscribers");
            }
            Err(result) => {
                tracing::error!("[Tardis.Cluster] broadcast channel send error: {:?}", result);
            }
        }
        let event = format!("tardis/broadcast/{}", self.ident);
        let json = serde_json::to_value(message).map_err(|e| TardisError::internal_error(&e.to_string(), ""))?;
        if peer_count().await != 0 {
            let _ = publish_event_no_response(event, json, ClusterEventTarget::Broadcast).await?;
        }
        Ok(())
    }
    pub fn new(ident: impl Into<String>, capacity: usize) -> Arc<Self> {
        let sender = broadcast::Sender::new(capacity);
        let cluster_chan = Arc::new(Self {
            ident: ident.into(),
            local_broadcast_channel: sender,
        });
        tracing::trace!("[Tardis.Cluster] create broadcast channel: {}", cluster_chan.event_name());
        let subscriber = BroadcastChannelSubscriber {
            channel: Arc::downgrade(&cluster_chan),
            event_name: cluster_chan.event_name(),
        };
        tokio::spawn(async {
            subscribe_if_not_exist(subscriber).await;
        });
        cluster_chan
    }
}

impl<T> Drop for ClusterBroadcastChannel<T>
where
    T: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    fn drop(&mut self) {
        let event_name = self.event_name();
        tokio::spawn(async move {
            unsubscribe(&event_name).await;
        });
    }
}

impl<T> std::ops::Deref for ClusterBroadcastChannel<T>
where
    T: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    type Target = broadcast::Sender<Arc<T>>;

    fn deref(&self) -> &Self::Target {
        &self.local_broadcast_channel
    }
}

pub struct BroadcastChannelSubscriber<T>
where
    T: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    event_name: String,
    channel: Weak<ClusterBroadcastChannel<T>>,
}

impl<T> ClusterHandler for BroadcastChannelSubscriber<T>
where
    T: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    fn event_name(&self) -> String {
        self.event_name.to_string()
    }
    async fn handle(self: Arc<Self>, message_req: TardisClusterMessageReq) -> TardisResult<Option<Value>> {
        if let Ok(message) = serde_json::from_value(message_req.msg) {
            if let Some(chan) = self.channel.upgrade() {
                let _ = chan.local_broadcast_channel.send(Arc::new(message));
            } else {
                unsubscribe(&self.event_name()).await;
            }
        }
        Ok(None)
    }
}
