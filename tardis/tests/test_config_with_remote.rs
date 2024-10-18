// https://github.com/mehcode/config-rs

use std::collections::HashMap;
use std::env;

use poem_openapi_derive::OpenApi;
use reqwest::StatusCode;
use tardis::basic::result::TardisResult;
use tardis::cache::cache_client::TardisCacheClient;
use tardis::config::config_nacos::nacos_client::{NacosClient, NacosConfigDescriptor};
use tardis::serde::{Deserialize, Serialize};

use tardis::test::test_container::nacos_server::NacosServer;
use tardis::TardisFuns;

use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync;
use testcontainers_modules::rabbitmq::RabbitMq;
use testcontainers_modules::redis::Redis;
use tracing::{info, warn};

use std::sync::Arc;
use tokio::sync::Mutex;
#[derive(Debug, Clone)]
struct TestApi {
    pub rand_key: String,
}

#[OpenApi]
impl TestApi {
    fn new() -> Self {
        Self {
            rand_key: format!("{:08x}", rand::random::<u32>()),
        }
    }
    #[oai(path = "/hello", method = "get")]
    async fn create(&self) -> tardis::web::web_resp::TardisApiResult<String> {
        tardis::web::web_resp::TardisResp::ok(self.rand_key.clone())
    }
}

#[allow(dead_code)]
struct DockerEnv {
    nacos_url: String,
    mq_url: String,
    nacos: ContainerAsync<NacosServer>,
    mq: ContainerAsync<RabbitMq>,
    cache: ContainerAsync<Redis>,
}

const NACOS_TAG: &str = "v2.2.3-slim";

