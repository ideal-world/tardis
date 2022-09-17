// https://github.com/mehcode/config-rs

use std::env;

use tardis::basic::result::TardisResult;
use tardis::serde::{Deserialize, Serialize};
use tardis::TardisFuns;

#[tokio::test]
#[ignore]
async fn test_config_with_remote() -> TardisResult<()> {
    env::set_var("RUST_LOG", "debug");

    TardisFuns::init("tests/config").await?;
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").level_num, 2);

    env::set_var("PROFILE", "remote");
    TardisFuns::init("tests/config").await?;
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试_romote_remote");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").level_num, 3);
    assert_eq!(
        TardisFuns::cs_config::<TestModuleConfig>("m1").db_proj.url,
        "ENC(9EE184E87EA31E6588C08BBC0F7C0E276DE482F7CEE914CBDA05DF619607A24E)"
    );

    env::set_var("PROFILE", "remote");
    env::set_var("Tardis_FW.ADV.SALT", "16a80c4aea768c98");
    TardisFuns::init("tests/config").await?;
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试_romote_remote");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").level_num, 3);
    assert_eq!(TardisFuns::cs_config::<TestModuleConfig>("m1").db_proj.url, "postgres://postgres@m1.proj");

    env::set_var("PROFILE", "remote");
    env::set_var("Tardis_FW.ADV.SALT", "16a80c4aea768c98");
    TardisFuns::init("tests/config").await?;
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试_romote_remote");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").level_num, 3);
    assert_eq!(TardisFuns::cs_config::<TestModuleConfig>("m1").db_proj.url, "postgres://postgres@m1.proj");

    env::set_var("PROFILE", "remote");
    env::set_var("Tardis_FW.ADV.SALT", "16a80c4aea768c98");
    env::set_var("Tardis_CS.PROJECT_NAME", "测试_env");
    TardisFuns::init("tests/config").await?;
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
