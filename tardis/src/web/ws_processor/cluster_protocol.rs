use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{borrow::Cow, collections::HashMap, sync::Arc};

use crate::{
    basic::result::TardisResult,
    cluster::{
        cluster_broadcast::ClusterBroadcastChannel,
        cluster_processor::{TardisClusterMessageReq, TardisClusterSubscriber},
    },
};

use super::{ws_insts_mapping_avatars, TardisWebsocketMgrMessage, WsBroadcastSender};

pub const EVENT_AVATAR: &str = "tardis/avatar";

pub(crate) struct Avatar;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum AvatarMessage {
    Sync { table: HashMap<String, Vec<String>> },
}

#[async_trait::async_trait]
impl TardisClusterSubscriber for Avatar {
    fn event_name(&self) -> Cow<'static, str> {
        EVENT_AVATAR.into()
    }

    async fn subscribe(&self, message_req: TardisClusterMessageReq) -> TardisResult<Option<Value>> {
        // let from_node = message_req.req_node_id;
        if let Ok(message) = serde_json::from_value(message_req.msg) {
            match message {
                AvatarMessage::Sync { table } => {
                    let mut routes = ws_insts_mapping_avatars().write().await;
                    for (k, v) in table {
                        routes.insert(k, v);
                    }
                }
            }
        }
        Ok(None)
    }
}

impl WsBroadcastSender for ClusterBroadcastChannel<TardisWebsocketMgrMessage> {
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TardisWebsocketMgrMessage> {
        self.local_broadcast_channel.subscribe()
    }

    fn send(&self, msg: TardisWebsocketMgrMessage) {
        self.send(msg);
    }
}

impl WsBroadcastSender for Arc<ClusterBroadcastChannel<TardisWebsocketMgrMessage>> {
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TardisWebsocketMgrMessage> {
        self.local_broadcast_channel.subscribe()
    }

    fn send(&self, msg: TardisWebsocketMgrMessage) {
        ClusterBroadcastChannel::send(self, msg);
    }
}
