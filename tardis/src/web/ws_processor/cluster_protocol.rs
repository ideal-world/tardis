use std::sync::Arc;

use crate::cluster::cluster_broadcast::ClusterBroadcastChannel;

use super::{TardisWebsocketMgrMessage, WsBroadcastSender};

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
