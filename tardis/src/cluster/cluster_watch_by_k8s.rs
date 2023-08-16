use k8s_openapi::api::core::v1::{Endpoints, Service};
use kube::{api::WatchParams, runtime::watcher, Api, Client};
use tracing::{error, trace};

use crate::{
    basic::{error::TardisError, result::TardisResult},
    config::config_dto::{ClusterConfig, WebServerConfig},
};
use futures::{StreamExt, TryStreamExt};

use super::cluster_processor;

pub async fn init(cluster_config: &ClusterConfig, webserver_config: &WebServerConfig) -> TardisResult<()> {
    let k8s_svc = cluster_config.k8s_svc.as_ref().expect("[Tardis.Cluster] [Client] need k8s_svc config in k8s mode").to_string();
    let web_server_port = webserver_config.port;

    refresh(&k8s_svc, web_server_port).await?;

    tokio::spawn(async move {
        if let Err(error) = watch(&k8s_svc, web_server_port).await {
            error!("[Tardis.Cluster] [Client] watch error: {}", error);
        }
    });
    Ok(())
}

async fn watch(k8s_svc: &str, web_server_port: u16) -> TardisResult<()> {
    let wc = watcher::Config::default();
    let endpoint_api: Api<Endpoints> = Api::all(get_client().await?);
    let mut endpoint_watcher = endpoint_api.watch(&WatchParams::default().fields(&format!("metadata.name={k8s_svc}")), "0").await?.boxed();
    while let Some(gateway_obj) = endpoint_watcher.try_next().await.unwrap_or_default() {
        refresh(&k8s_svc, web_server_port).await?;
    }
    Ok(())
}

async fn refresh(k8s_svc: &str, web_server_port: u16) -> TardisResult<()> {
    trace!("[Tardis.Cluster] [Client] watching");
    let service_api: Api<Service> = Api::all(get_client().await?);
    let service = service_api.get(k8s_svc).await?;
    let port_mapping = service
        .spec
        .as_ref()
        .and_then(|spec| spec.ports.as_ref())
        .and_then(|ports| {
            ports.iter().find(|port_obj| {
                port_obj
                    .target_port
                    .as_ref()
                    .map(|target_port| match target_port {
                        k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(target_port) => target_port == &(web_server_port as i32),
                        // TODO
                        k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::String(_) => true,
                    })
                    .unwrap_or(false)
            })
        })
        .map(|port_obj| port_obj.port)
        .ok_or_else(|| {
            TardisError::wrap(
                &format!("[Tardis.Cluster] [Client] kubernetes error: can not find node target_port for service {}", k8s_svc),
                "",
            )
        })? as u16;

    let endpoint_api: Api<Endpoints> = Api::all(get_client().await?);
    let endpoint = endpoint_api.get(k8s_svc).await?;
    // fetch all addresses from all subsets
    let active_nodes = endpoint
        .subsets
        .iter()
        .flat_map(|subsets| {
            subsets
                .iter()
                .flat_map(|subset| subset.addresses.as_ref().map(|addresses| addresses.iter().map(|address| address.ip.to_string()).collect::<Vec<_>>()).unwrap_or(Vec::new()))
        })
        .map(|ip: String| (ip, port_mapping))
        .collect::<Vec<_>>();
    cluster_processor::refresh_nodes(active_nodes).await?;
    Ok(())
}

async fn get_client() -> TardisResult<Client> {
    Client::try_default().await.map_err(|error| TardisError::wrap(&format!("[Tardis.Cluster] [Client] kubernetes error: {error:?}"), ""))
}
