use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use futures::Future;
use futures_util::{SinkExt, StreamExt};
use poem::web::websocket::{BoxWebSocketUpgraded, Message, WebSocket};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, instrument, trace, warn};

use crate::basic::error::TardisError;
use crate::cluster::cluster_publish::ClusterEvent;
use crate::cluster::cluster_receive::init_response_dispatcher;
use crate::cluster::cluster_watch_by_cache;
#[cfg(feature = "k8s")]
use crate::cluster::cluster_watch_by_k8s;
use crate::config::config_dto::FrameworkConfig;
use crate::tardis_static;
use crate::web::web_server::status_api::TardisStatus;
use crate::web::web_server::TardisWebServer;
use crate::web::ws_client::TardisWSClient;
use crate::web::ws_processor::ws_insts_mapping_avatars;
// use crate::web::ws_processor::cluster_protocol::Avatar;
use crate::{basic::result::TardisResult, TardisFuns};

pub const CLUSTER_NODE_WHOAMI: &str = "__cluster_node_who_am_i__";
/// cluster ping event
pub const EVENT_PING: &str = "tardis/ping";
/// cluster status check event
pub const EVENT_STATUS: &str = "tardis/status";
pub const CLUSTER_MESSAGE_CACHE_SIZE: usize = 10000;
pub const WHOAMI_TIMEOUT: Duration = Duration::from_secs(30);

tardis_static! {
    pub async set local_socket_addr: SocketAddr;
    pub async set local_node_id: String;
    pub async set responser_dispatcher: mpsc::Sender<TardisClusterMessageResp>;
    pub(crate) cache_nodes: Arc<RwLock<HashMap<ClusterRemoteNodeKey, TardisClusterNodeRemote>>>;
    pub(crate) subscribers: Arc<RwLock<HashMap<String, ClusterHandlerObj>>>;
}

/// clone the cache_nodes_info at current time
pub async fn load_cache_nodes_info() -> HashMap<ClusterRemoteNodeKey, TardisClusterNodeRemote> {
    cache_nodes().read().await.clone()
}

pub async fn peer_count() -> usize {
    cache_nodes().read().await.keys().filter(|k| matches!(k, ClusterRemoteNodeKey::NodeId(_))).count()
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum ClusterRemoteNodeKey {
    SocketAddr(SocketAddr),
    NodeId(String),
}

impl From<SocketAddr> for ClusterRemoteNodeKey {
    fn from(val: SocketAddr) -> Self {
        ClusterRemoteNodeKey::SocketAddr(val)
    }
}

impl From<String> for ClusterRemoteNodeKey {
    fn from(val: String) -> Self {
        ClusterRemoteNodeKey::NodeId(val)
    }
}

impl ClusterRemoteNodeKey {
    pub fn as_socket_addr(&self) -> Option<SocketAddr> {
        match self {
            ClusterRemoteNodeKey::SocketAddr(socket_addr) => Some(*socket_addr),
            _ => None,
        }
    }
    pub fn as_node_id(&self) -> Option<String> {
        match self {
            ClusterRemoteNodeKey::NodeId(node_id) => Some(node_id.clone()),
            _ => None,
        }
    }
}

impl std::fmt::Display for ClusterRemoteNodeKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClusterRemoteNodeKey::SocketAddr(socket_addr) => write!(f, "{}", socket_addr),
            ClusterRemoteNodeKey::NodeId(node_id) => write!(f, "[id]{}", node_id),
        }
    }
}

pub type ClusterMessageId = String;

/// Cluster event subscriber trait, a subscriber object can be registered to the cluster event system and respond to the event
///
/// # Register
/// see [`subscribe`], [`subscribe_boxed`] and [`subscribe_if_not_exist`]
pub trait ClusterHandler: Send + Sync + 'static {
    fn event_name(&self) -> String;
    fn handle(self: Arc<Self>, message_req: TardisClusterMessageReq) -> impl Future<Output = TardisResult<Option<Value>>> + Send;
}

