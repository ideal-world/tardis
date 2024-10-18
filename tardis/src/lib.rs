#![doc= include_str!("../README.md")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/ideal-world/tardis/main/logo.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(clippy::unwrap_used, clippy::undocumented_unsafe_blocks, clippy::dbg_macro)]
// #![warn(clippy::indexing_slicing)]

extern crate core;
#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;

use std::{any::Any, sync::Arc};

#[cfg(feature = "future")]
pub use async_stream;
#[cfg(feature = "future")]
pub use async_trait;
pub use chrono;
pub use derive_more;
#[cfg(feature = "future")]
pub use futures;
#[cfg(feature = "future")]
pub use futures_util;
pub use lru;
#[cfg(feature = "web-server-grpc")]
pub use poem_grpc;
pub use rand;
pub use regex;
pub use serde;
use serde::Deserialize;
pub use serde_json;

#[cfg(feature = "test")]
pub use testcontainers;
pub use tokio;
pub use tracing as log;
// we still need to pub use tracing for some macros
// in tracing relies on `$crate` witch infers `tracing`.
use basic::error::TardisErrorWithExt;
use basic::result::TardisResult;
use basic::tracing::TardisTracing;
pub use paste;
#[cfg(feature = "tardis-macros")]
#[cfg(any(feature = "reldb-postgres", feature = "reldb-mysql", feature = "reldb-sqlite"))]
pub use tardis_macros::{TardisCreateEntity, TardisCreateIndex, TardisCreateTable, TardisEmptyBehavior, TardisEmptyRelation};
pub use tracing;
pub use url;

use crate::basic::field::TardisField;
use crate::basic::json::TardisJson;
use crate::basic::uri::TardisUri;
#[cfg(feature = "cache")]
use crate::cache::cache_client::TardisCacheClient;
use crate::config::config_dto::{FrameworkConfig, TardisConfig};
#[cfg(feature = "reldb-core")]
use crate::db::domain::tardis_db_config::TardisDataDict;
#[cfg(feature = "reldb-core")]
use crate::db::reldb_client::TardisRelDBClient;
#[cfg(feature = "mail")]
use crate::mail::mail_client::TardisMailClient;
#[cfg(feature = "mq")]
use crate::mq::mq_client::TardisMQClient;
#[cfg(feature = "os")]
use crate::os::os_client::TardisOSClient;
#[cfg(feature = "web-client")]
use crate::search::search_client::TardisSearchClient;
use crate::utils::*;
#[cfg(feature = "web-client")]
use crate::web::web_client::TardisWebClient;
#[cfg(feature = "web-server")]
use crate::web::web_server::TardisWebServer;

/// The operational portal for Tardis core features / Tardis核心功能的操作入口
///
/// # Initialization / 初始化
///
/// ## Define project-level configuration object / 定义项目级配置对象
///
/// ```ignore
/// use serde::{Serialize,Deserialize};
/// #[derive(Debug, Serialize, Deserialize)]
/// #[serde(default)]
/// struct ExampleConfig {
///     conf1: String,
///     conf2: String,
/// }
/// impl Default for ExampleConfig {
///     fn default() -> Self {
///         ExampleConfig {
///             conf1: String::new(),
///             conf2: String::new(),
///         }
///     }
/// }
/// ```
/// ## Define configuration file / 定义配置文件
///
/// The name of the configuration file is `conf-<profile>.toml`, where `conf-default.toml` is the
/// base configuration and you can define a file such as `conf-test.toml` to override the base configuration.
///
/// 配置文件名称为 `conf-<profile>.toml`，其中 `conf-default.toml` 为基础配置，可定义诸如 `conf-test.toml` 用于覆盖基础配置.
///
/// The current configuration environment can be specified via ```env::set_var("PROFILE", "test")``.
///
/// 可通过 ```env::set_var("PROFILE", "test")``` 来指定当前的配置环境.
///
/// The format of the configuration file is.
///
/// 配置文件的格式为：
///
/// ```txt
/// <project specific configuration set> / <项目特殊配置集合>
///
/// <Tardis configuration set> / <Tardis配置集合>
/// ```
///
/// The project-specific configuration set is the format defined in the first step, for the
/// Tardis configuration set see [`FrameworkConfig`](basic::config::FrameworkConfig) .
///
/// 项目特殊的配置集合即为第一步定义的格式，Tardis配置集合见 [`FrameworkConfig`](basic::config::FrameworkConfig) .
///
/// . An example configuration / 一个示例配置
/// ```toml
/// conf1 = "some value"
/// conf2 = "some value"
///
/// [db]
/// enabled = false
/// port = 8089
///
/// [web_server]
/// enabled = false
///
/// [cache]
/// enabled = false
///
/// [mq]
/// enabled = false
/// ```
///
/// ## Perform initialization operation / 执行初始化操作
///
/// ```ignore
/// use tardis::TardisFuns;
/// TardisFuns::init("proj/config").await?;
/// ```
///
/// More examples of initialization can be found in: `test_basic_config.rs` .
///
/// 更多初始化的示例可参考： `test_basic_config.rs` .
///
/// # 使用
///
/// ```ignore
/// use tardis::TardisFuns;
/// TardisFuns::ws_config();  
/// TardisFuns::fw_config();
/// TardisFuns::field;
/// TardisFuns::json;
/// TardisFuns::uri;  
/// TardisFuns::crypto;   
/// TardisFuns::reldb();    
/// TardisFuns::web_server();  
/// TardisFuns::web_client();  
/// TardisFuns::cache();
/// TardisFuns::mq();
/// ```
#[derive(Default)]
pub struct TardisFuns {
    custom_config: TardisComponentMap<CachedJsonValue>,
    framework_config: TardisComponent<FrameworkConfig>,
    components: basic::component::ComponentStore,
    pub(crate) tracing: TardisComponent<TardisTracing>,
    #[cfg(feature = "reldb-core")]
    reldb: TardisComponentMap<TardisRelDBClient>,
    #[cfg(feature = "web-server")]
    web_server: TardisComponent<TardisWebServer>,
    #[cfg(feature = "web-client")]
    web_client: TardisComponentMap<TardisWebClient>,
    #[cfg(feature = "cache")]
    cache: TardisComponentMap<TardisCacheClient>,
    #[cfg(feature = "mq")]
    mq: TardisComponentMap<TardisMQClient>,
    #[cfg(feature = "web-client")]
    search: TardisComponentMap<TardisSearchClient>,
    #[cfg(feature = "mail")]
    mail: TardisComponentMap<TardisMailClient>,
    #[cfg(feature = "os")]
    os: TardisComponentMap<TardisOSClient>,
}

