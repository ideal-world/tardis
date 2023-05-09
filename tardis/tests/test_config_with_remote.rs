// https://github.com/mehcode/config-rs

use std::env;

use poem_openapi_derive::OpenApi;
use reqwest::StatusCode;
use tardis::basic::result::TardisResult;
use tardis::config::config_dto::TardisConfig;
use tardis::config::config_nacos::nacos_client::{NacosClient, NacosConfigDescriptor};
use tardis::serde::{Deserialize, Serialize};
use tardis::TardisFuns;

use std::sync::Arc;
use tokio::sync::Mutex;

struct TestApi {
    pub randown_key: String,
}

#[OpenApi]
impl TestApi {
    fn new() -> Self {
        Self {
            randown_key: format!("{:08x}", rand::random::<u32>()),
        }
    }
    #[oai(path = "/hello", method = "get")]
    async fn create(&self) -> tardis::web::web_resp::TardisApiResult<String> {
        tardis::web::web_resp::TardisResp::ok(self.randown_key.clone())
    }
}

#[tokio::test]
// #[ignore = "need a nacos server or a nacos test-container"]
async fn test_config_with_remote() -> TardisResult<()> {
    use std::fs::*;
    env::set_var("RUST_LOG", "info,tardis=debug");
    env::set_var("PROFILE", "default");
    // for debug only
    // env::set_current_dir("./tardis").unwrap();
    TardisFuns::init(Some("tests/config")).await?;
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").level_num, 2);

    // load remote config
    env::set_var("PROFILE", "remote");
    TardisFuns::init(Some("tests/config")).await?;
    let fw_config = TardisFuns::fw_config().conf_center.as_ref().expect("fail to get conf_center config");
    // this client is for test only
    let mut client: NacosClient = NacosClient::new(&fw_config.url);
    // get auth
    client.login(&fw_config.username, &fw_config.password).await?;
    // going to put test-app-default into remote
    let remote_cfg_default = NacosConfigDescriptor::new("test-app-default", "DEFAULT_GROUP", &Arc::new(Mutex::new(None)));
    let remote_cfg_remote = NacosConfigDescriptor::new("test-app-remote", "DEFAULT_GROUP", &Arc::new(Mutex::new(None)));
    // 1. delete remote config if exists
    let _delete_result = client
        .delete_config(&remote_cfg_default)
        .await
        .and_then(|e| {
            log::warn!("delete remote config failed: {}", e);
            Ok(false)
        })
        .unwrap();
    let _delete_result = client
        .delete_config(&remote_cfg_remote)
        .await
        .and_then(|e| {
            log::warn!("delete remote config failed: {}", e);
            Ok(false)
        })
        .unwrap();
    // 2. publish remote config
    log::info!("publish remote config: {:?}", remote_cfg_default);
    let pub_result = client
        .publish_config(
            &remote_cfg_default,
            &mut File::open("./tests/config/remote-config/conf-remote-v1.toml").expect("fail to open conf-remote-v1"),
        )
        .await
        .expect("fail to publish remote config");
    assert!(pub_result);
    log::info!("publish remote config success");

    // 3. get remote config
    TardisFuns::shutdown().await?;
    env::set_var("PROFILE", "remote");
    TardisFuns::init(Some("tests/config")).await?;
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试_romote_uploaded");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").level_num, 3);
    assert_eq!(
        TardisFuns::cs_config::<TestModuleConfig>("m1").db_proj.url,
        "ENC(9EE184E87EA31E6588C08BBC0F7C0E276DE482F7CEE914CBDA05DF619607A24E)"
    );
    // 3.1 test web server
    let api = TestApi::new();
    let key = api.randown_key.clone();
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
    let update_result = client
        .publish_config(
            &remote_cfg_default,
            &mut File::open("./tests/config/remote-config/conf-remote-v2.toml").expect("fail to open conf-remote-v2"),
        )
        .await
        .expect("fail to update remote config");
    log::info!("update remote config result: {:?}", update_result);
    // 4.1 wait for polling, and tardis will reboot since the remote config has been updated
    let mut count_down = 15;
    while count_down > 0 {
        if count_down % 5 == 0 {
            log::info!("wait {}s for polling", count_down);
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        count_down -= 1;
    }
    // 4.2 check if the local config has been updated
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试_romote_uploaded_v2");
    // 5.1 test web client
    let result = TardisFuns::web_client().get_to_str("https://postman-echo.com/get", None).await?;
    assert_eq!(result.code, StatusCode::OK.as_u16());
    // 5.2 test web server
    let api = TestApi::new();
    let key = api.randown_key.clone();
    // let server = TardisFuns::web_server();
    // server.add_route(api).await;
    // tokio::spawn(server.start());
    // wait for server to start
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    // wait for server to start
    let response = TardisFuns::web_client().get_to_str("http://localhost:8081/hello", None).await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());
    assert!(response.body.unwrap().contains(&key));
    // 5.3 test mq
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