pub struct ClusterHandlerObj {
    pub event_name: String,
    pub handle: Box<dyn Fn(TardisClusterMessageReq) -> Pin<Box<dyn Future<Output = TardisResult<Option<Value>>> + Send>> + Send + Sync>,
}
impl ClusterHandlerObj {
    pub fn new<H: ClusterHandler>(handler: H) -> Self {
        let acred = Arc::new(handler);
        Self {
            event_name: acred.event_name(),
            handle: Box::new(move |message_req| {
                let cloned = acred.clone();
                let fut = cloned.handle(message_req);
                Box::pin(fut)
            }),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum ClusterEventTarget {
    #[default]
    /// broadcast to all known nodes that id is known
    Broadcast,
    /// to single remote node
    Single(ClusterRemoteNodeKey),
    /// to multi nodes
    Multi(Vec<ClusterRemoteNodeKey>),
    /// raw client
    Client(Arc<TardisWSClient>),
}

impl ClusterEventTarget {
    pub fn multi<V: Into<ClusterRemoteNodeKey>, I: IntoIterator<Item = V>>(iter: I) -> Self {
        ClusterEventTarget::Multi(iter.into_iter().map(|v| v.into()).collect())
    }
}

impl From<SocketAddr> for ClusterEventTarget {
    fn from(val: SocketAddr) -> Self {
        ClusterEventTarget::Single(ClusterRemoteNodeKey::SocketAddr(val))
    }
}

impl From<String> for ClusterEventTarget {
    fn from(val: String) -> Self {
        ClusterEventTarget::Single(ClusterRemoteNodeKey::NodeId(val))
    }
}

impl<'s> From<&'s str> for ClusterEventTarget {
    fn from(val: &'s str) -> Self {
        ClusterEventTarget::Single(ClusterRemoteNodeKey::NodeId(val.to_string()))
    }
}

impl<S: Into<String>> From<Vec<S>> for ClusterEventTarget {
    fn from(val: Vec<S>) -> Self {
        ClusterEventTarget::Multi(val.into_iter().map(|id| ClusterRemoteNodeKey::NodeId(id.into())).collect::<Vec<_>>())
    }
}

impl From<Arc<TardisWSClient>> for ClusterEventTarget {
    fn from(val: Arc<TardisWSClient>) -> Self {
        ClusterEventTarget::Client(val)
    }
}

struct EventPing;

impl ClusterHandler for EventPing {
    fn event_name(&self) -> String {
        EVENT_PING.to_string()
    }
    async fn handle(self: Arc<Self>, _message_req: TardisClusterMessageReq) -> TardisResult<Option<Value>> {
        Ok(Some(serde_json::to_value(local_node_id().await).expect("spec always be a valid json value")))
    }
}

pub(crate) struct EventStatus;

impl ClusterHandler for EventStatus {
    fn event_name(&self) -> String {
        EVENT_STATUS.to_string()
    }

    async fn handle(self: Arc<Self>, _message_req: TardisClusterMessageReq) -> TardisResult<Option<Value>> {
        Ok(Some(serde_json::to_value(TardisStatus::fetch().await).expect("status always be a valid json value")))
    }
}

impl EventStatus {
    pub async fn get_by_id(cluster_id: &str) -> TardisResult<TardisStatus> {
        if cluster_id == *local_node_id().await {
            Ok(TardisStatus::fetch().await)
        } else {
            let resp = publish_event_one_response(
                EventStatus.event_name(),
                Default::default(),
                ClusterEventTarget::Single(ClusterRemoteNodeKey::NodeId(cluster_id.to_string())),
                None,
            )
            .await?;
            serde_json::from_value(resp.msg).map_err(|e| {
                let error_info = format!("[Tardis.Cluster] [Client] receive message error: {e}");
                TardisError::wrap(&error_info, "-1-tardis-cluster-receive-message-error")
            })
        }
    }
}

pub async fn init_by_conf(conf: &FrameworkConfig, cluster_server: &TardisWebServer) -> TardisResult<()> {
    if let Some(cluster_config) = &conf.cluster {
        let web_server_config = conf.web_server.as_ref().expect("missing web server config");
        let access_host = web_server_config.access_host.unwrap_or(web_server_config.host);
        let access_port = web_server_config.access_port.unwrap_or(web_server_config.port);
        let access_addr = SocketAddr::new(access_host, access_port);
        info!("[Tardis.Cluster] Initializing cluster");
        init_node(cluster_server, access_addr).await?;
        match cluster_config.watch_kind.to_lowercase().as_str() {
            #[cfg(feature = "k8s")]
            "k8s" => {
                info!("[Tardis.Cluster] Initializing cluster by k8s watch");
                cluster_watch_by_k8s::init(cluster_config, web_server_config).await?;
            }
            "cache" => {
                info!("[Tardis.Cluster] Initializing cluster by default watch");
                cluster_watch_by_cache::init(cluster_config, web_server_config).await?;
            }
            _ => panic!("[Tardis.Cluster] Unsupported cluster watch kind: {}", cluster_config.watch_kind),
        }
        info!("[Tardis.Cluster] Initialized cluster");
    }
    Ok(())
}

async fn init_node(cluster_server: &TardisWebServer, access_addr: SocketAddr) -> TardisResult<()> {
    info!("[Tardis.Cluster] Initializing node");
    set_local_node_id(TardisFuns::field.nanoid());
    set_local_socket_addr(access_addr);
    debug!("[Tardis.Cluster] Initializing response dispatcher");
    set_responser_dispatcher(init_response_dispatcher());
    debug!("[Tardis.Cluster] Register exchange route");
    cluster_server.add_route(ClusterAPI).await;

    debug!("[Tardis.Cluster] Register default events");
    subscribe(EventPing).await;
    #[cfg(feature = "web-server")]
    {
        subscribe(EventStatus).await;
        subscribe(ws_insts_mapping_avatars().clone()).await;
    }

    info!("[Tardis.Cluster] Initialized node");
    Ok(())
}

#[instrument]
pub async fn refresh_nodes(active_nodes: &HashSet<SocketAddr>) -> TardisResult<()> {
    trace!("[Tardis.Cluster] Refreshing nodes");
    trace!("[Tardis.Cluster] Find all active nodes: {:?}", active_nodes);
    let mut cache_nodes = cache_nodes().write().await;
    let socket_set = cache_nodes.keys().filter_map(ClusterRemoteNodeKey::as_socket_addr).collect::<HashSet<_>>();
    // remove inactive nodes
    for inactive_node in socket_set.difference(active_nodes) {
        if let Some(remote) = cache_nodes.remove(&ClusterRemoteNodeKey::SocketAddr(*inactive_node)) {
            // load_cache_nodes_info()
            info!("[Tardis.Cluster] remove inactive node {remote:?} from cache");
            cache_nodes.remove(&ClusterRemoteNodeKey::NodeId(remote.node_id));
            // TODO
            // be nice to the server, close the connection
            // remote.client
        }
    }
    // add new nodes
    for new_nodes_addr in active_nodes.difference(&socket_set) {
        if local_socket_addr().await == new_nodes_addr {
            // skip local node
            continue;
        }
        let remote = add_remote_node(*new_nodes_addr).await?;
        info!("[Tardis.Cluster] New remote nodes: {remote:?}");

        cache_nodes.insert(ClusterRemoteNodeKey::SocketAddr(*new_nodes_addr), remote.clone());
        cache_nodes.insert(ClusterRemoteNodeKey::NodeId(remote.node_id.clone()), remote);
    }
    let mut table = String::new();
    for (k, v) in cache_nodes.iter() {
        use std::fmt::Write;
        if matches!(k, ClusterRemoteNodeKey::NodeId(_)) {
            writeln!(&mut table, "{k:20} | {v:40} ").expect("shouldn't fail");
        }
    }
    Ok(())
}

async fn add_remote_node(socket_addr: SocketAddr) -> TardisResult<TardisClusterNodeRemote> {
    if *local_socket_addr().await == socket_addr {
        return Err(TardisError::wrap(
            &format!("[Tardis.Cluster] [Client] add remote node {socket_addr}: can't add local node"),
            "-1-tardis-cluster-add-remote-node-error",
        ));
    }
    debug!("[Tardis.Cluster] Connect node: {socket_addr}");
    // is this node
    let client = TardisFuns::ws_client(&format!("ws://{socket_addr}/tardis/cluster/ws/exchange"), move |message| async move {
        if let tokio_tungstenite::tungstenite::Message::Text(message) = message {
            match TardisFuns::json.str_to_obj::<TardisClusterMessageResp>(&message) {
                Ok(message_resp) => {
                    if let Err(error) = responser_dispatcher().await.send(message_resp).await {
                        error!("[Tardis.Cluster] [Client] response message {message}: {error}");
                    }
                }
                Err(error) => error!("[Tardis.Cluster] [Client] response message {message}: {error}"),
            }
        }
        None
    })
    .await?;
    let client = Arc::new(client);
    let resp = ClusterEvent::new(EVENT_PING).target(client.clone()).one_response(Some(WHOAMI_TIMEOUT)).publish_one_response().await?;
    let resp_node_id = resp.resp_node_id;
    let remote = TardisClusterNodeRemote { node_id: resp_node_id, client };
    Ok(remote)
}

/// subscribe a boxed cluster event
pub async fn subscribe_boxed(handler: ClusterHandlerObj) {
    let event_name = handler.event_name.clone();
    info!("[Tardis.Cluster] [Server] subscribe event {event_name}");
    subscribers().write().await.insert(event_name, handler);
}

/// subscribe a cluster event
pub async fn subscribe<H: ClusterHandler>(handler: H) {
    subscribe_boxed(ClusterHandlerObj::new(handler)).await;
}

/// subscribe a cluster event if not exist
pub async fn subscribe_if_not_exist<H: ClusterHandler>(handler: H) {
    let mut wg = subscribers().write().await;
    let event_name = handler.event_name();
    #[allow(clippy::map_entry)]
    if !wg.contains_key(&event_name) {
        info!("[Tardis.Cluster] [Server] subscribe event {event_name}");
        wg.insert(event_name, ClusterHandlerObj::new(handler));
    }
}

/// unsubscribe a cluster event
pub async fn unsubscribe(event_name: &str) {
    info!("[Tardis.Cluster] [Server] unsubscribe event {event_name}");
    subscribers().write().await.remove(event_name);
}

/// a request message for cluster
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TardisClusterMessageReq {
    pub(crate) msg_id: String,
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

/// a response message for cluster

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TardisClusterMessageResp {
    pub(crate) msg_id: String,
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
#[derive(Debug, Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct TardisClusterNodeSpecifier {
    pub id: String,
    pub socket_addr: SocketAddr,
}

pub struct TardisClusterNodeLocal {
    pub spec: TardisClusterNodeSpecifier,
}

#[derive(Debug, Clone)]
pub struct TardisClusterNodeRemote {
    pub node_id: String,
    pub client: Arc<TardisWSClient>,
}

impl std::fmt::Display for TardisClusterNodeRemote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{is_online} / {node_id} / {url}",
            is_online = if self.client.is_connected() { "online" } else { "offline" },
            node_id = self.node_id,
            url = self.client.url
        )
    }
}
pub enum TardisClusterNode {
    Local(TardisClusterNodeLocal),
    Remote(TardisClusterNodeRemote),
}

impl TardisClusterNode {}

use std::hash::Hash;

use super::cluster_publish::publish_event_one_response;

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
                                    if let Some(subscriber) = subscribers().read().await.get(message_req.event.as_str()) {
                                        let msg_id = message_req.msg_id();
                                        match (subscriber.handle)(message_req).await {
                                            Ok(Some(message_resp)) => {
                                                if let Err(error) = socket
                                                    .send(Message::Text(
                                                        TardisFuns::json
                                                            .obj_to_string(&TardisClusterMessageResp::new(message_resp.clone(), msg_id, local_node_id().await.to_string()))
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