tardis_static! {
    tardis_instance: TardisFuns;
}

impl TardisFuns {}

#[allow(unsafe_code)]
impl TardisFuns {
    /// Get the configuration file from the specified path and initialize it / 从指定的路径中获取配置文件并初始化
    ///
    /// # Arguments
    ///
    /// * `relative_path` - the directory where the configuration file is located, without the
    ///     configuration file name / 配置文件所在目录，不包含配置文件名
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::env;
    /// use tardis::TardisFuns;
    /// env::set_var("PROFILE", "test");
    /// TardisFuns::init("proj/config").await;
    /// ```
    ///
    /// # Errors
    /// Config file not found or invalid config file format
    pub async fn init(relative_path: Option<&str>) -> TardisResult<()> {
        TardisTracing::init_default();
        let config = TardisConfig::init(relative_path).await?;
        TardisFuns::init_conf(config).await
    }

    /// Initialize log / 初始化日志
    ///
    /// The [init](Self::init) function will automatically call this function
    ///
    /// [init](Self::init) 函数时会自动调用此函数
    pub fn init_log() {
        TardisTracing::init_default();
    }

    /// Component Store / 组件仓库
    pub fn store() -> &'static crate::basic::component::ComponentStore {
        &tardis_instance().components
    }

    /// Initialized by the configuration object / 通过配置对象初始化
    ///
    /// This function does not require a configuration file, it uses the rust object instance to
    /// initialize directly.
    ///
    /// 本函数不需要配置文件，直接使用rust对象实例初始化.
    ///
    /// # Arguments
    ///
    /// * `conf` - configuration object instance / 配置对象实例
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use tardis::basic::config::{CacheConfig, FrameworkConfig, TardisConfig};
    /// use tardis::TardisFuns;
    /// let cache_module_config = CacheModuleConfig::builder().url(url).build();
    /// TardisFuns::init_conf(
    ///     TardisConfig::builder()
    ///         .fw(FrameworkConfig::builder()
    ///             .cache(
    ///                 CacheConfig::builder()
    ///                 .default(
    ///                     cache_module_config.clone()
    ///                 ).modules([
    ///                     ("m1".to_string(), cache_module_config.clone())
    ///                 ]).build())
    ///             .build())
    ///         .build(),
    /// )
    /// .await?;
    /// ```
    pub async fn init_conf(conf: TardisConfig) -> TardisResult<()> {
        let custom_config = conf.cs.iter().map(|(k, v)| (k.clone(), CachedJsonValue::new(v.clone()))).collect::<HashMap<_, _>>();
        tardis_instance().custom_config.replace_inner(custom_config);
        tardis_instance().framework_config.set(conf.fw);
        #[allow(unused_variables)]
        let fw_conf = TardisFuns::fw_config();
        if let Some(log_config) = &fw_conf.log {
            tardis_instance().tracing.get().update_config(log_config)?;
        }
        #[cfg(feature = "reldb-core")]
        {
            if let Some(db_config) = &fw_conf.db {
                tardis_instance().reldb.init_by(db_config).await?;
            }
        }
        #[cfg(feature = "web-server")]
        {
            if let Some(_web_server_config) = &fw_conf.web_server {
                tracing::info!("initialize web server");
                let web_server = TardisWebServer::init_by_conf(&fw_conf)?;
                // take out previous webserver first, because TARDIS_INST is not send and can't live cross an `await` point
                let inherit = tardis_instance().web_server.get();
                if inherit.is_running().await {
                    // 1. should always shutdown first
                    let _ = inherit.shutdown().await;
                    // 2. load initializers
                    web_server.load_initializer(inherit).await;
                    // 3. restart webserver
                    web_server.start().await?;
                }
                tardis_instance().web_server.set(web_server)
            }
        }
        #[cfg(feature = "web-client")]
        {
            if let Some(web_client_config) = &fw_conf.web_client {
                tardis_instance().web_client.init_by(web_client_config).await?;
            }
        }
        #[cfg(feature = "cache")]
        {
            if let Some(cache_config) = &fw_conf.cache {
                tardis_instance().cache.init_by(cache_config).await?;
            }
        }
        #[cfg(feature = "mq")]
        {
            if let Some(mq_config) = &fw_conf.mq {
                tardis_instance().mq.init_by(mq_config).await?;
            }
        }
        #[cfg(feature = "web-client")]
        {
            if let Some(search_config) = &fw_conf.search {
                tardis_instance().search.init_by(search_config).await?;
            }
        }
        #[cfg(feature = "mail")]
        {
            if let Some(mail_config) = &fw_conf.mail {
                tardis_instance().mail.init_by(mail_config).await?;
            }
        }
        #[cfg(feature = "os")]
        {
            if let Some(os_config) = &fw_conf.os {
                tardis_instance().os.init_by(os_config).await?;
            }
        }
        Ok(())
    }

    /// Build single Module by the specified code / 通过指定的 code 构造单模块实例
    ///
    /// # Arguments
    ///
    /// * `code` - The specified code in the custom configuration / 配置中指定的模块名
    /// * `lang` - The specified language (if project support multi-language) / 指定语种（如果项目支持多语种）
    ///
    /// # Examples
    ///
    ///  ```ignore
    /// use tardis::TardisFuns;
    /// let funs = TardisFuns::inst("product", None);
    /// ```
    pub fn inst(code: impl Into<String>, lang: Option<String>) -> TardisFunsInst {
        TardisFunsInst::new(code.into(), lang)
    }

    /// Build single module with db connect by the specified code / 通过指定的 code 构造携带数据库连接的单模块实例
    ///
    /// # Arguments
    ///
    /// * `code` - The specified code in the custom configuration / 配置中指定的模块名
    /// * `lang` - The specified language (if project support multi-language) / 指定语种（如果项目支持多语种）
    ///
    /// # Examples
    ///
    ///  ```ignore
    /// use tardis::TardisFuns;
    /// let funs = TardisFuns::inst_with_db_conn("product", None);
    /// ```
    #[cfg(feature = "reldb-core")]
    pub fn inst_with_db_conn(code: impl Into<String>, lang: Option<String>) -> TardisFunsInst {
        TardisFunsInst::new_with_db_conn(code.into(), lang)
    }

    /// Clone the current config into [`TardisConfig`], this may be used to reload config.
    ///
    pub fn clone_config() -> TardisConfig {
        let cs = tardis_instance().custom_config.read().iter().map(|(k, v)| (k.clone(), v.raw().clone())).collect::<HashMap<String, serde_json::Value>>();
        let fw = tardis_instance().framework_config.get().as_ref().clone();
        TardisConfig { cs, fw }
    }

    /// Get the custom configuration object / 获取自定义配置对象
    ///
    /// # Panic
    /// If the configuration object does not exist, this will fallback to the default config.
    /// Though, if the default config cannot be deserialized as `T`, this will panic.
    pub fn cs_config<T: 'static + for<'a> Deserialize<'a> + Any + Send + Sync>(code: &str) -> Arc<T> {
        let code = code.to_lowercase();
        let code = code.as_str();
        let conf = &tardis_instance().custom_config;
        if let Some(t) = conf.get(code) {
            let t = t.get::<T>().unwrap_or_else(|e| panic!("[Tardis.Config] Custom Config [{code}] type conversion error {e}"));
            return t;
        }
        if !code.is_empty() {
            return Self::cs_config("");
        }
        panic!("[Tardis.Config] Custom Config [{code}] or [] doesn't exist");
    }

    /// Get default language in the custom configuration / 从自定义配置中获取默认语言
    pub fn default_lang() -> Option<String> {
        tardis_instance().framework_config.get().app.default_lang.clone()
    }

    /// Get the Tardis configuration object / 获取Tardis配置对象
    pub fn fw_config() -> Arc<FrameworkConfig> {
        tardis_instance().framework_config.get()
    }

    pub fn fw_config_opt() -> Option<Arc<FrameworkConfig>> {
        Some(tardis_instance().framework_config.get())
    }

    /// Using the field feature / 使用字段功能
    ///
    /// # Examples
    /// ```ignore
    ///
    /// use tardis::TardisFuns;
    /// TardisFuns::field.is_phone("18657120202");
    ///
    /// TardisFuns::field.incr_by_base62("abcd1");
    /// ```
    #[allow(non_upper_case_globals)]
    pub const field: TardisField = TardisField {};

    /// Using the json feature / 使用Json功能
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// let test_config = TestConfig {
    ///         project_name: "测试".to_string(),
    ///         level_num: 0,
    ///         db_proj: DatabaseConfig { url: "http://xxx".to_string() },
    ///     };
    ///
    /// // Rust object to Json string / Rust对象转成Json字符串
    /// let json_str = TardisFuns::json.obj_to_string(&test_config).unwrap();
    ///
    /// // Json string to Rust Object / Json字符串转成Rust对象
    /// TardisFuns::json.str_to_obj::<TestConfig<DatabaseConfig>>(&json_str).unwrap();
    /// ```
    #[allow(non_upper_case_globals)]
    pub const json: TardisJson = TardisJson {};

    /// Using the uri feature / 使用Url功能
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// // Query sort
    /// assert_eq!(TardisFuns::uri.format("api://a1.t1/e1?q2=2&q1=1&q3=3").unwrap(), "api://a1.t1/e1?q1=1&q2=2&q3=3");
    /// ```
    #[allow(non_upper_case_globals)]
    pub const uri: TardisUri = TardisUri {};

    /// Use of encryption/decryption/digest features / 使用加解密/摘要功能
    ///
    /// Supported algorithms: base64/md5/sha/mac/aes/rsa/sm2/sm3/sm4.
    ///
    /// 支持的算法： base64/md5/sha/hmac/aes/rsa/sm2/sm3/sm4.
    ///
    /// This feature needs to be enabled #[cfg(feature = "crypto")] and #[cfg(feature = "crypto-with-sm")] .
    ///
    /// 本功能需要启用 #[cfg(feature = "crypto")] 和 #[cfg(feature = "crypto-with-sm")] .
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// TardisFuns::crypto.base64.decode("测试");
    /// TardisFuns::crypto.digest.sha256("测试");
    /// TardisFuns::crypto.digest.sm3("测试");
    /// ```
    #[allow(non_upper_case_globals)]
    #[cfg(feature = "crypto")]
    pub const crypto: crypto::crypto_main::TardisCrypto = crypto::crypto_main::TardisCrypto {
        key: crypto::crypto_key::TardisCryptoKey {},
        hex: crypto::crypto_hex::TardisCryptoHex {},
        base64: crypto::crypto_base64::TardisCryptoBase64 {},
        aead: crypto::crypto_aead::TardisCryptoAead {},
        rsa: crypto::crypto_rsa::TardisCryptoRsa {},
        #[cfg(feature = "crypto-with-sm")]
        sm4: crypto::crypto_sm2_4::TardisCryptoSm4 {},
        #[cfg(feature = "crypto-with-sm")]
        sm2: crypto::crypto_sm2_4::TardisCryptoSm2 {},
        digest: crypto::crypto_digest::TardisCryptoDigest {},
    };

    pub fn tracing() -> Arc<TardisTracing> {
        tardis_instance().tracing.get()
    }

    /// Use the relational database feature / 使用关系型数据库功能
    ///
    /// This feature needs to be enabled #[cfg(feature = "reldb/reldb-postgres/reldb-mysql/reldb-sqlite")] .
    ///
    /// 本功能需要启用 #[cfg(feature = "reldb/reldb-postgres/reldb-mysql/reldb-sqlite")] .
    ///
    /// # Steps to use / 使用步骤
    ///
    /// 1. Initialize the database configuration / 初始化数据库配置 @see [init](Self::init)
    /// 2. Add the database / 添加数据库 E.g.
    /// ```ignore
    /// mod todos{
    ///     use tardis::basic::dto::TardisContext;
    ///     use tardis::db::reldb_client::TardisActiveModel;
    ///     use tardis::db::sea_orm::*;
    ///     
    ///     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    ///     #[sea_orm(table_name = "todos")]
    ///     pub struct Model {
    ///         #[sea_orm(primary_key)]
    ///         pub id: i32,
    ///         pub code: String,
    ///         pub description: String,
    ///         pub done: bool,
    ///     }
    ///     
    ///     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    ///     pub enum Relation {}
    ///     
    ///     impl TardisActiveModel for ActiveModel {
    ///         fn fill_ctx(&mut self, _: &TardisContext, _: bool) {}
    ///     }
    ///     
    ///     impl ActiveModelBehavior for ActiveModel {}
    /// }
    /// ```
    /// 3. Call this function to complete various data processing operations / 调用本函数完成各种数据处理操作 E.g.
    /// ```ignore
    /// use tardis::basic::error::TardisError;
    /// use tardis::TardisFuns;
    /// use tardis::db::sea_orm::*;
    /// use tardis::db::sea_orm::sea_query::Query;
    /// // Initialize table structure
    /// TardisFuns::reldb().conn().create_table_from_entity(todos::Entity).await?;
    /// // Create record
    /// let todo_id = TardisFuns::reldb()
    ///     .conn()
    ///     .insert_one(
    ///         todos::ActiveModel {
    ///             code: Set(todo_add_req.code.to_string()),
    ///             description: Set(todo_add_req.description.to_string()),
    ///             done: Set(todo_add_req.done),
    ///             ..Default::default()
    ///         },
    ///         &ctx.0,
    ///     ).unwrap()
    ///     .last_insert_id;
    /// // Query record
    /// let todo = TardisFuns::reldb()
    ///     .conn()
    ///     .get_dto(
    ///         DbQuery::select()
    ///             .columns(vec![todos::Column::Id, todos::Column::Code, todos::Column::Description, todos::Column::Done])
    ///             .from(todos::Entity)
    ///             .and_where(todos::Column::Id.eq(todo_id)),
    ///     )
    ///     .await.unwrap();
    /// ```
    #[cfg(feature = "reldb-core")]
    pub fn reldb() -> Arc<TardisRelDBClient> {
        Self::reldb_by_module("")
    }

    #[cfg(feature = "reldb-core")]
    pub fn reldb_by_module(code: &str) -> Arc<TardisRelDBClient> {
        let code = code.to_lowercase();
        let code = code.as_str();
        tardis_instance().reldb.get(code).unwrap_or_else(|| panic!("[Tardis.Config] RelDB {code} instance doesn't exist"))
    }

    #[cfg(feature = "reldb-core")]
    pub fn reldb_by_module_or_default(code: &str) -> Arc<TardisRelDBClient> {
        let code = code.to_lowercase();
        let code = code.as_str();
        tardis_instance().reldb.get(code).unwrap_or_else(Self::reldb)
    }

    #[allow(non_upper_case_globals)]
    #[cfg(feature = "reldb-core")]
    pub const dict: TardisDataDict = TardisDataDict {};

    #[cfg(feature = "web-server")]
    pub fn web_server() -> web::web_server::ArcTardisWebServer {
        tardis_instance().web_server.get().into()
    }

    /// Use the web  client feature / 使用web客户端功能
    ///
    /// This feature needs to be enabled #[cfg(feature = "web-client")] .
    ///
    /// 本功能需要启用 #[cfg(feature = "web-client")] .
    ///
    /// # Steps to use / 使用步骤
    ///
    /// 1. Initialize the cache configuration / 初始化缓存配置 @see [init](Self::init)
    /// 2. Call this function to complete various cache processing operations / 调用本函数完成各种缓存处理操作
    ///
    /// E.g.
    /// ```ignore
    /// use std::collections::HashMap;
    /// use tardis::basic::error::TardisError;
    /// use tardis::basic::result::{TardisResult, TARDIS_RESULT_ACCEPTED_CODE, TARDIS_RESULT_SUCCESS_CODE};
    /// use tardis::serde::{Deserialize, Serialize};
    /// use tardis::TardisFuns;
    /// use tardis::web::web_resp::{TardisApiResult, TardisResp};
    /// use reqwest::StatusCode;
    ///
    /// struct TodoResp {
    ///     id: i64,
    ///     code: TrimString,
    ///     description: String,
    ///     done: bool,
    /// }
    /// // Initiate a get request / 发起 Get 请求
    /// let res_object = TardisFuns::web_client()
    ///     .get::<TardisResp<TodoResp>>("https://www.xxx.com/query", Some([("User-Agent".to_string(), "Tardis".to_string())].iter().cloned().collect()))
    ///     .await
    ///     .unwrap();
    /// assert_eq!(response.code, 200);
    /// assert_eq!(response.body.as_ref().unwrap().code, TARDIS_RESULT_SUCCESS_CODE);
    /// assert_eq!(response.body.as_ref().unwrap().data.as_ref().unwrap().code.to_string(), "code1");
    /// // Initiate a get request return string / 发起 Get 请求并返回字符串
    /// let response = TardisFuns::web_client().
    /// get_to_str("https://www.xxx.com", Some([("User-Agent".to_string(), "Tardis".to_string())].iter().cloned().collect()))
    /// .await
    /// .unwrap();
    /// assert_eq!(response.code, StatusCode::OK.as_u16());
    /// assert!(response.body.unwrap().contains("xxx"));
    /// // Initiate a post request return string / 发起 Post 请求并返回字符串
    /// let request = serde_json::json!({
    ///     "lang": "rust",
    ///     "body": "json"
    /// });
    /// let response = TardisFuns::web_client().post_obj_to_str("https://www.xxx.com", &request, None).await?;
    /// assert_eq!(response.code, StatusCode::OK.as_u16());
    /// assert!(response.body.unwrap().contains(r#"data": "{\"body\":\"json\",\"lang\":\"rust\"}"#));
    ///
    /// // Initiate a post request return the custom struct / 发起 Post 请求并返回自定义结构
    /// #[derive(Debug, Serialize, Deserialize)]
    /// struct Post {
    ///     id: Option<i32>,
    ///     title: String,
    ///     body: String,
    ///     #[serde(rename = "userId")]
    ///     user_id: i32,
    /// }
    /// let new_post = Post {
    ///     id: None,
    ///     title: "idealworld".into(),
    ///     body: "http://idealworld.group/".into(),
    ///     user_id: 1,
    /// };
    /// let response = TardisFuns::web_client().post::<Post, Post>("https://postman-echo.com/post", &new_post, None).await?;
    /// assert_eq!(response.code, StatusCode::CREATED.as_u16());
    /// assert_eq!(response.body.unwrap().body, "http://idealworld.group/");
    /// ```
    #[cfg(feature = "web-client")]
    pub fn web_client() -> Arc<TardisWebClient> {
        Self::web_client_by_module("")
    }

    #[cfg(feature = "web-client")]
    pub fn web_client_by_module(code: &str) -> Arc<TardisWebClient> {
        let code = code.to_lowercase();
        let code = code.as_str();
        tardis_instance().web_client.get(code).unwrap_or_else(|| panic!("[Tardis.Config] Web Client {code} instance doesn't exist"))
    }

    #[cfg(feature = "web-client")]
    pub fn web_client_by_module_or_default(code: &str) -> Arc<TardisWebClient> {
        let code = code.to_lowercase();
        let code = code.as_str();
        tardis_instance().web_client.get(code).unwrap_or_else(Self::web_client)
    }

    #[cfg(feature = "ws-client")]
    pub async fn ws_client<F, T>(str_url: &str, on_message: F) -> TardisResult<web::ws_client::TardisWSClient>
    where
        F: Fn(tokio_tungstenite::tungstenite::Message) -> T + Send + Sync + Clone + 'static,
        T: futures::Future<Output = Option<tokio_tungstenite::tungstenite::Message>> + Send + Sync + 'static,
    {
        web::ws_client::TardisWSClient::connect(str_url, on_message).await
    }

    /// Use the distributed cache feature / 使用分布式缓存功能
    ///
    /// This feature needs to be enabled #[cfg(feature = "cache")] .
    ///
    /// 本功能需要启用 #[cfg(feature = "cache")] .
    ///
    /// # Steps to use / 使用步骤
    ///
    /// 1. Initialize the cache configuration / 初始化缓存配置 @see [init](Self::init)
    /// 2. Call this function to complete various cache processing operations / 调用本函数完成各种缓存处理操作
    ///
    /// E.g.
    /// ```ignore
    /// use tardis::TardisFuns;
    /// assert_eq!(TardisFuns::cache().get("test_key").await.unwrap(), None);
    /// client.set("test_key", "测试").await.unwrap();
    /// assert_eq!(TardisFuns::cache().get("test_key").await.unwrap(), "测试");
    /// assert!(TardisFuns::cache().set_nx("test_key2", "测试2").await.unwrap());
    /// assert!(!TardisFuns::cache().set_nx("test_key2", "测试2").await.unwrap());
    /// ```
    #[cfg(feature = "cache")]
    pub fn cache() -> Arc<TardisCacheClient> {
        Self::cache_by_module("")
    }

    #[cfg(feature = "cache")]
    pub fn cache_by_module(code: &str) -> Arc<TardisCacheClient> {
        let code = code.to_lowercase();
        let code = code.as_str();
        tardis_instance().cache.get(code).unwrap_or_else(|| panic!("[Tardis.Config] Cache {code} instance doesn't exist"))
    }

    #[cfg(feature = "cache")]
    pub fn cache_by_module_or_default(code: &str) -> Arc<TardisCacheClient> {
        let code = code.to_lowercase();
        let code = code.as_str();
        tardis_instance().cache.get(code).unwrap_or_else(Self::cache)
    }

    /// Use the message queue feature / 使用消息队列功能
    ///
    /// This feature needs to be enabled #[cfg(feature = "mq")] .
    ///
    /// 本功能需要启用 #[cfg(feature = "mq")] .
    ///
    /// # Steps to use / 使用步骤
    ///
    /// 1. Initialize the mq configuration / 初始化队列配置 @see [init](Self::init)
    /// 2. Call this function to complete various mq processing operations / 调用本函数完成各种队列处理操作
    ///
    /// E.g.
    /// ```ignore
    /// use tardis::TardisFuns;
    /// // publish a message / 发布一条消息
    /// TardisFuns::mq().publish("mq_topic_user_add", String::from("message content")).await.unwrap();
    /// // listen topic and consume message / 监听频道并且消费消息
    /// funs.mq().subscribe("mq_topic_user_add", |(_, _)| async { Ok(()) }).await?;
    /// ```
    #[cfg(feature = "mq")]
    pub fn mq() -> Arc<TardisMQClient> {
        Self::mq_by_module("")
    }

    #[cfg(feature = "mq")]
    pub fn mq_by_module(code: &str) -> Arc<TardisMQClient> {
        let code = code.to_lowercase();
        let code = code.as_str();
        tardis_instance().mq.get(code).unwrap_or_else(|| panic!("[Tardis.Config] MQ {code} instance doesn't exist"))
    }

    #[cfg(feature = "mq")]
    pub fn mq_by_module_or_default(code: &str) -> Arc<TardisMQClient> {
        let code = code.to_lowercase();
        let code = code.as_str();
        tardis_instance().mq.get(code).unwrap_or_else(Self::mq)
    }

    /// Use the distributed search feature / 使用分布式搜索功能
    ///
    /// This feature needs to be enabled #[cfg(feature = "web-client")] .
    ///
    /// 本功能需要启用 #[cfg(feature = "web-client")] .
    ///
    /// # Steps to use / 使用步骤
    ///
    /// 1. Initialize the web client configuration / 初始化web客户端配置 @see [init](Self::init)
    /// 2. Call this function to complete various search processing operations / 调用本函数完成各种搜索处理操作
    ///
    /// E.g.
    /// ```ignore
    /// use tardis::TardisFuns;
    /// TardisFuns::search().create_index("test_index").await.unwrap();
    /// let id = TardisFuns::search().create_record("test_index", r#"{"user":{"id":1, "name":"张三", "open":false}}"#).await.unwrap();
    /// assert_eq!(TardisFuns::search().get_record("test_index", &id).await.unwrap(), r#"{"user":{"id":4,"name":"Tom","open":true}}"#);
    /// TardisFuns::search().simple_search("test_index", "张三").await.unwrap();
    /// ```
    #[cfg(feature = "web-client")]
    pub fn search() -> Arc<TardisSearchClient> {
        Self::search_by_module("")
    }

    #[cfg(feature = "web-client")]
    pub fn search_by_module(code: &str) -> Arc<TardisSearchClient> {
        let code = code.to_lowercase();
        let code = code.as_str();
        tardis_instance().search.get(code).unwrap_or_else(|| panic!("[Tardis.Config] Search {code} instance doesn't exist"))
    }

    #[cfg(feature = "web-client")]
    pub fn search_by_module_or_default(code: &str) -> Arc<TardisSearchClient> {
        let code = code.to_lowercase();
        let code = code.as_str();
        tardis_instance().search.get(code).unwrap_or_else(Self::search)
    }

    #[cfg(feature = "mail")]
    pub fn mail() -> Arc<TardisMailClient> {
        Self::mail_by_module("")
    }

    #[cfg(feature = "mail")]
    pub fn mail_by_module(code: &str) -> Arc<TardisMailClient> {
        let code = code.to_lowercase();
        let code = code.as_str();
        tardis_instance().mail.get(code).unwrap_or_else(|| panic!("[Tardis.Config] Mail {code} instance doesn't exist"))
    }

    #[cfg(feature = "mail")]
    pub fn mail_by_module_or_default(code: &str) -> Arc<TardisMailClient> {
        let code = code.to_lowercase();
        let code = code.as_str();
        tardis_instance().mail.get(code).unwrap_or_else(Self::mail)
    }

    #[cfg(feature = "os")]
    pub fn os() -> Arc<TardisOSClient> {
        Self::os_by_module("")
    }

    #[cfg(feature = "os")]
    pub fn os_by_module(code: &str) -> Arc<TardisOSClient> {
        let code = code.to_lowercase();
        let code = code.as_str();
        tardis_instance().os.get(code).unwrap_or_else(|| panic!("[Tardis.Config] Os {code} instance doesn't exist"))
    }

    #[cfg(feature = "os")]
    pub fn os_by_module_or_default(code: &str) -> Arc<TardisOSClient> {
        let code = code.to_lowercase();
        let code = code.as_str();
        tardis_instance().os.get(code).unwrap_or_else(Self::os)
    }

    /// # Parameters
    /// - `clean: bool`: if use clean mode, it will cleanup all user setted configs like webserver modules
    async fn shutdown_internal(#[allow(unused_variables)] clean: bool) -> TardisResult<()> {
        tracing::info!("[Tardis] Shutdown...");
        // using a join set to collect async task, because `&TARDIS_INST` is not `Send`
        #[cfg(feature = "web-client")]
        tardis_instance().web_client.clear();
        #[cfg(feature = "cache")]
        tardis_instance().cache.clear();
        #[cfg(feature = "mail")]
        tardis_instance().mail.clear();
        #[cfg(feature = "os")]
        tardis_instance().os.clear();
        // reldb needn't shutdown
        // connection will be closed by drop calling
        // see: https://www.sea-ql.org/SeaORM/docs/install-and-config/connection/
        #[cfg(feature = "reldb-core")]
        tardis_instance().reldb.clear();
        #[cfg(feature = "mq")]
        {
            let mq = tardis_instance().mq.drain();
            for (code, client) in mq {
                if let Err(e) = client.close().await {
                    tracing::error!("[Tardis] Encounter an error while shutting down MQClient [{code}]: {}", e);
                }
            }
        }
        #[cfg(feature = "web-server")]
        {
            let web_server = tardis_instance().web_server.get();
            if web_server.is_running().await {
                if let Err(e) = web_server.shutdown().await {
                    tracing::error!("[Tardis] Encounter an error while shutting down webserver: {}", e);
                }
            }
        }
        tracing::info!("[Tardis] Shutdown finished");
        Ok(())
    }

    /// shutdown totally
    pub async fn shutdown() -> TardisResult<()> {
        Self::shutdown_internal(true).await
    }

    /// hot reload tardis instance by a new [`TardisConfig`].
    ///
    /// there should have only one hot reload task at the same time. If it's called when other reload task is running,
    /// it will wait until the other task finished.
    pub async fn hot_reload(conf: TardisConfig) -> TardisResult<()> {
        use tokio::sync::Semaphore;
        tardis_static! {
            tardis_load_semaphore: Semaphore = Semaphore::new(1);
        }
        let _sync = tardis_load_semaphore().acquire().await.expect("reload_semaphore is static so it shouldn't be closed.");
        let new_custom_config = conf.cs.iter().map(|(k, v)| (k.clone(), CachedJsonValue::new(v.clone()))).collect::<HashMap<_, _>>();
        let new_framework_config = conf.fw;
        #[allow(unused_variables)]
        let old_custom_config = tardis_instance().custom_config.replace_inner(new_custom_config);
        #[allow(unused_variables)]
        let old_framework_config = tardis_instance().framework_config.replace(new_framework_config);

        #[allow(unused_variables)]
        let fw_config = TardisFuns::fw_config();

        if fw_config.log != old_framework_config.log {
            if let Some(log_config) = &fw_config.log {
                tardis_instance().tracing.get().update_config(log_config)?;
            }
        }

        #[cfg(feature = "reldb-core")]
        {
            if fw_config.db != old_framework_config.db {
                if let Some(db_config) = &fw_config.db {
                    tardis_instance().reldb.init_by(db_config).await?;
                }
            }
        }
        #[cfg(feature = "web-server")]
        {
            if fw_config.web_server.is_some() && old_framework_config.web_server != fw_config.web_server {
                let web_server = TardisWebServer::init_by_conf(&fw_config)?;
                let old_server = tardis_instance().web_server.get();
                // if there's some inherit webserver
                if old_server.is_running().await {
                    // 1. shutdown webserver
                    old_server.shutdown().await?;
                    // 2. load initializers
                    web_server.load_initializer(old_server).await;
                    // 3. restart webserver
                    web_server.start().await?;
                }
                tardis_instance().web_server.set(web_server)
            }
        }
        #[cfg(feature = "web-client")]
        {
            if let Some(web_client_config) = &fw_config.web_client {
                tardis_instance().web_client.init_by(web_client_config).await?;
            }
        }
        #[cfg(feature = "cache")]
        {
            if let Some(cache_config) = &fw_config.cache {
                tardis_instance().cache.init_by(cache_config).await?;
            }
        }
        #[cfg(feature = "mq")]
        {
            if fw_config.mq != old_framework_config.mq {
                if let Some(mq_config) = &fw_config.mq {
                    let mut old_mq_clients = tardis_instance().mq.init_by(mq_config).await?;
                    for (code, client) in old_mq_clients.drain() {
                        if let Err(e) = client.close().await {
                            tracing::error!("[Tardis] Encounter an error while shutting down MQClient [{code}]: {}", e);
                        }
                    }
                }
            }
        }
        #[cfg(feature = "mail")]
        {
            if let Some(mail_config) = &fw_config.mail {
                tardis_instance().mail.init_by(mail_config).await?;
            }
        }
        #[cfg(feature = "os")]
        {
            if let Some(os_config) = &fw_config.os {
                tardis_instance().os.init_by(os_config).await?;
            }
        }
        Ok(())
    }
}

