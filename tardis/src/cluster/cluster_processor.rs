use std::sync::Arc;

use futures_util::future::join_all;
use futures_util::{Future, SinkExt, StreamExt};
use poem::web::websocket::{BoxWebSocketUpgraded, Message, WebSocket};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, trace, warn};

use crate::basic::error::TardisError;
use crate::cluster::{cluster_watch_by_cache, cluster_watch_by_k8s};
use crate::config::config_dto::FrameworkConfig;
use crate::web::ws_client::TardisWSClient;
use crate::{basic::result::TardisResult, web::web_server::TardisWebServer, TardisFuns};

pub const CLUSTER_NODE_WHOAMI: &str = "__cluster_node_who_am_i__";
pub const CLUSTER_MESSAGE_CACHE_SIZE: usize = 10000;

lazy_static! {
    static ref SERVER_MESSAGE_SENDERS: Arc<RwLock<Option<broadcast::Sender<TardisClusterMessageReq>>>> = Arc::new(RwLock::new(None));
    static ref SERVER_MESSAGE_RESPONDER: Arc<RwLock<Option<broadcast::Sender<TardisClusterMessageResp>>>> = Arc::new(RwLock::new(None));
    static ref CLIENT_MESSAGE_RESPONDER: Arc<RwLock<Option<broadcast::Sender<TardisClusterMessageResp>>>> = Arc::new(RwLock::new(None));
    static ref CLUSTER_CACHE_NODES: Arc<RwLock<Vec<TardisClusterNode>>> = Arc::new(RwLock::new(Vec::new()));
    static ref CLUSTER_CURRENT_NODE_ID: Arc<RwLock<String>> = Arc::new(RwLock::new(String::new()));
}

pub async fn init_by_conf(conf: &FrameworkConfig, cluster_server: &TardisWebServer) -> TardisResult<()> {
    if let Some(cluster_config) = &conf.cluster {
        info!("[Tardis.Cluster] Initializing cluster");
        init_node(cluster_server).await?;
        match cluster_config.watch_kind.to_lowercase().as_str() {
            #[cfg(feature = "k8s")]
            "k8s" => {
                info!("[Tardis.Cluster] Initializing cluster by k8s");
                cluster_watch_by_k8s::init(cluster_config,&conf.web_server).await?;
            }
            "cache" => {
                info!("[Tardis.Cluster] Initializing cluster by default");
                cluster_watch_by_cache::init(cluster_config, &conf.web_server).await?;
            }
            _ => panic!("[Tardis.Cluster] Unsupported cluster watch kind: {}", cluster_config.watch_kind),
        }
        info!("[Tardis.Cluster] Initialized cluster");
    }
    Ok(())
}

async fn init_node(cluster_server: &TardisWebServer) -> TardisResult<()> {
    info!("[Tardis.Cluster] Initializing node");
    *CLUSTER_CURRENT_NODE_ID.write().await = TardisFuns::field.nanoid();
    if !SERVER_MESSAGE_SENDERS.read().await.is_some() {
        *SERVER_MESSAGE_SENDERS.write().await = Some(broadcast::channel::<TardisClusterMessageReq>(CLUSTER_MESSAGE_CACHE_SIZE).0);
    }
    if !SERVER_MESSAGE_RESPONDER.read().await.is_some() {
        *SERVER_MESSAGE_RESPONDER.write().await = Some(broadcast::channel::<TardisClusterMessageResp>(CLUSTER_MESSAGE_CACHE_SIZE).0);
    }
    if !CLIENT_MESSAGE_RESPONDER.read().await.is_some() {
        *CLIENT_MESSAGE_RESPONDER.write().await = Some(broadcast::channel::<TardisClusterMessageResp>(CLUSTER_MESSAGE_CACHE_SIZE).0);
    }
    debug!("[Tardis.Cluster] Register exchange route");
    cluster_server.add_route(ClusterAPI).await;
    debug!("[Tardis.Cluster] Register default events");
    subscribe_event(CLUSTER_NODE_WHOAMI, |_| async { Ok(Some(Value::String(CLUSTER_CURRENT_NODE_ID.read().await.to_string()))) });

    info!("[Tardis.Cluster] Initialized node");
    Ok(())
}