async fn initialize_docker_env() -> DockerEnv {
    // init nacos docker
    use tardis::test::test_container::nacos_server::NacosServerMode;
    use tardis::test::test_container::TardisTestContainer;

    // nacos
    let mut nacos = tardis::test::test_container::nacos_server::NacosServer::default();
    nacos
        .nacos_auth_enable(true)
        .nacos_auth_identity_key("nacos".into())
        .nacos_auth_identity_value("nacos".into())
        .nacos_auth_token(tardis::crypto::crypto_base64::TardisCryptoBase64.encode("nacos server for test_config_with_remote"))
        .nacos_auth_token_expire_seconds(10)
        .mode(NacosServerMode::Standalone);
    nacos.tag = NACOS_TAG.to_string();
    let nacos = nacos.start().await.expect("fail to start nacos server");
    let port = nacos.get_host_port_ipv4(8848).await.expect("fail to get nacos port");

    let nacos_url = format!("{schema}://{ip}:{port}/nacos", schema = "http", ip = "127.0.0.1");
    env::set_var("TARDIS_FW.CONF_CENTER.URL", nacos_url.clone());
    nacos.start().await.expect("fail to start nacos server");
    println!("nacos server started at: {}", nacos_url);

    // mq
    let mq = TardisTestContainer::rabbit_custom().await.expect("fail to start rabbitmq");
    let port = mq.get_host_port_ipv4(5672).await.expect("fail to get mq port");
    let mq_url = format!(
        "{schema}://{user}:{pswd}@{ip}:{port}/%2f",
        schema = "amqp",
        user = "guest",
        pswd = "guest",
        ip = "127.0.0.1",
    );
    env::set_var("TARDIS_FW.MQ.URL", mq_url.clone());
    env::set_var("TARDIS_FW.MQ.MODULES.M1.URL", mq_url.clone());
    println!("rabbit-mq started at: {}", mq_url);

    // redis
    let redis = TardisTestContainer::redis_custom().await.expect("fail to start redis");
    let port = redis.get_host_port_ipv4(6379).await.expect("fail to get redis port");
    let redis_url = format!("redis://localhost:{port}/0");
    env::set_var("TARDIS_FW.CACHE.URL", redis_url.clone());

    DockerEnv {
        mq_url,
        nacos_url,
        nacos,
        mq,
        cache: redis,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_config_with_remote() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis::config=debug");
    env::set_var("PROFILE", "remote");

    let docker_env = initialize_docker_env().await;
    TardisFuns::init(Some("tests/config")).await?;
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试_romote_locale");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").level_num, 3);
    TardisFuns::shutdown().await?;

    // load remote config
    let mut client: NacosClient = NacosClient::new_test(&docker_env.nacos_url);
    // get auth
    client.login("nacos", "nacos").await?;
    // going to put test-app-default into remote
    let remote_cfg_default = NacosConfigDescriptor::new("test-app-default", "DEFAULT_GROUP", &Arc::new(Mutex::new(None)));
    let remote_cfg_remote = NacosConfigDescriptor::new("test-app-remote", "DEFAULT_GROUP", &Arc::new(Mutex::new(None)));
    // 1. delete remote config if exists
    let _delete_result = client
        .delete_config(&remote_cfg_default)
        .await
        .map(|e| {
            warn!("delete remote config failed: {}", e);
            false
        })
        .unwrap();
    let _delete_result = client
        .delete_config(&remote_cfg_remote)
        .await
        .map(|e| {
            warn!("delete remote config failed: {}", e);
            false
        })
        .unwrap();
    // 2. publish remote config
    info!("publish remote config: {:?}", remote_cfg_default);
    let config_file = include_str!("./config/remote-config/conf-remote-v1.toml");
    let pub_result = client.publish_config(&remote_cfg_default, &mut config_file.as_bytes()).await.expect("fail to publish remote config");
    assert!(pub_result);
    info!("publish remote config success");
    TardisFuns::shutdown().await?;

    // 3. get remote config
    env::set_var("PROFILE", "remote");
    TardisFuns::init(Some("tests/config")).await?;
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试_romote_uploaded");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").level_num, 4);
    assert_eq!(
        TardisFuns::cs_config::<TestModuleConfig>("m1").db_proj.url,
        "ENC(9EE184E87EA31E6588C08BBC0F7C0E276DE482F7CEE914CBDA05DF619607A24E)"
    );
    // 3.1 test web server
    let api = TestApi::new();
    let key = api.rand_key.clone();
    let server = TardisFuns::web_server();
    server.add_route(api).await;
    server.start().await?;
    let response = TardisFuns::web_client().get_to_str("http://localhost:8080/hello", None).await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());
    assert!(response.body.unwrap().contains(&key));
    // 3.2 test cache
    {
        let cache_client = TardisFuns::cache();
        test_cache(&cache_client).await?;
    }
    // 4. update remote config
    // wait for 5s
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    // upload config
    let config_file = include_str!("./config/remote-config/conf-remote-v2.toml");
    let update_result = client.publish_config(&remote_cfg_default, &mut config_file.as_bytes()).await.expect("fail to update remote config");
    info!("update remote config result: {:?}", update_result);
    // 4.1 wait for polling, and tardis will reboot since the remote config has been updated
    let mut count_down = 30;
    while count_down > 0 {
        if count_down % 5 == 0 {
            info!("wait {}s for polling", count_down);
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        count_down -= 1;
    }
    // 4.2 check if the local config has been updated
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试_romote_uploaded_v2");
    // 5.1 test if web server router is still usable
    {
        let response = TardisFuns::web_client().get_to_str("http://localhost:8080/hello", None).await?;
        assert_eq!(response.code, StatusCode::OK.as_u16());
        // rand key should be same
        assert!(response.body.unwrap().contains(&key));
    }
    // wait for server to start
    // 5.2 test cache
    {
        let cache_client = TardisFuns::cache();
        test_cache(&cache_client).await?;
    }
    // 5.3 test mq
    {
        info!("test mq");
        TardisFuns::mq();
        let mq_client = TardisFuns::mq_by_module("m1");

        mq_client
            .response("test-addr", |(header, msg)| async move {
                println!("response1 {}", msg);
                assert_eq!(header.get("k1").unwrap(), "v1");
                assert_eq!(msg, "测试!");
                Ok(())
            })
            .await?;

        let mut header = HashMap::new();
        header.insert("k1".to_string(), "v1".to_string());

        mq_client.request("test-addr", "测试!".to_string(), &header).await?;
    }

    TardisFuns::shutdown().await?;
    Ok(())
}

async fn test_cache(cache_client: &TardisCacheClient) -> TardisResult<()> {
    info!("test cache");
    cache_client.set("test_key", "测试").await?;
    let str_value = cache_client.get("test_key").await?.unwrap();
    assert_eq!(str_value, "测试");
    Ok(())
}
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
struct TestConfig {
    project_name: String,
    level_num: u8,
    db_proj: DatabaseConfig,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
struct TestModuleConfig {
    db_proj: DatabaseConfig,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
struct DatabaseConfig {
    url: String,
}