/// Single module objects  / 单模块对象
///
/// # Initialization / 初始化
///
/// ## Build objects extracted through the TardisFuns portal / 通过 TardisFuns 提取出对象
///    
///  ```ignore
/// use tardis::TardisFuns;
/// let funs = TardisFuns::inst("product".to_string(), None);
/// ```
///
/// ## Or build with db connect
///
/// ```ignore
/// use tardis::TardisFuns;
/// let funs = TardisFuns::inst_with_db_conn("product".to_string(), None);
/// ```
pub struct TardisFunsInst {
    module_code: String,
    err: TardisErrorWithExt,
    #[cfg(feature = "reldb-core")]
    db: Option<db::reldb_client::TardisRelDBlConnection>,
}

impl TardisFunsInst {
    pub(crate) fn new(code: String, lang: Option<String>) -> Self {
        Self {
            module_code: code.to_lowercase(),
            err: TardisErrorWithExt {
                ext: code.to_lowercase(),
                lang: if lang.is_some() { lang } else { TardisFuns::fw_config().app.default_lang.clone() },
            },
            #[cfg(feature = "reldb-core")]
            db: None,
        }
    }

    #[cfg(feature = "reldb-core")]
    pub(crate) fn new_with_db_conn(code: String, lang: Option<String>) -> Self {
        let reldb = TardisFuns::reldb_by_module_or_default(&code);
        Self {
            module_code: code.to_lowercase(),
            err: TardisErrorWithExt {
                ext: code.to_lowercase(),
                lang: if lang.is_some() { lang } else { TardisFuns::fw_config().app.default_lang.clone() },
            },
            db: Some(reldb.conn()),
        }
    }

