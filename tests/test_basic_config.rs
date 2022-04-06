// https://github.com/mehcode/config-rs

use std::env;

use tardis::basic::result::TardisResult;
use tardis::serde::{Deserialize, Serialize};
use tardis::TardisFuns;

#[tokio::test]
async fn test_basic_config() -> TardisResult<()> {
    env::set_var("PROFILE", "test");
    TardisFuns::init("tests/config").await?;
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试");
    assert!(!TardisFuns::fw_config().db.enabled);
    assert_eq!(TardisFuns::fw_config().db.url, "postgres://postgres@test");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").db_proj.url, "postgres://postgres@test.proj");
    assert_eq!(TardisFuns::fw_config().app.name, "APP1");

    env::set_var("PROFILE", "prod");
    TardisFuns::init("tests/config").await?;
    assert_eq!(TardisFuns::fw_config().db.url, "postgres://postgres@prod");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").db_proj.url, "postgres://postgres@prod.proj");
    assert_eq!(TardisFuns::fw_config().app.name, "Tardis Application");
    assert_eq!(TardisFuns::cs_config::<TestModuleConfig>("m1").db_proj.url, "postgres://postgres@m1.proj");

    // cli example: env Tardis_DB.URL=test Tardis_app.name=xx ./xxx
    env::set_var("Tardis_FW.DB.URL", "test");
    TardisFuns::init("tests/config").await?;
    assert_eq!(TardisFuns::fw_config().db.url, "test");
    assert_eq!(TardisFuns::fw_config().app.name, "Tardis Application");

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
