use std::{env, time::Duration};

use tardis::{
    basic::result::TardisResult,
    cluster::cluster_processor,
    config::config_dto::{CacheConfig, ClusterConfig, DBConfig, FrameworkConfig, MQConfig, MailConfig, OSConfig, SearchConfig, TardisConfig, WebServerConfig},
    test::test_container::TardisTestContainer,
    TardisFuns,
};
use testcontainers::clients;
use tokio::time::sleep;
use tracing::info;

#[tokio::test]
async fn test_cluster() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis=trace");
    let node_id = if env::var("other").is_ok() { "2" } else { "1" };

    let docker = clients::Cli::default();
    let redis_container = TardisTestContainer::redis_custom(&docker);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = if node_id == "1" {
        println!("=====\r\nredis port = {redis_port}\r\n=====");
        format!("redis://127.0.0.1:{redis_port}/0")
    } else {
        format!("redis://127.0.0.1:{}/0", env::var("redis_port").unwrap())
    };

    cluster_processor::set_node_id(&format!("node_{node_id}")).await;
    TardisFuns::init_conf(TardisConfig {
        cs: Default::default(),
        fw: FrameworkConfig {
            app: Default::default(),
            web_server: WebServerConfig {
                enabled: true,
                access_host: Some("127.0.0.1".to_string()),
                port: format!("80{node_id}").parse().unwrap(),
                ..Default::default()
            },
            cache: CacheConfig {
                enabled: true,
                url: redis_url,
                ..Default::default()
            },
            db: DBConfig {
                enabled: false,
                ..Default::default()
            },
            mq: MQConfig {
                enabled: false,
                ..Default::default()
            },
            search: SearchConfig {
                enabled: false,
                ..Default::default()
            },
            mail: MailConfig {
                enabled: false,
                ..Default::default()
            },
            os: OSConfig {
                enabled: false,
                ..Default::default()
            },
            cluster: Some(ClusterConfig {
                watch_kind: "cache".to_string(),
                k8s_svc: None,
                cache_check_interval_sec: Some(10),
            }),
            ..Default::default()
        },
    })
    .await
    .unwrap();

    TardisFuns::web_server().start().await?;

    test_by_cache(&node_id).await?;

    Ok(())
}

async fn test_by_cache(node_id: &str) -> TardisResult<()> {
    TardisFuns::cluster_subscribe_event("echo", |message_req| async move {
        info!("message_req:{message_req:?}");
        Ok(Some(serde_json::Value::String(format!("pong {}", message_req.req_node_id))))
    });
    if node_id != "1" {
        sleep(Duration::from_secs(1)).await;
        let resp = TardisFuns::cluster_publish_event_and_wait_resp("echo", serde_json::Value::String("ping".to_string()), &format!("node_1")).await?;
        assert_eq!(resp.msg.as_str().unwrap(), &format!("pong node_{node_id}"));
        assert_eq!(&resp.resp_node_id, "node_1");
    } else {
        sleep(Duration::from_secs(10000)).await;
    }

    Ok(())
}