pub async fn refresh_nodes(active_nodes: Vec<(String, u16)>) -> TardisResult<()> {
    trace!("[Tardis.Cluster] Refreshing nodes");
    trace!("[Tardis.Cluster] Find all active nodes: {:?}", active_nodes);
    let mut cache_nodes = CLUSTER_CACHE_NODES.write().await;
    trace!("[Tardis.Cluster] Remove inactive nodes from cache");
    cache_nodes.retain(|cache_node| active_nodes.iter().any(|(active_node_ip, active_node_port)| cache_node.ip == *active_node_ip && cache_node.port == *active_node_port));
    trace!("[Tardis.Cluster] Add new active nodes to cache");
    let added_active_nodes = active_nodes
        .iter()
        .filter(|(active_node_ip, active_node_port)| !cache_nodes.iter().any(|cache_node| cache_node.ip == *active_node_ip && cache_node.port == *active_node_port))
        .collect::<Vec<_>>();
    for (active_node_ip, active_node_port) in added_active_nodes {
        cache_nodes.push(add_node(active_node_ip, *active_node_port).await?);
    }
    trace!("[Tardis.Cluster] Refreshed nodes");
    Ok(())
}

async fn add_node(node_ip: &str, node_port: u16) -> TardisResult<TardisClusterNode> {
    debug!("[Tardis.Cluster] Connect node: {node_ip}:{node_port}");
    let client = TardisFuns::ws_client(&format!("ws://{node_ip}:{node_port}/tardis/cluster/ws/exchange"), move |message| async move {
        if let tokio_tungstenite::tungstenite::Message::Text(message) = message {
            match TardisFuns::json.str_to_obj::<TardisClusterMessageResp>(&message) {
                Ok(message_resp) => {
                    if let Err(error) = CLIENT_MESSAGE_RESPONDER.read().await.as_ref().expect("Global variable [CLIENT_MESSAGE_RESPONDER] doesn't exist").send(message_resp) {
                        error!("[Tardis.Cluster] [Client] response message {message}: {error}");
                    }
                }
                Err(error) => error!("[Tardis.Cluster] [Client] response message {message}: {error}"),
            }
        }
        None
    })
    .await?;
    debug!("[Tardis.Cluster] Determine whether it is the current node");
    let whoami_msg_id = do_publish_event(CLUSTER_NODE_WHOAMI, Value::Null, vec![&client]).await?;
    let whoami_result = publish_event_wait_resp(&whoami_msg_id).await?;
    let node_id: &str = whoami_result.as_str().ok_or_else(|| {
        TardisError::format_error(
            &format!("[Tardis.Cluster] {CLUSTER_NODE_WHOAMI} event response message format error"),
            "406-tardis-cluster-response-message-format-error",
        )
    })?;
    let is_current_node = node_id == CLUSTER_CURRENT_NODE_ID.read().await.as_str();
    Ok(TardisClusterNode {
        id: node_id.to_string(),
        ip: node_ip.to_string(),
        port: node_port,
        current: is_current_node,
        client: if is_current_node { Some(client) } else { None },
    })
}

