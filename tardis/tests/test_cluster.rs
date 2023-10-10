use std::{
    env,
    ffi::OsStr,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use async_trait::async_trait;
use futures_util::future::join_all;
use serde_json::{json, Value};
use tardis::{
    basic::result::TardisResult,
    cluster::cluster_processor::{self, TardisClusterMessageReq, TardisClusterSubscriber},
    config::config_dto::{CacheModuleConfig, ClusterConfig, FrameworkConfig, TardisConfig, WebServerCommonConfig, WebServerConfig, WebServerModuleConfig},
    consts::IP_LOCALHOST,
    test::test_container::TardisTestContainer,
    TardisFuns,
};
use testcontainers::clients;
use tokio::{process::Command, time::sleep};
use tracing::info;

#[tokio::test(flavor = "multi_thread")]
async fn test_cluster() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis=debug");
    let cluster_url = env::var("cluster_url");

    if let Ok(cluster_url) = cluster_url {
        start_node(cluster_url, &env::var("node_id").unwrap()).await?;
    } else {
        let program = env::args().next().as_ref().map(Path::new).and_then(Path::file_name).and_then(OsStr::to_str).map(String::from).unwrap();

        let docker = clients::Cli::default();
        let redis_container = TardisTestContainer::redis_custom(&docker);
        let cluster_url = format!("redis://127.0.0.1:{}/0", redis_container.get_host_port_ipv4(6379));

        let results = join_all(vec![
            invoke_node(&cluster_url, "1", &program),
            invoke_node(&cluster_url, "2", &program),
            invoke_node(&cluster_url, "3", &program),
        ])
        .await;
        assert!(results.into_iter().all(|r| r.unwrap()));
    }

    Ok(())
}

async fn invoke_node(cluster_url: &str, node_id: &str, program: &str) -> TardisResult<bool> {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .env("cluster_url", cluster_url)
            .env("node_id", node_id)
            .env("LS_COLORS", "rs=0:di=38;5;27:mh=44;38;5;15")
            .args(["/C", program])
            .output()
            .await
            .expect("failed to execute process")
    } else {
        Command::new("sh")
            .env("cluster_url", cluster_url)
            .env("node_id", node_id)
            .env("LS_COLORS", "rs=0:di=38;5;27:mh=44;38;5;15")
            .arg("-c")
            .arg(program)
            .output()
            .await
            .expect("failed to execute process")
    };
    let output_msg = String::from_utf8(strip_ansi_escapes::strip(output.stdout)).unwrap();
    println!("{node_id} stdout:");
    output_msg.lines().for_each(|line| println!("{line}"));

    Ok(!output_msg.contains("test result: FAILED"))
}

async fn start_node(cluster_url: String, node_id: &str) -> TardisResult<()> {
    cluster_processor::set_node_id(&format!("node_{node_id}")).await;
    TardisFuns::init_conf(TardisConfig {
        cs: Default::default(),
        fw: FrameworkConfig::builder()
            .web_server(
                WebServerConfig::builder().common(WebServerCommonConfig::builder().access_host(IP_LOCALHOST).port(80).build()).default(WebServerModuleConfig::default()).build(),
            )
            .cache(CacheModuleConfig::builder().url(cluster_url.parse().unwrap()).build())
            .cluster(ClusterConfig {
                watch_kind: "cache".to_string(),
                k8s_svc: None,
                cache_check_interval_sec: Some(1),
            })
            .build(),
    })
    .await
    .unwrap();

    if node_id == "2" {
        sleep(Duration::from_secs(2)).await;
    } else if node_id == "3" {
        sleep(Duration::from_secs(4)).await;
    }
    TardisFuns::web_server().start().await?;
    sleep(Duration::from_secs(1)).await;

    test_ping(node_id).await?;
    test_echo(node_id).await?;

    if node_id == "1" {
        sleep(Duration::from_secs(1)).await;
    } else if node_id == "2" {
        sleep(Duration::from_secs(7)).await;
    } else {
        sleep(Duration::from_secs(10)).await;
    }
    Ok(())
}

static PING_COUNTER: AtomicUsize = AtomicUsize::new(0);

async fn test_ping(node_id: &str) -> TardisResult<()> {
    TardisFuns::cluster_subscribe_event("ping", Box::new(ClusterSubscriberPingTest {})).await;
    if node_id == "1" {
        // expect hit 0 times
        let result = TardisFuns::cluster_publish_event("ping", json!(1000), None).await;
        assert!(result.is_err());
        sleep(Duration::from_secs(5)).await;
        // expect hit 2 times (to node_2, node_3)
        TardisFuns::cluster_publish_event("ping", json!(400), None).await?;
        sleep(Duration::from_secs(5)).await;
        assert_eq!(PING_COUNTER.load(Ordering::SeqCst), 50 + 4);
    } else if node_id == "2" {
        // expect hit 1 times (to node_1)
        TardisFuns::cluster_publish_event("ping", json!(50), None).await?;
        sleep(Duration::from_secs(5)).await;
        assert_eq!(PING_COUNTER.load(Ordering::SeqCst), 400 + 4);
    } else {
        // expect hit 2 times (to node_1, node_2)
        TardisFuns::cluster_publish_event("ping", json!(4), None).await?;
        sleep(Duration::from_secs(5)).await;
        assert_eq!(PING_COUNTER.load(Ordering::SeqCst), 400);
    }
    Ok(())
}

struct ClusterSubscriberPingTest;

#[async_trait]
impl TardisClusterSubscriber for ClusterSubscriberPingTest {
    async fn subscribe(&self, message_req: TardisClusterMessageReq) -> TardisResult<Option<Value>> {
        info!("message_req:{message_req:?}");
        PING_COUNTER.fetch_add(message_req.msg.as_i64().unwrap() as usize, Ordering::SeqCst);
        Ok(None)
    }
}

struct ClusterSubscriberEchoTest;

#[async_trait]
impl TardisClusterSubscriber for ClusterSubscriberEchoTest {
    async fn subscribe(&self, message_req: TardisClusterMessageReq) -> TardisResult<Option<Value>> {
        info!("message_req:{message_req:?}");
        Ok(Some(serde_json::Value::String(format!("echo {}", message_req.req_node_id))))
    }
}

async fn test_echo(node_id: &str) -> TardisResult<()> {
    TardisFuns::cluster_subscribe_event("echo", Box::new(ClusterSubscriberEchoTest {})).await;
    if node_id == "1" {
        let resp = TardisFuns::cluster_publish_event_and_wait_resp("echo", serde_json::Value::String("hi".to_string()), "node_3").await?;
        assert_eq!(resp.msg.as_str().unwrap(), &format!("echo node_{node_id}"));
        assert_eq!(&resp.resp_node_id, "node_3");
    } else if node_id == "2" {
        let resp = TardisFuns::cluster_publish_event_and_wait_resp("echo", serde_json::Value::String("hi".to_string()), "node_3").await?;
        assert_eq!(resp.msg.as_str().unwrap(), &format!("echo node_{node_id}"));
        assert_eq!(&resp.resp_node_id, "node_3");
    } else {
        let resp = TardisFuns::cluster_publish_event_and_wait_resp("echo", serde_json::Value::String("hi".to_string()), "node_3").await;
        assert!(resp.is_err());
    }
    Ok(())
}
