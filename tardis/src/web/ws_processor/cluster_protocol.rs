use std::sync::Arc;

use crate::{basic::result::TardisResult, cluster::cluster_broadcast::ClusterBroadcastChannel};

use super::{TardisWebsocketMgrMessage, WsBroadcastSender};

impl WsBroadcastSender for ClusterBroadcastChannel<TardisWebsocketMgrMessage> {
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Arc<TardisWebsocketMgrMessage>> {
        self.local_broadcast_channel.subscribe()
    }

    async fn send(&self, msg: TardisWebsocketMgrMessage) -> TardisResult<()> {
        self.send(msg).await
    }
}

impl WsBroadcastSender for Arc<ClusterBroadcastChannel<TardisWebsocketMgrMessage>> {
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Arc<TardisWebsocketMgrMessage>> {
        self.as_ref().subscribe()
    }

    async fn send(&self, msg: TardisWebsocketMgrMessage) -> TardisResult<()> {
        ClusterBroadcastChannel::send(self, msg).await
    }
}
