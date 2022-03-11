use tardis::basic::result::TardisResult;
use tardis::serde::{Deserialize, Serialize};
use tardis::TardisFuns;

#[tokio::test]
async fn test_basic_json() -> TardisResult<()> {
    let test_config = TestConfig {
        project_name: "测试".to_string(),
        level_num: 0,
        db_proj: DatabaseConfig { url: "http://xxx".to_string() },
    };

    let json_str = TardisFuns::json.obj_to_string(&test_config)?;
    assert_eq!(json_str, r#"{"project_name":"测试","level_num":0,"db_proj":{"url":"http://xxx"}}"#);

    let json_obj = TardisFuns::json.str_to_obj::<TestConfig<DatabaseConfig>>(&json_str)?;
    assert_eq!(json_obj.project_name, "测试");
    assert_eq!(json_obj.level_num, 0);
    assert_eq!(json_obj.db_proj.url, "http://xxx");

    let json_value = TardisFuns::json.str_to_json(&json_str)?;
    assert_eq!(json_value["project_name"], "测试");
    assert_eq!(json_value["level_num"], 0);
    assert_eq!(json_value["db_proj"]["url"], "http://xxx");

    let json_value = TardisFuns::json.obj_to_json(&json_obj)?;
    assert_eq!(json_value["project_name"], "测试");
    assert_eq!(json_value["level_num"], 0);
    assert_eq!(json_value["db_proj"]["url"], "http://xxx");

    let json_obj = TardisFuns::json.json_to_obj::<TestConfig<DatabaseConfig>>(json_value)?;
    assert_eq!(json_obj.project_name, "测试");
    assert_eq!(json_obj.level_num, 0);
    assert_eq!(json_obj.db_proj.url, "http://xxx");

    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct TestConfig<T> {
    project_name: String,
    level_num: u8,
    db_proj: T,
}

impl<T: Default> Default for TestConfig<T> {
    fn default() -> Self {
        TestConfig {
            project_name: "".to_string(),
            level_num: 0,
            db_proj: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct DatabaseConfig {
    url: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig { url: "".to_string() }
    }
}
