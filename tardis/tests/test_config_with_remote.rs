// https://github.com/mehcode/config-rs

use std::env;

use tardis::basic::result::TardisResult;
use tardis::config::config_dto::TardisConfig;
use tardis::config::config_nacos::nacos_client::{NacosClient, NacosConfigDescriptor};
use tardis::serde::{Deserialize, Serialize};
use tardis::TardisFuns;

use std::sync::Arc;
use tokio::sync::Mutex;
#[tokio::test]
// #[ignore]
async fn test_config_with_remote() -> TardisResult<()> {
    use std::fs::*;
    env::set_var("RUST_LOG", "debug");
    env::set_var("PROFILE", "default");

    env::set_current_dir("./tardis").unwrap();
    TardisFuns::init(Some("tests/config")).await?;
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").level_num, 2);

    // load remote config
    env::set_var("PROFILE", "remote");
    TardisFuns::init(Some("tests/config")).await?;
    let fw_config = TardisFuns::fw_config().conf_center.as_ref().expect("fail to get conf_center config");
    let mut client: NacosClient = NacosClient::new(&fw_config.url);
    client.login(&fw_config.username, &fw_config.password).await?;
    let remote_cfg = NacosConfigDescriptor::new("conf-remote", "DEFAULT_GROUP", &Arc::new(Mutex::new(None)));
    // delete remote config if exists
    let delete_result = client.delete_config(&remote_cfg).await;
    match delete_result {
        Ok(_) => {}
        Err(e) => {
            log::warn!("delete remote config failed: {}", e);
        }
    }
    // publish remote config
    log::info!("publish remote config: {:?}", remote_cfg);
    let pub_result =
        client.publish_config(&remote_cfg, &mut File::open("./tests/config/conf-remote.toml").expect("fail to open conf-remote.toml")).await.expect("fail to publish remote config");
    assert!(pub_result);
    log::info!("publish remote config success");

    // get remote config
    env::set_var("PROFILE", "remote");
    TardisFuns::init(Some("tests/config")).await?;
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试_romote_remote");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").level_num, 3);
    assert_eq!(
        TardisFuns::cs_config::<TestModuleConfig>("m1").db_proj.url,
        "ENC(9EE184E87EA31E6588C08BBC0F7C0E276DE482F7CEE914CBDA05DF619607A24E)"
    );

    // update remote config
    let update_result = client
        .publish_config(&remote_cfg, &mut File::open("./tests/config/conf-remote-updated.toml").expect("fail to open conf-remote-updated.toml"))
        .await
        .expect("fail to update remote config");
    log::info!("update remote config result: {:?}", update_result);
    let mut count_down = 30;
    while count_down > 0 {
        if count_down %5 == 0 {
            log::info!("wait {}s for polling", count_down);
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        count_down -= 1;
    }
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试_romote_remote_v2");


    env::set_var("PROFILE", "remote");
    env::set_var("Tardis_FW.ADV.SALT", "16a80c4aea768c98");
    TardisFuns::init(Some("tests/config")).await?;
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试_romote_remote");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").level_num, 3);
    assert_eq!(TardisFuns::cs_config::<TestModuleConfig>("m1").db_proj.url, "postgres://postgres@m1.proj");

    env::set_var("PROFILE", "remote");
    env::set_var("Tardis_FW.ADV.SALT", "16a80c4aea768c98");
    TardisFuns::init(Some("tests/config")).await?;
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试_romote_remote");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").level_num, 3);
    assert_eq!(TardisFuns::cs_config::<TestModuleConfig>("m1").db_proj.url, "postgres://postgres@m1.proj");

    env::set_var("PROFILE", "remote");
    env::set_var("Tardis_FW.ADV.SALT", "16a80c4aea768c98");
    env::set_var("Tardis_CS.PROJECT_NAME", "测试_env");
    TardisFuns::init(Some("tests/config")).await?;
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试_env");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").level_num, 3);
    assert_eq!(TardisFuns::cs_config::<TestModuleConfig>("m1").db_proj.url, "postgres://postgres@m1.proj");

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
struct TestModuleConfig {
    db_proj: DatabaseConfig,
}

impl Default for TestModuleConfig {
    fn default() -> Self {
        TestModuleConfig {
            db_proj: DatabaseConfig::default(),
        }
    }
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
