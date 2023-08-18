use std::collections::HashMap;
use std::sync::Arc;

use futures_util::future::join_all;
use futures_util::{SinkExt, StreamExt};
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
use async_trait::async_trait;

pub const CLUSTER_NODE_WHOAMI: &str = "__cluster_node_who_am_i__";
pub const CLUSTER_MESSAGE_CACHE_SIZE: usize = 10000;

lazy_static! {
    static ref SUBSCRIBES: Arc<RwLock<HashMap<String, Box<dyn TardisClusterSubscriber>>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref CLIENT_MESSAGE_RESPONDER: Arc<RwLock<Option<broadcast::Sender<TardisClusterMessageResp>>>> = Arc::new(RwLock::new(None));
    static ref CLUSTER_CACHE_NODES: Arc<RwLock<Vec<TardisClusterNode>>> = Arc::new(RwLock::new(Vec::new()));
    static ref CLUSTER_CURRENT_NODE_ID: Arc<RwLock<String>> = Arc::new(RwLock::new(String::new()));
}

#[async_trait]
pub trait TardisClusterSubscriber: Send + Sync + 'static {
    async fn subscribe(&self, message_req: TardisClusterMessageReq) -> TardisResult<Option<Value>>;
}

struct ClusterSubscriberWhoAmI;

#[async_trait]
impl TardisClusterSubscriber for ClusterSubscriberWhoAmI {
    async fn subscribe(&self, _message_req: TardisClusterMessageReq) -> TardisResult<Option<Value>> {
        Ok(Some(Value::String(CLUSTER_CURRENT_NODE_ID.read().await.to_string())))
    }
}

