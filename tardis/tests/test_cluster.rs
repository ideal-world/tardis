use std::{
    env,
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use futures_util::future::join_all;
use serde_json::{json, Value};
use tardis::{
    basic::{result::TardisResult, tracing::TardisTracing},
    cluster::{
        cluster_broadcast::ClusterBroadcastChannel,
        cluster_hashmap::ClusterStaticHashMap,
        cluster_processor::{self, subscribe, ClusterEventTarget, ClusterHandler, TardisClusterMessageReq},
        cluster_publish::publish_event_one_response,
    },
    config::config_dto::{CacheModuleConfig, ClusterConfig, FrameworkConfig, LogConfig, TardisConfig, WebServerCommonConfig, WebServerConfig, WebServerModuleConfig},
    consts::IP_LOCALHOST,
    tardis_static,
    test::test_container::TardisTestContainer,
    TardisFuns,
};
use testcontainers::clients;
use tokio::{io::AsyncReadExt, process::Command, time::sleep};
use tracing::info;
use tracing_subscriber::filter::Directive;

#[tokio::test(flavor = "multi_thread")]
async fn test_cluster() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis=debug");
    let cluster_url = env::var("cluster_url");
    if let Ok(cluster_url) = cluster_url {
        start_node(cluster_url, &env::var("node_id").unwrap()).await?;
    } else {
        // let program = env::args().next().as_ref().map(Path::new).and_then(Path::file_name).and_then(OsStr::to_str).map(String::from).unwrap();
        let program = env::current_exe()?;

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

async fn invoke_node(cluster_url: &str, node_id: &str, program: &Path) -> TardisResult<bool> {
    let mut child = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .env("cluster_url", cluster_url)
            .env("node_id", node_id)
            .env("LS_COLORS", "rs=0:di=38;5;27:mh=44;38;5;15")
            .arg("/C")
            .arg(program)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?
    } else {
        Command::new("sh")
            .env("cluster_url", cluster_url)
            .env("node_id", node_id)
            .env("LS_COLORS", "rs=0:di=38;5;27:mh=44;38;5;15")
            .arg("-c")
            .arg(program)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?
    };
    let mut buf = [0; 1024];
    let mut err_buf = [0; 1024];
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    loop {
        tokio::select! {
            result = stdout.read(&mut buf) => {
                let size = result?;
                if size != 0 {
                    println!("node[{node_id}]/stdout:");
                    println!("{}", String::from_utf8_lossy(&buf[..size]));
                }
            }
            result = stderr.read(&mut err_buf) => {
                let size = result?;
                if size != 0 {
                    println!("node[{node_id}]/stdout:");
                    println!("{}", String::from_utf8_lossy(&err_buf[..size]));
                }
            }
            exit_code = child.wait() => {
                if let Ok(exit_code) = exit_code {
                    return Ok(exit_code.success())
                } else {
                    return Ok(false)
                }
            }

        };
    }
}

async fn start_node(cluster_url: String, node_id: &str) -> TardisResult<()> {
    subscribe(map().clone()).await;
    // subscribe
    broadcast();
    cluster_processor::set_local_node_id(format!("node_{node_id}"));
    let port = portpicker::pick_unused_port().unwrap();
    TardisTracing::initializer().with_fmt_layer().with_env_layer().init();
    TardisFuns::init_conf(TardisConfig {
        cs: Default::default(),
        fw: FrameworkConfig::builder()
            .web_server(
                WebServerConfig::builder().common(WebServerCommonConfig::builder().access_host(IP_LOCALHOST).port(port).build()).default(WebServerModuleConfig::default()).build(),
            )
            .cache(CacheModuleConfig::builder().url(cluster_url.parse().unwrap()).build())
            .cluster(ClusterConfig {
                watch_kind: "cache".to_string(),
                k8s_ns: None,
                k8s_svc: None,
                cache_check_interval_sec: Some(1),
            })
            .log(
                LogConfig::builder()
                    .level("info".parse::<Directive>().unwrap_or_default())
                    .directives(if node_id == "2" {
                        ["tardis=trace".parse::<Directive>().unwrap()]
                    } else {
                        ["tardis=debug".parse::<Directive>().unwrap()]
                    })
                    .build(),
            )
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
    let task = {
        let node_id = node_id.to_string();
        {
            let node_id = node_id.to_string();
            tokio::spawn(async move {
                let mut receiver = broadcast().subscribe();
                while let Ok(msg) = receiver.recv().await {
                    println!("node[{node_id}]/broadcast: {msg}");
                    bc_recv_count().fetch_add(1, Ordering::SeqCst);
                }
            });
        }
        tokio::spawn(async move {
            test_broadcast(&node_id).await;
        })
    };
    test_ping(node_id).await?;
    test_echo(node_id).await?;
    test_hash_map(node_id).await?;

    if node_id == "1" {
        sleep(Duration::from_secs(1)).await;
    } else if node_id == "2" {
        sleep(Duration::from_secs(7)).await;
    } else {
        sleep(Duration::from_secs(10)).await;
    }
    let result = tokio::join!(task);
    result.0.unwrap();
    Ok(())
}

static PING_COUNTER: AtomicUsize = AtomicUsize::new(0);

async fn test_ping(node_id: &str) -> TardisResult<()> {
    TardisFuns::cluster_subscribe_event(ClusterSubscriberPingTest).await;
    if node_id == "1" {
        // expect hit 0 times
        let result = TardisFuns::cluster_publish_event("ping", json!(1000), ClusterEventTarget::Broadcast).await;
        assert!(result.is_err());
        sleep(Duration::from_secs(5)).await;
        // expect hit 2 times (to node_2, node_3)
        TardisFuns::cluster_publish_event("ping", json!(400), ClusterEventTarget::Broadcast).await?;
        sleep(Duration::from_secs(5)).await;
        assert_eq!(PING_COUNTER.load(Ordering::SeqCst), 50 + 4);
    } else if node_id == "2" {
        // expect hit 1 times (to node_1)
        TardisFuns::cluster_publish_event("ping", json!(50), ClusterEventTarget::Broadcast).await?;
        sleep(Duration::from_secs(5)).await;
        assert_eq!(PING_COUNTER.load(Ordering::SeqCst), 400 + 4);
    } else {
        // expect hit 2 times (to node_1, node_2)
        TardisFuns::cluster_publish_event("ping", json!(4), ClusterEventTarget::Broadcast).await?;
        sleep(Duration::from_secs(5)).await;
        assert_eq!(PING_COUNTER.load(Ordering::SeqCst), 400);
    }
    Ok(())
}

struct ClusterSubscriberPingTest;

impl ClusterHandler for ClusterSubscriberPingTest {
    fn event_name(&self) -> String {
        "ping".into()
    }
    async fn handle(self: Arc<Self>, message_req: TardisClusterMessageReq) -> TardisResult<Option<Value>> {
        info!("message_req:{message_req:?}");
        PING_COUNTER.fetch_add(message_req.msg.as_i64().unwrap() as usize, Ordering::SeqCst);
        Ok(None)
    }
}

struct ClusterSubscriberEchoTest;

impl ClusterHandler for ClusterSubscriberEchoTest {
    fn event_name(&self) -> String {
        "echo".into()
    }
    async fn handle(self: Arc<Self>, message_req: TardisClusterMessageReq) -> TardisResult<Option<Value>> {
        info!("message_req:{message_req:?}");
        Ok(Some(serde_json::Value::String(format!("echo {}", message_req.req_node_id))))
    }
}

async fn test_echo(node_id: &str) -> TardisResult<()> {
    TardisFuns::cluster_subscribe_event(ClusterSubscriberEchoTest).await;
    if node_id == "1" {
        let resp = TardisFuns::cluster_publish_event_one_resp("echo", serde_json::Value::String("hi".to_string()), "node_3").await?;
        assert_eq!(resp.msg.as_str().unwrap(), &format!("echo node_{node_id}"));
        assert_eq!(&resp.resp_node_id, "node_3");
    } else if node_id == "2" {
        let resp = publish_event_one_response("echo", serde_json::Value::String("hi".to_string()), "node_3", Some(Duration::from_secs(1))).await;
        assert!(resp.is_ok());
    } else {
        let resp = TardisFuns::cluster_publish_event_one_resp("echo", serde_json::Value::String("hi".to_string()), "node_3").await;
        assert!(resp.is_err());
    }
    Ok(())
}

tardis_static! {
    pub map: ClusterStaticHashMap<String, String> = ClusterStaticHashMap::new("test");
    broadcast: Arc<ClusterBroadcastChannel<String>> = ClusterBroadcastChannel::new("test_channel", 100);
    bc_recv_count: AtomicUsize = AtomicUsize::new(0);
}
async fn test_hash_map(node_id: &str) -> TardisResult<()> {
    match node_id {
        "1" => {
            map().insert("item1".to_string(), "from_node1".to_string()).await?;
            let value = map().get("item1".to_string()).await?;
            assert_eq!(value, Some("from_node1".to_string()));
        }
        "2" => {
            map().insert("item2".to_string(), "from_node2".to_string()).await?;
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let value = map().get("item1".to_string()).await?;
                if value.is_some() {
                    assert_eq!(value, Some("from_node1".to_string()));
                    break;
                }
            }
            let value = map().get("item2".to_string()).await?;
            assert_eq!(value, Some("from_node2".to_string()));
            tokio::time::sleep(Duration::from_secs(5)).await;
            map().remove("item2".to_string()).await?;
            let value = map().get("item2".to_string()).await?;
            assert_eq!(value, None);
        }
        "3" => {}
        _ => {}
    }
    Ok(())
}

async fn test_broadcast(node_id: &str) {
    TardisFuns::cluster_subscribe_event(ClusterSubscriberEchoTest).await;
    tokio::time::sleep(Duration::from_secs(6)).await;
    match node_id {
        "1" => {
            broadcast().send("message1-1".to_string()).await.expect("send failed");
            broadcast().send("message1-2".to_string()).await.expect("send failed");
        }
        "2" => {
            broadcast().send("message2-1".to_string()).await.expect("send failed");
            broadcast().send("message2-2".to_string()).await.expect("send failed");
        }
        "3" => {
            broadcast().send("message3-1".to_string()).await.expect("send failed");
            broadcast().send("message3-2".to_string()).await.expect("send failed");
        }
        _ => {}
    }
    let result = tokio::time::timeout(Duration::from_secs(20), async move {
        loop {
            if bc_recv_count().load(Ordering::SeqCst) == 6 {
                break;
            } else {
                tokio::task::yield_now().await;
            }
        }
    })
    .await;
    assert!(result.is_ok(), "bc_recv_count={}", bc_recv_count().load(Ordering::SeqCst));
}
