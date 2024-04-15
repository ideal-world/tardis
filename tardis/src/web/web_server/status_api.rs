//！ # Status Api
//！ For debug usage, get the current status of the tardis server.
//！
use poem_openapi::{param::Query, Object};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::web::web_resp::{TardisApiResult, TardisResp};
#[derive(Debug, Clone)]
pub struct TardisStatusApi;
#[derive(Debug, Serialize, Deserialize, Object)]
pub struct TardisStatus {
    pub version: String,
    #[cfg(feature = "cluster")]
    pub cluster: TardisClusterStatus,
    pub fw_config: serde_json::Value,
}

impl TardisStatus {
    pub async fn fetch() -> TardisStatus {
        TardisStatus {
            version: env!("CARGO_PKG_VERSION").to_string(),
            #[cfg(feature = "cluster")]
            cluster: TardisClusterStatus::fetch().await,
            fw_config: serde_json::to_value(crate::TardisFuns::fw_config().as_ref().clone()).unwrap_or_default(),
        }
    }
}

#[cfg(feature = "cluster")]
#[derive(Debug, Serialize, Deserialize, Object)]
pub struct TardisClusterStatus {
    pub cluster_id: String,
    pub peer_nodes: HashMap<String, String>,
    pub subscribed: Vec<String>,
}

#[cfg(feature = "cluster")]
impl TardisClusterStatus {
    pub async fn fetch() -> TardisClusterStatus {
        TardisClusterStatus {
            cluster_id: crate::cluster::cluster_processor::local_node_id().await.to_string(),
            peer_nodes: crate::cluster::cluster_processor::cache_nodes().read().await.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            subscribed: crate::cluster::cluster_processor::subscribers().read().await.iter().map(|(k, _)| k.to_string()).collect(),
        }
    }
}

#[poem_openapi::OpenApi]
impl TardisStatusApi {
    #[allow(unused_variables)]
    #[oai(path = "/status", method = "get")]
    pub async fn status(&self, cluster_id: Query<Option<String>>) -> TardisApiResult<TardisStatus> {
        let cluster_id = cluster_id.0;
        if let Some(id) = cluster_id {
            #[cfg(feature = "cluster")]
            {
                if id == *crate::cluster::cluster_processor::local_node_id().await {
                    return TardisResp::ok(TardisStatus::fetch().await);
                }
                let status = crate::cluster::cluster_processor::EventStatus::get_by_id(&id).await?;
                return TardisResp::ok(status);
            }
            #[cfg(not(feature = "cluster"))]
            {
                return TardisResp::err(crate::basic::error::TardisError::internal_error("cluster features not enabled", ""));
            }
        }
        TardisResp::ok(TardisStatus::fetch().await)
    }
}