pub async fn init_by_conf(conf: &FrameworkConfig, cluster_server: &TardisWebServer) -> TardisResult<()> {
    if let Some(cluster_config) = &conf.cluster {
        info!("[Tardis.Cluster] Initializing cluster");
        init_node(cluster_server).await?;
        match cluster_config.watch_kind.to_lowercase().as_str() {
            #[cfg(feature = "k8s")]
            "k8s" => {
                info!("[Tardis.Cluster] Initializing cluster by k8s watch");
                cluster_watch_by_k8s::init(cluster_config, &conf.web_server).await?;
            }
            "cache" => {
                info!("[Tardis.Cluster] Initializing cluster by default watch");
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
    if CLUSTER_CURRENT_NODE_ID.read().await.is_empty() {
        *CLUSTER_CURRENT_NODE_ID.write().await = TardisFuns::field.nanoid();
    }
    if !CLIENT_MESSAGE_RESPONDER.read().await.is_some() {
        *CLIENT_MESSAGE_RESPONDER.write().await = Some(broadcast::channel::<TardisClusterMessageResp>(CLUSTER_MESSAGE_CACHE_SIZE).0);
    }
    debug!("[Tardis.Cluster] Register exchange route");
    cluster_server.add_route(ClusterAPI).await;

    debug!("[Tardis.Cluster] Register default events");
    subscribe_event(CLUSTER_NODE_WHOAMI, Box::new(ClusterSubscriberWhoAmI {})).await;

    info!("[Tardis.Cluster] Initialized node");
    Ok(())
}

pub async fn set_node_id(node_id: &str) {
    *CLUSTER_CURRENT_NODE_ID.write().await = node_id.to_string();
}

pub async fn refresh_nodes(active_nodes: Vec<(String, u16)>) -> TardisResult<()> {
    trace!("[Tardis.Cluster] Refreshing nodes");
    trace!("[Tardis.Cluster] Find all active nodes: {:?}", active_nodes);
    let mut cache_nodes = CLUSTER_CACHE_NODES.write().await;
    trace!("[Tardis.Cluster] Try remove inactive nodes from cache");
    cache_nodes.retain(|cache_node| active_nodes.iter().any(|(active_node_ip, active_node_port)| cache_node.ip == *active_node_ip && cache_node.port == *active_node_port));
    trace!("[Tardis.Cluster] Try add new active nodes to cache");
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
    let resp_node_id = publish_event_wait_resp(&whoami_msg_id).await?.resp_node_id;

    let is_current_node = resp_node_id == CLUSTER_CURRENT_NODE_ID.read().await.as_str();
    if !is_current_node {
        info!("[Tardis.Cluster] Join node: {node_ip}:{node_port}")
    }
    Ok(TardisClusterNode {
        id: resp_node_id,
        ip: node_ip.to_string(),
        port: node_port,
        current: is_current_node,
        client: if !is_current_node { Some(client) } else { None },
    })
}

pub async fn subscribe_event(event: &str, sub_fun: Box<dyn TardisClusterSubscriber>) {
    info!("[Tardis.Cluster] [Server] subscribe event {event}");
    SUBSCRIBES.write().await.insert(event.to_string(), sub_fun);
}

pub async fn publish_event(event: &str, message: Value, node_ids: Option<Vec<&str>>) -> TardisResult<String> {
    trace!("[Tardis.Cluster] [Client] publish event {event} , message {message} , to {node_ids:?}");
    let cache_nodes = CLUSTER_CACHE_NODES.read().await;
    if !cache_nodes.iter().any(|cache_node| !cache_node.current) {
        return Err(TardisError::not_found(
            &format!("[Tardis.Cluster] [Client] publish event {event} , message {message} : no active nodes found"),
            "404-tardis-cluster-publish-message-node-not-exit",
        ));
    }
    trace!(
        "[Tardis.Cluster] [Client] cache nodes {}",
        cache_nodes
            .iter()
            .map(|cache_node| format!("[node_id={} , {}:{} , current={}]", cache_node.id, cache_node.ip, cache_node.port, cache_node.current))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let node_clients = cache_nodes
        .iter()
        .filter(|cache_node| cache_node.client.is_some() && node_ids.as_ref().map(|node_ids| node_ids.contains(&cache_node.id.as_str())).unwrap_or(true))
        .map(|cache_node| (cache_node.client.as_ref().expect("ignore"), &cache_node.id))
        .collect::<Vec<_>>();
    if let Some(node_ids) = node_ids {
        if node_clients.len() != node_ids.len() {
            let not_found_node_ids = node_ids.into_iter().filter(|node_id| !node_clients.iter().any(|node_client| node_client.1 == node_id)).collect::<Vec<_>>().join(",");
            return Err(TardisError::not_found(
                &format!("[Tardis.Cluster] [Client] publish event {event} , message {message} to [{not_found_node_ids}] not found"),
                "404-tardis-cluster-publish-message-node-not-exit",
            ));
        }
    }
    do_publish_event(event, message, node_clients.iter().map(|n| n.0).collect::<Vec<_>>()).await
}

async fn do_publish_event(event: &str, message: Value, node_clients: Vec<&TardisWSClient>) -> TardisResult<String> {
    let message_req = TardisClusterMessageReq::new(message.clone(), event.to_string(), CLUSTER_CURRENT_NODE_ID.read().await.to_string());
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
        Ok(message_req.msg_id())
    }
}

async fn publish_event_wait_resp(msg_id: &str) -> TardisResult<TardisClusterMessageResp> {
    loop {
        match CLIENT_MESSAGE_RESPONDER.read().await.as_ref().expect("Global variable [CLIENT_MESSAGE_RESPONDER] doesn't exist").subscribe().recv().await {
            Ok(message_response) => {
                if message_response.msg_id == msg_id {
                    return Ok(message_response);
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

pub async fn publish_event_and_wait_resp(event: &str, message: Value, node_id: &str) -> TardisResult<TardisClusterMessageResp> {
    trace!("[Tardis.Cluster] [Client] publish and wait resp, event {event} , message {message} , to {node_id}");
    let msg_id = publish_event(event, message, Some(vec![node_id])).await?;
    publish_event_wait_resp(&msg_id).await
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TardisClusterMessageReq {
    msg_id: String,
    pub req_node_id: String,
    pub msg: Value,
    pub event: String,
}

impl TardisClusterMessageReq {
    pub fn new(msg: Value, event: String, req_node_id: String) -> Self {
        Self {
            msg_id: TardisFuns::field.nanoid(),
            req_node_id,
            msg,
            event,
        }
    }

    pub fn msg_id(&self) -> String {
        self.msg_id.to_string()
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TardisClusterMessageResp {
    msg_id: String,
    pub resp_node_id: String,
    pub msg: Value,
}

impl TardisClusterMessageResp {
    pub fn new(msg: Value, msg_id: String, resp_node_id: String) -> Self {
        Self { msg_id, msg, resp_node_id }
    }

    pub fn msg_id(&self) -> String {
        self.msg_id.to_string()
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
            .on_upgrade(|mut socket| async move {
                while let Some(Ok(ws_message)) = socket.next().await {
                    match ws_message {
                        Message::Text(ws_message) => {
                            trace!("[Tardis.Cluster] [Server] receive message {ws_message}");
                            match TardisFuns::json.str_to_obj::<TardisClusterMessageReq>(&ws_message) {
                                Ok(message_req) => {
                                    if let Some(subscriber) = SUBSCRIBES.read().await.get(&message_req.event) {
                                        let msg_id = message_req.msg_id();
                                        match subscriber.subscribe(message_req).await {
                                            Ok(Some(message_resp)) => {
                                                if let Err(error) = socket
                                                    .send(Message::Text(
                                                        TardisFuns::json
                                                            .obj_to_string(&TardisClusterMessageResp::new(
                                                                message_resp.clone(),
                                                                msg_id,
                                                                CLUSTER_CURRENT_NODE_ID.read().await.to_string(),
                                                            ))
                                                            .expect("ignore"),
                                                    ))
                                                    .await
                                                {
                                                    error!("[Tardis.Cluster] [Server] response message {message_resp:?}: {error}");
                                                    break;
                                                }
                                            }
                                            Ok(None) => {}
                                            Err(error) => {
                                                warn!("[Tardis.Cluster] [Server] subscribe function by message {ws_message:?}: {error}");
                                            }
                                        }
                                    } else {
                                        warn!("[Tardis.Cluster] [Server] receive message {ws_message}: subscribe not found");
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
            })
            .boxed()
    }
}
