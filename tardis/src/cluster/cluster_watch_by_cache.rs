use std::{collections::HashSet, net::SocketAddr, time::Duration};

use chrono::Utc;
use tokio::time;
use tracing::{error, trace};

use crate::{
    basic::result::TardisResult,
    cache::cache_client::TardisCacheClient,
    cluster::cluster_processor,
    config::config_dto::{component::WebServerConfig, ClusterConfig},
    TardisFuns,
};

pub const CACHE_NODE_INFO_KEY: &str = "tardis:cluster:node";
pub const CACHE_NODE_ALIVE_CHECK_DELAYED_TIMES: i8 = 3;

pub async fn init(cluster_config: &ClusterConfig, web_server_config: &WebServerConfig) -> TardisResult<()> {
    let access_host = web_server_config.access_host.unwrap_or(web_server_config.host);
    let access_port = web_server_config.access_port.unwrap_or(web_server_config.port);
    let cache_check_interval_sec = cluster_config.cache_check_interval_sec.unwrap_or(10);
    let access_addr = SocketAddr::new(access_host, access_port);
    // heart beat
    tokio::spawn(async move {
        let client = TardisFuns::cache();
        let mut interval = time::interval(Duration::from_secs(cache_check_interval_sec as u64));
        loop {
            {
                trace!("[Tardis.Cluster] [Client] heartbeat...");
                if let Err(error) = client.hset(CACHE_NODE_INFO_KEY, &access_addr.to_string(), &Utc::now().timestamp().to_string()).await {
                    error!("[Tardis.Cluster] [Client] heartbeat error: {}", error);
                }
            }
            interval.tick().await;
        }
    });
    tokio::spawn(async move {
        let client = TardisFuns::cache();
        let mut interval = time::interval(Duration::from_secs(cache_check_interval_sec as u64));
        loop {
            {
                if let Err(error) = watch(&client, cache_check_interval_sec).await {
                    error!("[Tardis.Cluster] [Client] watch error: {}", error);
                }
            }
            interval.tick().await;
        }
    });
    Ok(())
}

async fn watch(client: &TardisCacheClient, cache_check_interval_sec: i32) -> TardisResult<()> {
    trace!("[Tardis.Cluster] [Client] watching");
    let all_nodes = client.hgetall(CACHE_NODE_INFO_KEY).await?;
    let active_ts = Utc::now().timestamp() - cache_check_interval_sec as i64 * CACHE_NODE_ALIVE_CHECK_DELAYED_TIMES as i64 - 1;
    let active_nodes = all_nodes
        .iter()
        .filter_map(|(active_node_key, active_node_ts)| (active_node_ts.parse::<i64>().unwrap_or(i64::MIN) > active_ts).then_some(active_node_key))
        .filter_map(|active_node_key| active_node_key.parse::<SocketAddr>().ok())
        .collect::<HashSet<SocketAddr>>();
    cluster_processor::refresh_nodes(&active_nodes).await?;
    let inactive_nodes = all_nodes.iter().filter(|(_, active_node_ts)| active_node_ts.parse::<i64>().unwrap_or(i64::MIN) <= active_ts).collect::<Vec<_>>();
    for (inactive_node_key, _) in inactive_nodes {
        client.hdel(CACHE_NODE_INFO_KEY, inactive_node_key).await?;
    }
    Ok(())
}
