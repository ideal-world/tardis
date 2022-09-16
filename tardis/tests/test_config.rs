// https://github.com/mehcode/config-rs

use regex::{Captures, Regex};
use std::env;

use tardis::basic::result::TardisResult;
use tardis::serde::{Deserialize, Serialize};
use tardis::TardisFuns;

#[tokio::test]
async fn test_config() -> TardisResult<()> {
    env::set_var("PROFILE", "test");
    TardisFuns::init("tests/config").await?;
    env::set_var("Tardis_FW.ADV.SALT", "16a80c4aea768c98");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").project_name, "测试");
    assert!(!TardisFuns::fw_config().db.enabled);
    assert_eq!(TardisFuns::fw_config().db.url, "postgres://postgres@test");
    assert_eq!(TardisFuns::cs_config::<TestConfig>("").db_proj.url, "postgres://postgres@test.proj");
    assert_eq!(TardisFuns::fw_config().app.name, "APP1");

    env::set_var("PROFILE", "prod");
    TardisFuns::init("tests/config").await?;
    env::set_var("Tardis_FW.ADV.SALT", "16a80c4aea768c98");
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

#[tokio::test]
async fn test_crypto_config() -> TardisResult<()> {
    let re = Regex::new(r"(?P<ENC>ENC\([A-Za-z0-9+/]*\))").unwrap();
    let before = r#"{"fw":{"app":{"name":"Hi{x}(a)"},"ak":"ENC(32ns9+s2/3df2v343)"},"sk":"ENC(4ewk2fsmd2)"}"#;
    let after = re.replace_all(before, |captures: &Captures| {
        let some = captures.get(1).map_or("", |m| m.as_str()).to_string();
        println!("{}", some);
        "1234"
    });
    assert_eq!(after, r#"{"fw":{"app":{"name":"Hi{x}(a)"},"ak":"1234"},"sk":"1234"}"#);

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