pub fn subscribe_event<F, T>(event: &str, sub_fun: F)
where
    F: Fn(TardisClusterMessageReq) -> T + Send + Sync + Copy + 'static,
    T: Future<Output = TardisResult<Option<Value>>> + Send + 'static,
{
    info!("[Tardis.Cluster] [Server] subscribe event {event}");
    let event = event.to_string();
    tokio::spawn(async move {
        while let Ok(message_req) = SERVER_MESSAGE_SENDERS.read().await.as_ref().expect("Global variable [SERVER_MESSAGE_SENDERS] doesn't exist").clone().subscribe().recv().await {
            if message_req.event == event {
                match sub_fun(message_req.clone()).await {
                    Ok(Some(message_reply)) => {
                        if let Err(error) = SERVER_MESSAGE_RESPONDER
                            .read()
                            .await
                            .as_ref()
                            .expect("Global variable [SERVER_MESSAGE_RESPONDER] doesn't exist")
                            .send(TardisClusterMessageResp::new(message_reply, message_req.id()))
                        {
                            error!("[Tardis.Cluster] [Server] reply message {message_req:?}: {error}");
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        warn!("[Tardis.Cluster] [Server] subscribe function by message {message_req:?}: {error}");
                    }
                }
            }
        }
    });
}

pub async fn publish_event(event: &str, message: Value, node_ids: Option<Vec<&str>>) -> TardisResult<String> {
    trace!("[Tardis.Cluster] [Client] publish event {event} , message {message} , to {node_ids:?}");
    let cache_nodes = CLUSTER_CACHE_NODES.read().await;
    let node_clients = cache_nodes
        .iter()
        .filter(|node| node.client.is_some() && node_ids.as_ref().map(|node_ids| node_ids.contains(&node.id.as_str())).unwrap_or(true))
        .map(|node| node.client.as_ref().expect("ignore"))
        .collect::<Vec<_>>();
    do_publish_event(event, message, node_clients).await
}

async fn do_publish_event(event: &str, message: Value, node_clients: Vec<&TardisWSClient>) -> TardisResult<String> {
    let message_req = TardisClusterMessageReq::new(message.clone(), event.to_string());
    let ws_message = tokio_tungstenite::tungstenite::Message::Text(TardisFuns::json.obj_to_string(&message_req)?);
    let publish_result = join_all(node_clients.iter().map(|client| client.send_raw_with_retry(ws_message.clone()))).await;

    if publish_result
        .iter()
        .filter(|result| {
            if let Err(error) = result {
                error!("[Tardis.Cluster] [Client] publish event {event} , message {message}: {error}");
                true
            } else {
                false
            }
        })
        .count()
        != 0
    {
        Err(TardisError::wrap(
            &format!("[Tardis.Cluster] [Client] publish event {event} , message {message} error"),
            "-1-tardis-cluster-publish-message-error",
        ))
    } else {
        Ok(message_req.id())
    }
}

async fn publish_event_wait_resp(msg_id: &str) -> TardisResult<Value> {
    loop {
        match CLIENT_MESSAGE_RESPONDER.read().await.as_ref().expect("Global variable [CLIENT_MESSAGE_RESPONDER] doesn't exist").subscribe().recv().await {
            Ok(message_response) => {
                if message_response.id == msg_id {
                    return Ok(message_response.msg);
                }
            }
            Err(error) => {
                error!("[Tardis.Cluster] [Client] receive message id {msg_id}: {error}");
                return Err(TardisError::wrap(
                    &format!("[Tardis.Cluster] [Client] receive message id {msg_id}: {error}"),
                    "-1-tardis-cluster-receive-message-error",
                ));
            }
        }
    }
}

pub async fn publish_event_and_wait_resp(event: &str, message: Value, node_id: &str) -> TardisResult<Value> {
    trace!("[Tardis.Cluster] [Client] publish and wait resp, event {event} , message {message} , to {node_id}");
    let msg_id = publish_event(event, message, Some(vec![node_id])).await?;
    publish_event_wait_resp(&msg_id).await
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TardisClusterMessageReq {
    id: String,
    pub msg: Value,
    pub event: String,
}

impl TardisClusterMessageReq {
    pub fn new(msg: Value, event: String) -> Self {
        Self {
            id: TardisFuns::field.nanoid(),
            msg,
            event,
        }
    }

    pub fn id(&self) -> String {
        self.id.to_string()
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TardisClusterMessageResp {
    id: String,
    pub msg: Value,
}

impl TardisClusterMessageResp {
    pub fn new(msg: Value, req_id: String) -> Self {
        Self { id: req_id, msg }
    }

    pub fn id(&self) -> String {
        self.id.to_string()
    }
}

pub struct TardisClusterNode {
    pub id: String,
    pub ip: String,
    pub port: u16,
    pub current: bool,
    pub client: Option<TardisWSClient>,
}

#[derive(Debug, Clone)]
struct ClusterAPI;

#[poem_openapi::OpenApi]
impl ClusterAPI {
    #[oai(path = "/tardis/cluster/ws/exchange", method = "get")]
    async fn exchange(&self, websocket: WebSocket) -> BoxWebSocketUpgraded {
        websocket
            .on_upgrade(|socket| async move {
                let (mut socket_write, mut socket_read) = socket.split();
                tokio::spawn(async move {
                    let message_sender = SERVER_MESSAGE_SENDERS.read().await.as_ref().expect("Global variable [SERVER_MESSAGE_SENDERS] doesn't exist").clone();
                    while let Some(Ok(ws_message)) = socket_read.next().await {
                        match ws_message {
                            Message::Text(ws_message) => {
                                trace!("[Tardis.Cluster] [Server] receive message {ws_message}");
                                match TardisFuns::json.str_to_obj::<TardisClusterMessageReq>(&ws_message) {
                                    Ok(message_req) => {
                                        if let Err(error) = message_sender.send(message_req) {
                                            error!("[Tardis.Cluster] [Server] send message {ws_message}: {error}");
                                        }
                                    }
                                    Err(error) => error!("[Tardis.Cluster] [Server] send message {ws_message}: {error}"),
                                }
                            }
                            Message::Close(ws_message) => {
                                trace!("[Tardis.Cluster] [Server] message receive: close {:?}", ws_message);
                            }
                            _ => {
                                warn!("[Tardis.Cluster] [Server] message receive: the type is not implemented");
                            }
                        }
                    }
                });
                tokio::spawn(async move {
                    while let Ok(message_resp) =
                        SERVER_MESSAGE_RESPONDER.read().await.as_ref().expect("Global variable [SERVER_MESSAGE_RESPONDER] doesn't exist").subscribe().recv().await
                    {
                        trace!("[Tardis.Cluster] [Server] response message {:?}", message_resp.clone());
                        if let Err(error) = socket_write.send(Message::Text(TardisFuns::json.obj_to_string(&message_resp).expect("ignore"))).await {
                            error!("[Tardis.Cluster] [Server] response message {message_resp:?}: {error}");
                            break;
                        }
                    }
                });
            })
            .boxed()
    }
}
