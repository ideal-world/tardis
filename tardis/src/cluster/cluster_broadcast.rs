use std::{
    borrow::Cow,
    sync::{Arc, Weak},
};

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::broadcast;

use crate::basic::result::TardisResult;

use super::{
    cluster_processor::{subscribe, unsubscribe, ClusterEventTarget, TardisClusterMessageReq, TardisClusterSubscriber},
    cluster_publish::publish_event_no_response,
};

pub struct ClusterBroadcastChannel<T>
where
    T: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned,
{
    pub ident: String,
    pub local_broadcast_channel: broadcast::Sender<T>,
}

impl<T> ClusterBroadcastChannel<T>
where
    T: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned,
{
    pub fn event_name(&self) -> String {
        format!("tardis/broadcast/{}", self.ident)
    }
    pub fn send(&self, message: T) {
        let _ = self.local_broadcast_channel.send(message.clone());
        let event = format!("tardis/broadcast/{}", self.ident);
        tokio::spawn(async move {
            if let Ok(json_value) = serde_json::to_value(message) {
                let json = json_value;
                let _ = publish_event_no_response(event, json, ClusterEventTarget::Broadcast).await;
            }
        });
    }
    pub fn new(ident: impl Into<String>, capacity: usize) -> Arc<Self> {
        let sender = broadcast::Sender::new(capacity);
        let cluster_chan = Arc::new(Self {
            ident: ident.into(),
            local_broadcast_channel: sender,
        });

        let subscriber = BroadcastChannelSubscriber {
            channel: Arc::downgrade(&cluster_chan),
            event_name: cluster_chan.event_name(),
        };
        tokio::spawn(subscribe(subscriber));
        cluster_chan
    }
}

impl<T> Drop for ClusterBroadcastChannel<T>
where
    T: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned,
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
    T: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned,
{
    type Target = broadcast::Sender<T>;

    fn deref(&self) -> &Self::Target {
        &self.local_broadcast_channel
    }
}

pub struct BroadcastChannelSubscriber<T>
where
    T: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned,
{
    event_name: String,
    channel: Weak<ClusterBroadcastChannel<T>>,
}

#[async_trait]
impl<T> TardisClusterSubscriber for BroadcastChannelSubscriber<T>
where
    T: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned,
{
    fn event_name(&self) -> Cow<'static, str> {
        self.event_name.to_string().into()
    }
    async fn subscribe(&self, message_req: TardisClusterMessageReq) -> TardisResult<Option<Value>> {
        if let Ok(message) = serde_json::from_value(message_req.msg) {
            if let Some(chan) = self.channel.upgrade() {
                let _ = chan.send(message);
            } else {
                unsubscribe(&self.event_name()).await;
            }
        }
        Ok(None)
    }
}
