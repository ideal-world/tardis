use chrono::{DateTime, TimeZone, Utc};
use serde::Deserializer;
use std::fs;
use tardis::basic::dto::TardisContext;
use tardis::basic::field::TrimString;
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

    let file = fs::File::open("tests/test-json-files/text.json")?;
    let json_obj: TestConfig<DatabaseConfig> = TardisFuns::json.reader_to_obj(file)?;
    assert_eq!(json_obj.project_name, "测试");
    assert_eq!(json_obj.level_num, 0);
    assert_eq!(json_obj.db_proj.url, "http://xxx");

    let json_obj = TardisFuns::json.file_to_obj::<TestConfig<DatabaseConfig>, &str>("tests/test-json-files/text.json")?;
    assert_eq!(json_obj.project_name, "测试");
    assert_eq!(json_obj.level_num, 0);
    assert_eq!(json_obj.db_proj.url, "http://xxx");

    let ctx: TardisContext = TardisFuns::json.str_to_obj(r#"{"own_paths":"ss/"}"#)?;
    assert_eq!(ctx.ak, "");
    assert_eq!(ctx.own_paths, "ss/");

    assert!(ctx.ext.read()?.is_empty());

    ctx.add_ext("task_id1", "测试")?;
    ctx.add_ext("task_id2", "dddddd")?;
    assert_eq!(ctx.get_ext("task_id1")?, Some("测试".to_string()));
    assert_eq!(ctx.get_ext("task_id2")?, Some("dddddd".to_string()));
    ctx.remove_ext("task_id2")?;
    assert_eq!(ctx.get_ext("task_id2")?, None);
    let ctx = TardisFuns::json.obj_to_string(&ctx)?;
    assert_eq!(ctx, r#"{"own_paths":"ss/","ak":"","owner":"","roles":[],"groups":[]}"#);

    let req_dto = UserAddReq {
        name: TrimString("星航大大".to_lowercase()),
        pwd: "123456".to_string(),
        age: 10,
        roles: vec![
            UserRoleReq {
                code: TrimString("admin".to_lowercase()),
                name: "管理员".to_string(),
            },
            UserRoleReq {
                code: TrimString("user".to_lowercase()),
                name: "用户".to_string(),
            },
        ],
        org: Some(UserOrgReq {
            code: TrimString("org1".to_lowercase()),
            name: "组织1".to_string(),
        }),
        status: Some(true),
        front_field: "front_field".to_string(),
    };

    let user_info: UserInfo = TardisFuns::json.copy(&req_dto)?;
    assert_eq!(user_info.id, "idxx");
    assert_eq!(user_info.name, "星航大大");
    assert_eq!(user_info.password, "123456");
    assert_eq!(user_info.age, 10);
    assert_eq!(user_info.roles.len(), 2);
    assert_eq!(user_info.roles[0].code, "admin");
    assert_eq!(user_info.roles[0].name, "管理员");
    assert_eq!(user_info.roles[1].code, "user");
    assert_eq!(user_info.roles[1].name, "用户");
    assert_eq!(user_info.org.as_ref().unwrap().code, "org1");
    assert_eq!(user_info.org.as_ref().unwrap().name, "组织1");
    assert_eq!(user_info.status, true);
    assert!(user_info.create_time.timestamp() <= Utc::now().timestamp());

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

#[derive(Serialize, Deserialize)]
struct UserAddReq {
    pub name: TrimString,
    #[serde(rename(serialize = "password"))]
    pub pwd: String,
    pub age: u8,
    pub roles: Vec<UserRoleReq>,
    pub org: Option<UserOrgReq>,
    pub status: Option<bool>,
    // no need to copy
    pub front_field: String,
}

#[derive(Serialize, Deserialize)]
struct UserRoleReq {
    pub code: TrimString,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
struct UserOrgReq {
    pub code: TrimString,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
struct UserInfo {
    #[serde(default = "id_default")]
    pub id: String,
    pub name: String,
    pub password: String,
    pub age: u8,
    pub roles: Vec<UserRoleInfo>,
    pub org: Option<UserOrgInfo>,
    pub status: bool,
    #[serde(default = "create_time_default")]
    #[serde(skip_serializing)]
    #[serde(deserialize_with = "deserialize_time")]
    pub create_time: DateTime<Utc>,
}

fn id_default() -> String {
    "idxx".to_string()
}

fn create_time_default() -> DateTime<Utc> {
    Utc::now()
}

pub fn deserialize_time<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Utc.datetime_from_str(&s, "%Y-%m-%d %H:%M:%S").map_err(serde::de::Error::custom)
}

#[derive(Serialize, Deserialize)]
struct UserRoleInfo {
    pub code: String,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
struct UserOrgInfo {
    pub code: String,
    pub name: String,
}
