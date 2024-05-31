use std::{
    borrow::Cow,
    sync::{Arc, Weak},
};

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::broadcast;

use crate::basic::{error::TardisError, result::TardisResult};

use super::{
    cluster_processor::{subscribe_if_not_exist, unsubscribe, ClusterEventTarget, TardisClusterMessageReq, TardisClusterSubscriber},
    cluster_publish::publish_event_no_response,
};

pub struct ClusterBroadcastChannel<T>
where
    T: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    pub ident: String,
    pub local_broadcast_channel: broadcast::Sender<T>,
}

impl<T> ClusterBroadcastChannel<T>
where
    T: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    pub fn event_name(&self) -> String {
        format!("tardis/broadcast/{}", self.ident)
    }
    pub async fn send(&self, message: T) -> TardisResult<()> {
        if let Err(result) = self.local_broadcast_channel.send(message.clone()) {
            tracing::error!("[Tardis.Cluster] broadcast channel send error: {:?}", result);
        }
        let event = format!("tardis/broadcast/{}", self.ident);
        let json = serde_json::to_value(message).map_err(|e|TardisError::internal_error(&e.to_string(), ""))?;
        let _ = publish_event_no_response(event, json, ClusterEventTarget::Broadcast).await?;
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
    type Target = broadcast::Sender<T>;

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

#[async_trait]
impl<T> TardisClusterSubscriber for BroadcastChannelSubscriber<T>
where
    T: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    fn event_name(&self) -> Cow<'static, str> {
        self.event_name.to_string().into()
    }
    async fn subscribe(&self, message_req: TardisClusterMessageReq) -> TardisResult<Option<Value>> {
        if let Ok(message) = serde_json::from_value(message_req.msg) {
            if let Some(chan) = self.channel.upgrade() {
                let _ = chan.local_broadcast_channel.send(message);
            } else {
                unsubscribe(&self.event_name()).await;
            }
        }
        Ok(None)
    }
}
