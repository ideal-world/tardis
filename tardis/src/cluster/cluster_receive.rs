use std::collections::HashMap;

use tokio::sync::{mpsc, RwLock};

use crate::tardis_static;

use self::listen::Listener;

use super::cluster_processor::{TardisClusterMessageResp, CLUSTER_MESSAGE_CACHE_SIZE};

enum ResponseFn {
    Once(Box<dyn FnOnce(TardisClusterMessageResp) + Send + Sync>),
    Multitime(Box<dyn Fn(TardisClusterMessageResp) -> bool + Send + Sync>),
}
tardis_static! {
    responser_subscribers: RwLock<HashMap::<String, ResponseFn>>;
}

pub(crate) async fn listen_reply<S: Listener>(strategy: S, id: String) -> S::Reply {
    strategy.subscribe(id).await
}

pub(crate) fn init_response_dispatcher() -> mpsc::Sender<TardisClusterMessageResp> {
    let (tx, mut rx) = mpsc::channel::<TardisClusterMessageResp>(CLUSTER_MESSAGE_CACHE_SIZE);
    // rx is for ws connections
    // tx is for response dispatcher
    let dispatch_task = async move {
        while let Some(resp) = rx.recv().await {
            let id = resp.msg_id.clone();
            tracing::trace!(
                "[Tardis.Cluster] dispatching received response: {id} from {node_id}, message: {resp:?}",
                id = id,
                node_id = resp.resp_node_id,
                resp = resp
            );
            if let Some(subscriber) = responser_subscribers().read().await.get(&id) {
                match subscriber {
                    ResponseFn::Once(_) => {
                        tokio::spawn(async move {
                            if let Some(ResponseFn::Once(f)) = responser_subscribers().write().await.remove(&id) {
                                f(resp)
                            }
                        });
                    }
                    ResponseFn::Multitime(f) => {
                        let drop_me = f(resp);
                        if drop_me {
                            tokio::spawn(async move {
                                responser_subscribers().write().await.remove(&id);
                            });
                        }
                    }
                }
            } else {
                tracing::trace!("[Tardis.Cluster] no subscriber found for message_id: {id}.", id = id);
            }
        }
    };
    tokio::spawn(dispatch_task);
    tx
}

pub mod listen {
    use std::time::Duration;

    use async_trait::async_trait;
    use tokio::sync::{broadcast, mpsc, oneshot};

    use crate::cluster::cluster_processor::TardisClusterMessageResp;

    use super::ResponseFn;
    #[async_trait]
    pub trait Listener {
        type Reply;
        async fn subscribe(self, id: String) -> Self::Reply;
    }

    /// The message will be received only once.
    #[derive(Debug, Default, Clone, Copy)]
    pub struct Once {
        pub(crate) timeout: Option<Duration>,
    }

    impl Once {
        pub fn with_timeout(timeout: Duration) -> Self {
            Self { timeout: Some(timeout) }
        }
    }

    #[async_trait]
    impl Listener for Once {
        type Reply = oneshot::Receiver<TardisClusterMessageResp>;

        async fn subscribe(self, id: String) -> Self::Reply {
            let (tx, rx) = oneshot::channel();
            let timeout_handle = {
                let id = id.clone();
                self.timeout.map(|timeout| {
                    tokio::spawn(async move {
                        tokio::time::sleep(timeout).await;

                        // super::responser_subscribers().write().await.remove(&id);
                        // tracing::trace!("[Tardis.Cluster] message {id} timeout");

                        if let Some(_task) = super::responser_subscribers().write().await.remove(&id) {
                            tracing::trace!("[Tardis.Cluster] message {id} timeout");
                        }
                    })
                })
            };
            super::responser_subscribers().write().await.insert(
                id,
                ResponseFn::Once(Box::new(move |resp| {
                    tracing::trace!("[Tardis.Cluster] Once listener receive resp {resp:?}");
                    // cleanup timeout callback
                    if let Some(ref timeout_handle) = timeout_handle {
                        timeout_handle.abort();
                    }
                    if let Err(e) = tx.send(resp) {
                        tracing::debug!("[Tardis.Cluster] message {e:?} missing receiver");
                    }
                })),
            );
            rx
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    /// send a message and receive all the responses until the receiver is dropped.
    pub struct Stream;

    #[async_trait]
    impl Listener for Stream {
        type Reply = mpsc::Receiver<TardisClusterMessageResp>;

        async fn subscribe(self, id: String) -> Self::Reply {
            let (tx, rx) = mpsc::channel(100);
            {
                let tx = tx.clone();
                let id = id.clone();
                tokio::spawn(async move {
                    tx.closed().await;
                    super::responser_subscribers().write().await.remove(&id);
                });
            }
            super::responser_subscribers().write().await.insert(
                id,
                ResponseFn::Multitime(Box::new(move |resp| {
                    if tx.is_closed() {
                        true
                    } else {
                        let tx = tx.clone();
                        tokio::spawn(async move { tx.send(resp).await });
                        false
                    }
                })),
            );
            rx
        }
    }

    /// Send a message and ignore the response.
    #[derive(Debug, Default, Clone, Copy)]
    pub struct Never;

    #[async_trait]
    impl Listener for Never {
        type Reply = String;

        async fn subscribe(self, id: String) -> Self::Reply {
            id
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct Broadcast {}

    #[async_trait]
    impl Listener for Broadcast {
        type Reply = broadcast::Receiver<TardisClusterMessageResp>;

        async fn subscribe(self, id: String) -> Self::Reply {
            let (tx, rx) = broadcast::channel(100);
            {
                let tx = tx.clone();
                let id = id.clone();
                tokio::spawn(async move {
                    if tx.receiver_count() == 0 {
                        super::responser_subscribers().write().await.remove(&id);
                    } else {
                        tokio::task::yield_now().await;
                    }
                });
            }
            super::responser_subscribers().write().await.insert(
                id,
                ResponseFn::Multitime(Box::new(move |resp| {
                    let _ = tx.send(resp);
                    tx.receiver_count() == 0
                })),
            );
            rx
        }
    }
}