    /// Get current module's code.
    pub fn module_code(&self) -> &str {
        &self.module_code
    }

    /// Get current module's config from custom configs.
    pub fn conf<T: 'static + for<'a> Deserialize<'a> + Any + Send + Sync>(&self) -> Arc<T> {
        TardisFuns::cs_config(&self.module_code)
    }

    /// Get current module's config from custom configs.
    pub fn err(&self) -> &TardisErrorWithExt {
        &self.err
    }

    /// Get current module's rel db client from custom configs.
    #[cfg(feature = "reldb-core")]
    pub fn reldb(&self) -> Arc<TardisRelDBClient> {
        TardisFuns::reldb_by_module_or_default(&self.module_code)
    }

    /// Get current module's db connection client from custom configs.
    #[cfg(feature = "reldb-core")]
    pub fn db(&self) -> &db::reldb_client::TardisRelDBlConnection {
        self.db.as_ref().expect("db is not initialized")
    }

    /// begin a transaction
    #[cfg(feature = "reldb-core")]
    pub async fn begin(&mut self) -> TardisResult<()> {
        self.db.as_mut().expect("db is not initialized").begin().await
    }

    /// commit transaction
    #[cfg(feature = "reldb-core")]
    pub async fn commit(self) -> TardisResult<()> {
        self.db.expect("db is not initialized").commit().await
    }

    /// rollback transaction
    #[cfg(feature = "reldb-core")]
    pub async fn rollback(self) -> TardisResult<()> {
        self.db.expect("db is not initialized").rollback().await
    }

    /// Get current module's cache client.
    #[cfg(feature = "cache")]
    pub fn cache(&self) -> Arc<TardisCacheClient> {
        TardisFuns::cache_by_module_or_default(&self.module_code)
    }

    /// Get current module's mq client.
    #[cfg(feature = "mq")]
    pub fn mq(&self) -> Arc<TardisMQClient> {
        TardisFuns::mq_by_module_or_default(&self.module_code)
    }

    /// Get current module's web client.
    #[cfg(feature = "web-client")]
    pub fn web_client(&self) -> Arc<TardisWebClient> {
        TardisFuns::web_client_by_module_or_default(&self.module_code)
    }

    /// Get current module's search client.
    #[cfg(feature = "web-client")]
    pub fn search(&self) -> Arc<TardisSearchClient> {
        TardisFuns::search_by_module_or_default(&self.module_code)
    }

    /// Get current module's mail client.
    #[cfg(feature = "mail")]
    pub fn mail(&self) -> Arc<TardisMailClient> {
        TardisFuns::mail_by_module_or_default(&self.module_code)
    }

    /// Get current module's os client.
    #[cfg(feature = "os")]
    pub fn os(&self) -> Arc<TardisOSClient> {
        TardisFuns::os_by_module_or_default(&self.module_code)
    }
}

pub mod basic;
#[cfg(feature = "cache")]
#[cfg_attr(docsrs, doc(cfg(feature = "cache")))]
pub mod cache;

pub mod config;
#[cfg(any(feature = "crypto", feature = "base64"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "crypto", feature = "base64"))))]
pub mod crypto;
#[cfg(feature = "reldb-core")]
#[cfg_attr(docsrs, doc(cfg(feature = "reldb-core")))]
pub mod db;
#[cfg(feature = "mail")]
#[cfg_attr(docsrs, doc(cfg(feature = "mail")))]
pub mod mail;
#[cfg(feature = "mq")]
#[cfg_attr(docsrs, doc(cfg(feature = "mq")))]
pub mod mq;
#[cfg(feature = "os")]
#[cfg_attr(docsrs, doc(cfg(feature = "os")))]
pub mod os;
#[cfg(feature = "web-client")]
#[cfg_attr(docsrs, doc(cfg(feature = "web-client")))]
pub mod search;
#[cfg(feature = "test")]
#[cfg_attr(docsrs, doc(cfg(feature = "test")))]
pub mod test;
pub mod web;

pub mod consts;
pub mod utils;
