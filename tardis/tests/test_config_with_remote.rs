// https://github.com/mehcode/config-rs

use std::collections::HashMap;
use std::env;

use poem_openapi_derive::OpenApi;
use reqwest::StatusCode;
use tardis::basic::result::TardisResult;
use tardis::config::config_nacos::nacos_client::{NacosClient, NacosConfigDescriptor};
use tardis::serde::{Deserialize, Serialize};

use tardis::test::test_container::nacos_server::NacosServer;
use tardis::TardisFuns;
use testcontainers::clients::Cli;
use testcontainers::images::generic::GenericImage;
use testcontainers::Container;
use tracing::{info, warn};

use std::sync::Arc;
use tokio::sync::Mutex;

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
struct DockerEnv<'d> {
    nacos_url: String,
    mq_url: String,
    nacos: Container<'d, NacosServer>,
    mq: Container<'d, GenericImage>,
}

fn initialize_docker_env(cli: &Cli) -> DockerEnv {
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
    nacos.tag = "v2.1.1-slim".to_string();
    let nacos = cli.run(nacos);
    let nacos_url = format!("{schema}://{ip}:{port}/nacos", schema = "http", ip = nacos.get_bridge_ip_address(), port = 8848);
    env::set_var("TARDIS_FW.CONF_CENTER.URL", nacos_url.clone());
    nacos.start();
    println!("nacos server started at: {}", nacos_url);

    // mq

    let mq = TardisTestContainer::rabbit_custom(cli);
    let mq_url = format!(
        "{schema}://{user}:{pswd}@{ip}:{port}/%2f",
        schema = "amqp",
        user = "guest",
        pswd = "guest",
        ip = mq.get_bridge_ip_address(),
        port = 5672
    );
    env::set_var("TARDIS_FW.MQ.URL", mq_url.clone());
    env::set_var("TARDIS_FW.MQ.MODULES.M1.URL", mq_url.clone());
    println!("rabbit-mq started at: {}", mq_url);

    DockerEnv { mq_url, nacos_url, nacos, mq }
}

#[tokio::test]
async fn test_config_with_remote() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis::config=debug");
    env::set_var("PROFILE", "remote");

    let docker = testcontainers::clients::Cli::docker();
    let docker_env = initialize_docker_env(&docker);
    // let nacos_url = format!("{schema}://{ip}:{port}/nacos", schema = "http", ip = "0.0.0.0", port = 8848);
    TardisFuns::init(Some("tests/config")).await?;
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试_romote_locale");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").level_num, 3);
    TardisFuns::shutdown().await?;

    // load remote config
    let mut client: NacosClient = unsafe { NacosClient::new_test(&docker_env.nacos_url) };
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
    tokio::spawn(server.start());
    // wait for server to start
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    // wait for server to start
    let response = TardisFuns::web_client().get_to_str("http://localhost:8080/hello", None).await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());
    assert!(response.body.unwrap().contains(&key));
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
    // 5.1 test web client
    {
        // let result = TardisFuns::web_client().get_to_str("https://postman-echo.com/get", None).await?;
        // assert_eq!(result.code, StatusCode::OK.as_u16());
    }
    // wait for server to start
    // 5.2 test mq
    {
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
        // if close cilent here, shutdown we called later will encounter an error
        // mq_client.close().await?;
    }

    TardisFuns::shutdown().await?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct TestConfig {
    project_name: String,
    level_num: u8,
    db_proj: DatabaseConfig,
}

impl Default for TestConfig {
    fn default() -> Self {
        TestConfig {
            project_name: "".to_string(),
            level_num: 0,
            db_proj: DatabaseConfig::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
struct TestModuleConfig {
    db_proj: DatabaseConfig,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct DatabaseConfig {
    url: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig { url: "".to_string() }
    }
}
