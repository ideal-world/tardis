//! **Elegant, Clean Rust development framework🛸**
//!
//! > TARDIS(\[tɑːrdɪs\] "Time And Relative Dimension In Space") From "Doctor Who".
//!
//! ## 💖 Core functions
//!
//! * Relational database client for MySQL, PostgresSQL
//! * Web service and web client for OpenAPI v3.x
//! * Distributed cache client for Redis protocol
//! * RabbitMQ client for AMQP protocol
//! * Search client for Elasticsearch
//! * Mail client for SMTP protocol
//! * Object Storage client for arbitrary S3 compatible APIs
//! * Mainstream encryption algorithms and SM2/3/4 algorithms
//! * Containerized unit testing of mainstream middleware
//! * Multi-environment configuration
//! * Multi-application aggregation
//! * Configure encryption support
//! * Internationalization and localization support
//! * Commonly used operations (E.g. uniform error handling, encryption and decryption, regular checksums)
//!
//! ## ⚙️Key Features
//!
//! * ``conf-remote`` enable the unified configuration center
//! * ``crypto`` encryption, decryption and digest operations
//! * ``crypto-with-sm`` encryption, decryption and digest with SM.x operations
//! * ``future`` asynchronous operations
//! * ``reldb-core`` relational database core operations(based on [SeaORM](https://github.com/SeaQL/sea-orm))
//! * ``reldb-postgres`` relational database with postgres driver
//! * ``reldb-mysql`` relational database with mysql driver
//! * ``reldb-sqlite`` relational database with sqlite driver
//! * ``reldb`` relational database with postgres/mysql/sqlite drivers
//! * ``web-server`` web service operations(based on [Poem](https://github.com/poem-web/poem))
//! * ``web-client`` web client operations
//! * ``ws-client`` webscoket client operations
//! * ``cache`` cache operations
//! * ``mq`` message queue operations
//! * ``mail`` mail send operations
//! * ``os`` object Storage operations
//! * ``test`` unit test operations
//!
//! ## 🚀 Quick start
//!
//! The core operations of the framework all use ``TardisFuns`` as an entry point.
//! E.g.
//!
//!> TardisFuns::init(relative_path)      // Initialize the configuration  
//!> TardisFuns::field.x                  // Some field operations  
//!> TardisFuns::reldb().x                // Some relational database operations  
//!> TardisFuns::web_server().x           // Some web service operations  
//!
//! ### Web service example
//!
//! Dependency Configuration
//! ```toml
//! [dependencies]
//! tardis = { version = "^0", features = ["web-server"] }
//! ```
//!
//! Processor Configuration
//!```ignore
//! use tardis::basic::error::TardisError;
//! use tardis::web::poem_openapi;
//! use tardis::web::poem_openapi::param::Query;
//! use tardis::web::web_resp::{TardisApiResult, TardisResp};
//!
//! pub struct Api;
//!
//! #[poem_openapi::OpenApi]
//! impl Api {
//!     #[oai(path = "/hello", method = "get")]
//!     async fn index(&self, name: Query<Option<String>>) -> TardisResult<String> {
//!         match name.0 {
//!             Some(name) => TardisResp::ok(format!("hello, {name}!")),
//!             None => TardisResp::err(TardisError::NotFound("name does not exist".to_string())),
//!         }
//!     }
//! }
//! ```
//!
//! Startup class configuration
//!```ignore
//! use tardis::basic::result::TardisResult;
//! use tardis::tokio;
//! use tardis::TardisFuns;
//! use crate::processor::Api;
//! mod processor;
//!
//! #[tokio::main]
//! async fn main() -> TardisResult<()> {
//!     // Initial configuration
//!     TardisFuns::init("config").await?;
//!     // Register the processor and start the web service
//!     TardisFuns::web_server().add_module("", Api).start().await?;
//!     TardisFuns::web_server().await;
//!     Ok(());
//! }
//! ```
//!
//! ### More examples
//!
//! ```ignore
//! |-- examples  
//!   |-- reldb              Relational database usage example
//!   |-- web-basic          Web service Usage Example
//!   |-- web-client         Web client Usage Example
//!   |-- websocket          WebSocket Usage Example
//!   |-- cache              Cache Usage Example
//!   |-- mq                 Message Queue Usage Example
//!   |-- todos              A complete project usage example
//!   |-- multi-apps         Multi-application aggregation example
//!   |-- pg-graph-search    Graph search by Postgresql example
//!   |-- perf-test          Performance test case
//! ```

#![doc(html_logo_url = "https://raw.githubusercontent.com/ideal-world/tardis/main/logo.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(clippy::unwrap_used)]

extern crate core;
#[macro_use]
extern crate lazy_static;

use std::any::Any;
use std::collections::HashMap;
use std::ptr::replace;

#[cfg(feature = "future")]
pub use async_stream;
#[cfg(feature = "future")]
pub use async_trait;
use basic::tracing::TardisTracing;
pub use chrono;
pub use derive_more;
#[cfg(feature = "future")]
pub use futures;
#[cfg(feature = "future")]
pub use futures_util;
pub use lru;
pub use rand;
pub use regex;
pub use serde;
use serde::de::DeserializeOwned;
pub use serde_json;
use serde_json::Value;
#[cfg(feature = "tardis-macros")]
#[cfg(any(feature = "reldb-postgres", feature = "reldb-mysql"))]
pub use tardis_macros::{TardisCreateEntity, TardisCreateIndex, TardisCreateTable, TardisEmptyBehavior, TardisEmptyRelation};
#[cfg(feature = "test")]
pub use testcontainers;
pub use tokio;
use tokio::sync::broadcast;
pub use tracing as log;
pub use url;

use basic::error::TardisErrorWithExt;
use basic::result::TardisResult;

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
///             conf1: "".to_string(),
///             conf2: "".to_string(),
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
pub struct TardisFuns {
    custom_config: Option<HashMap<ModuleCode, Value>>,
    _custom_config_cached: Option<HashMap<ModuleCode, Box<dyn Any>>>,
    shutdown_signal_sender: Option<broadcast::Sender<()>>,
    framework_config: Option<FrameworkConfig>,
    #[cfg(feature = "reldb-core")]
    reldb: Option<HashMap<ModuleCode, TardisRelDBClient>>,
    #[cfg(feature = "web-server")]
    web_server: Option<TardisWebServer>,
    #[cfg(feature = "web-client")]
    web_client: Option<HashMap<ModuleCode, TardisWebClient>>,
    #[cfg(feature = "cache")]
    cache: Option<HashMap<ModuleCode, TardisCacheClient>>,
    #[cfg(feature = "mq")]
    mq: Option<HashMap<ModuleCode, TardisMQClient>>,
    #[cfg(feature = "web-client")]
    search: Option<HashMap<ModuleCode, TardisSearchClient>>,
    #[cfg(feature = "mail")]
    mail: Option<HashMap<ModuleCode, TardisMailClient>>,
    #[cfg(feature = "os")]
    os: Option<HashMap<ModuleCode, TardisOSClient>>,
}
type ModuleCode = String;

static mut TARDIS_INST: TardisFuns = TardisFuns {
    custom_config: None,
    _custom_config_cached: None,
    shutdown_signal_sender: None,
    framework_config: None,
    #[cfg(feature = "reldb-core")]
    reldb: None,
    #[cfg(feature = "web-server")]
    web_server: None,
    #[cfg(feature = "web-client")]
    web_client: None,
    #[cfg(feature = "cache")]
    cache: None,
    #[cfg(feature = "mq")]
    mq: None,
    #[cfg(feature = "web-client")]
    search: None,
    #[cfg(feature = "mail")]
    mail: None,
    #[cfg(feature = "os")]
    os: None,
};

#[allow(unsafe_code)]
impl TardisFuns {
    /// Get the configuration file from the specified path and initialize it / 从指定的路径中获取配置文件并初始化
    ///
    /// # Arguments
    ///
    /// * `relative_path` - the directory where the configuration file is located, without the
    /// configuration file name / 配置文件所在目录，不包含配置文件名
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::env;
    /// use tardis::TardisFuns;
    /// env::set_var("PROFILE", "test");
    /// TardisFuns::init("proj/config").await;
    /// ```
    pub async fn init(relative_path: Option<&str>) -> TardisResult<()> {
        #[cfg(not(feature = "tracing"))]
        TardisTracing::init_log()?;
        let config = TardisConfig::init(relative_path).await?;
        TardisFuns::init_conf(config).await
    }

    /// Initialize log / 初始化日志
    ///
    /// The [init](Self::init) function will automatically call this function
    ///
    /// [init](Self::init) 函数时会自动调用此函数
    pub fn init_log() -> TardisResult<()> {
        #[cfg(not(feature = "tracing"))]
        TardisTracing::init_log()?;
        Ok(())
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
    /// use tardis::basic::config::{CacheConfig, DBConfig, FrameworkConfig, MQConfig, SearchConfig, MailConfig, OSConfig, TardisConfig, WebServerConfig};
    /// use tardis::TardisFuns;
    /// let result = TardisFuns::init_conf(TardisConfig {
    ///             cs: Default::default(),
    ///             fw: FrameworkConfig {
    ///                 app: Default::default(),
    ///                 web_server: WebServerConfig {
    ///                     enabled: false,
    ///                     ..Default::default()
    ///                 },
    ///                 web_client: Default::default(),
    ///                 cache: CacheConfig { enabled: true, url:"".to_string(),..Default::default() },
    ///                 db: DBConfig {
    ///                     enabled: false,
    ///                     ..Default::default()
    ///                 },
    ///                 mq: MQConfig {
    ///                     enabled: false,
    ///                     ..Default::default()
    ///                 },
    ///                 search: SearchConfig{
    ///                    enabled: false,
    ///                    ..Default::default()
    ///                 },
    ///                 mail: MailConfig{
    ///                    enabled: false,
    ///                    ..Default::default()
    ///                 },
    ///                 os: OSConfig{
    ///                    enabled: false,
    ///                    ..Default::default()
    ///                 },
    ///                 adv: Default::default(),
    ///             },
    ///         })
    ///         .await;
    /// ```
    pub async fn init_conf(conf: TardisConfig) -> TardisResult<()> {
        #[cfg(feature = "tracing")]
        TardisTracing::init_tracing(&conf)?;
        unsafe {
            replace(&mut TARDIS_INST.shutdown_signal_sender, Some(broadcast::channel(1).0));
        };
        unsafe {
            replace(&mut TARDIS_INST.custom_config, Some(conf.cs));
            replace(&mut TARDIS_INST._custom_config_cached, Some(HashMap::new()));
            replace(&mut TARDIS_INST.framework_config, Some(conf.fw));
        };
        #[cfg(feature = "reldb-core")]
        {
            if TardisFuns::fw_config().db.enabled {
                let reldb_clients = TardisRelDBClient::init_by_conf(TardisFuns::fw_config()).await?;
                unsafe {
                    replace(&mut TARDIS_INST.reldb, Some(reldb_clients));
                };
            }
        }
        #[cfg(feature = "web-server")]
        {
            if TardisFuns::fw_config().web_server.enabled {
                let web_server = TardisWebServer::init_by_conf(TardisFuns::fw_config())?;
                unsafe {
                    // take out previous webserver first, because TARDIS_INST is not send and can't live cross an `await` point
                    let inherit = {
                        TARDIS_INST.web_server.take()
                        // `&mut TARDIS_INST` dropped here
                    };
                    // if there's some inherit webserver
                    if let Some(inherit) = inherit {
                        // 1. load initializers
                        web_server.load_initializer(inherit).await;
                        // 2. restart webserver
                        web_server.start().await?;
                    }
                    replace(&mut TARDIS_INST.web_server, Some(web_server));
                };
            }
        }
        #[cfg(feature = "web-client")]
        {
            let web_clients = TardisWebClient::init_by_conf(TardisFuns::fw_config())?;
            unsafe {
                replace(&mut TARDIS_INST.web_client, Some(web_clients));
            };
        }
        #[cfg(feature = "cache")]
        {
            if TardisFuns::fw_config().cache.enabled {
                let cache_clients = TardisCacheClient::init_by_conf(TardisFuns::fw_config()).await?;
                unsafe {
                    replace(&mut TARDIS_INST.cache, Some(cache_clients));
                };
            }
        }
        #[cfg(feature = "mq")]
        {
            if TardisFuns::fw_config().mq.enabled {
                let mq_clients = TardisMQClient::init_by_conf(TardisFuns::fw_config()).await?;
                unsafe {
                    replace(&mut TARDIS_INST.mq, Some(mq_clients));
                };
            }
        }
        #[cfg(feature = "web-client")]
        {
            if TardisFuns::fw_config().search.enabled && !TardisFuns::fw_config().search.url.is_empty() {
                let search_clients = TardisSearchClient::init_by_conf(TardisFuns::fw_config())?;
                unsafe {
                    replace(&mut TARDIS_INST.search, Some(search_clients));
                };
            }
        }
        #[cfg(feature = "mail")]
        {
            if TardisFuns::fw_config().mail.enabled {
                let mail_clients = TardisMailClient::init_by_conf(TardisFuns::fw_config())?;
                unsafe {
                    replace(&mut TARDIS_INST.mail, Some(mail_clients));
                };
            }
        }
        #[cfg(feature = "os")]
        {
            if TardisFuns::fw_config().os.enabled {
                let os_clients = TardisOSClient::init_by_conf(TardisFuns::fw_config())?;
                unsafe {
                    replace(&mut TARDIS_INST.os, Some(os_clients));
                };
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
    /// let funs = TardisFuns::inst("product".to_string(), None);
    /// ```
    pub fn inst(code: String, lang: Option<String>) -> TardisFunsInst {
        TardisFunsInst::new(code, lang)
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
    /// let funs = TardisFuns::inst_with_db_conn("product".to_string(), None);
    /// ```
    #[cfg(feature = "reldb-core")]
    pub fn inst_with_db_conn(code: String, lang: Option<String>) -> TardisFunsInst {
        TardisFunsInst::new_with_db_conn(code, lang)
    }

    /// Get the custom configuration object / 获取自定义配置对象
    pub fn cs_config<T: 'static + DeserializeOwned>(code: &str) -> &T {
        let code = code.to_lowercase();
        let code = code.as_str();
        unsafe {
            let conf = TARDIS_INST.custom_config.as_ref().expect("[Tardis.Config] Custom Config doesn't exist");
            let cached_conf = TARDIS_INST._custom_config_cached.as_ref().expect("[Tardis.Config] Custom Config doesn't exist");
            if let Some(t) = conf.get(code) {
                if let Some(cached_t) = cached_conf.get(code) {
                    return cached_t.downcast_ref::<T>().unwrap_or_else(|| panic!("[Tardis.Config] Custom Config [{code}] type error"));
                }
                let t: T = TardisFuns::json.json_to_obj(t.clone()).unwrap_or_else(|_| panic!("[Tardis.Config] Custom Config [{code}] type conversion error"));
                TARDIS_INST._custom_config_cached.as_mut().expect("[Tardis.Config] Custom Config doesn't exist").insert(code.to_string(), Box::new(t));
                return cached_conf
                    .get(code)
                    .unwrap_or_else(|| panic!("[Tardis.Config] Custom Config [{code}] doesn't exist"))
                    .downcast_ref::<T>()
                    .unwrap_or_else(|| panic!("[Tardis.Config] Custom Config [{code}] type error"));
            }
            if !code.is_empty() {
                return Self::cs_config("");
            }
            panic!("[Tardis.Config] Custom Config [{code}] or [] doesn't exist");
        }
    }

    /// Get default language in the custom configuration / 从自定义配置中获取默认语言
    pub fn default_lang() -> Option<String> {
        unsafe {
            match &TARDIS_INST.framework_config {
                None => None,
                Some(t) => t.app.default_lang.clone(),
            }
        }
    }

    /// Get the Tardis configuration object / 获取Tardis配置对象
    pub fn fw_config() -> &'static FrameworkConfig {
        unsafe {
            match &TARDIS_INST.framework_config {
                None => panic!("[Tardis.Config] Framework Config doesn't exist"),
                Some(t) => t,
            }
        }
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
        aes: crypto::crypto_aes::TardisCryptoAes {},
        rsa: crypto::crypto_rsa::TardisCryptoRsa {},
        #[cfg(feature = "crypto-with-sm")]
        sm4: crypto::crypto_sm2_4::TardisCryptoSm4 {},
        #[cfg(feature = "crypto-with-sm")]
        sm2: crypto::crypto_sm2_4::TardisCryptoSm2 {},
        digest: crypto::crypto_digest::TardisCryptoDigest {},
    };

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
    pub fn reldb() -> &'static TardisRelDBClient {
        Self::reldb_by_module("")
    }

    #[cfg(feature = "reldb-core")]
    pub fn reldb_by_module(code: &str) -> &'static TardisRelDBClient {
        let code = code.to_lowercase();
        let code = code.as_str();
        unsafe {
            match &TARDIS_INST.reldb {
                None => panic!("[Tardis.Config] RelDB instance doesn't exist"),
                Some(t) => match t.get(code) {
                    None => panic!("[Tardis.Config] RelDB {code} instance doesn't exist"),
                    Some(t) => t,
                },
            }
        }
    }

    #[cfg(feature = "reldb-core")]
    pub fn reldb_by_module_or_default(code: &str) -> &'static TardisRelDBClient {
        let code = code.to_lowercase();
        let code = code.as_str();
        unsafe {
            match &TARDIS_INST.reldb {
                None => panic!("[Tardis.Config] RelDB instance doesn't exist"),
                Some(t) => match t.get(code) {
                    None => Self::reldb(),
                    Some(t) => t,
                },
            }
        }
    }

    #[allow(non_upper_case_globals)]
    #[cfg(feature = "reldb-core")]
    pub const dict: TardisDataDict = TardisDataDict {};

    #[cfg(feature = "web-server")]
    pub fn web_server() -> &'static TardisWebServer {
        unsafe {
            match &mut TARDIS_INST.web_server {
                None => panic!("[Tardis.Config] Web Server default instance doesn't exist"),
                Some(t) => t,
            }
        }
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
    pub fn web_client() -> &'static TardisWebClient {
        Self::web_client_by_module("")
    }

    #[cfg(feature = "web-client")]
    pub fn web_client_by_module(code: &str) -> &'static TardisWebClient {
        let code = code.to_lowercase();
        let code = code.as_str();
        unsafe {
            match &TARDIS_INST.web_client {
                None => panic!("[Tardis.Config] Web Client instance doesn't exist"),
                Some(t) => match t.get(code) {
                    None => panic!("[Tardis.Config] Web Client {code} instance doesn't exist"),
                    Some(t) => t,
                },
            }
        }
    }

    #[cfg(feature = "web-client")]
    pub fn web_client_by_module_or_default(code: &str) -> &'static TardisWebClient {
        let code = code.to_lowercase();
        let code = code.as_str();
        unsafe {
            match &TARDIS_INST.web_client {
                None => panic!("[Tardis.Config] Web Client instance doesn't exist"),
                Some(t) => match t.get(code) {
                    None => Self::web_client(),
                    Some(t) => t,
                },
            }
        }
    }

    #[cfg(feature = "ws-client")]
    pub async fn ws_client<F, T>(str_url: &str, fun: F) -> TardisResult<web::ws_client::TardisWSClient<F, T>>
    where
        F: Fn(tokio_tungstenite::tungstenite::Message) -> T + Send + Sync + Copy + 'static,
        T: futures::Future<Output = Option<tokio_tungstenite::tungstenite::Message>> + Send + 'static,
    {
        web::ws_client::TardisWSClient::init(str_url, fun).await
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
    pub fn cache() -> &'static TardisCacheClient {
        Self::cache_by_module("")
    }

    #[cfg(feature = "cache")]
    pub fn cache_by_module(code: &str) -> &'static TardisCacheClient {
        let code = code.to_lowercase();
        let code = code.as_str();
        unsafe {
            match &TARDIS_INST.cache {
                None => panic!("[Tardis.Config] Cache instance doesn't exist"),
                Some(t) => match t.get(code) {
                    None => panic!("[Tardis.Config] Cache {code} instance doesn't exist"),
                    Some(t) => t,
                },
            }
        }
    }

    #[cfg(feature = "cache")]
    pub fn cache_by_module_or_default(code: &str) -> &'static TardisCacheClient {
        let code = code.to_lowercase();
        let code = code.as_str();
        unsafe {
            match &TARDIS_INST.cache {
                None => panic!("[Tardis.Config] Cache instance doesn't exist"),
                Some(t) => match t.get(code) {
                    None => Self::cache(),
                    Some(t) => t,
                },
            }
        }
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
    /// E.g.
    /// ```ignore
    /// use tardis::TardisFuns;
    /// // publish a message / 发布一条消息
    /// TardisFuns::mq().publish("mq_topic_user_add", String::from("message content")).await.unwrap();
    /// // listen topic and consume message / 监听频道并且消费消息
    /// funs.mq().subscribe("mq_topic_user_add", |(_, _)| async { Ok(()) }).await?;
    /// ```
    #[cfg(feature = "mq")]
    pub fn mq() -> &'static TardisMQClient {
        Self::mq_by_module("")
    }

    #[cfg(feature = "mq")]
    pub fn mq_by_module(code: &str) -> &'static TardisMQClient {
        let code = code.to_lowercase();
        let code = code.as_str();
        unsafe {
            match &TARDIS_INST.mq {
                None => panic!("[Tardis.Config] MQ instance doesn't exist"),
                Some(t) => match t.get(code) {
                    None => panic!("[Tardis.Config] MQ {code} instance doesn't exist"),
                    Some(t) => t,
                },
            }
        }
    }

    #[cfg(feature = "mq")]
    pub fn mq_by_module_or_default(code: &str) -> &'static TardisMQClient {
        let code = code.to_lowercase();
        let code = code.as_str();
        unsafe {
            match &TARDIS_INST.mq {
                None => panic!("[Tardis.Config] MQ instance doesn't exist"),
                Some(t) => match t.get(code) {
                    None => Self::mq(),
                    Some(t) => t,
                },
            }
        }
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
    /// E.g.
    /// ```ignore
    /// use tardis::TardisFuns;
    /// TardisFuns::search().create_index("test_index").await.unwrap();
    /// let id = TardisFuns::search().create_record("test_index", r#"{"user":{"id":1, "name":"张三", "open":false}}"#).await.unwrap();
    /// assert_eq!(TardisFuns::search().get_record("test_index", &id).await.unwrap(), r#"{"user":{"id":4,"name":"Tom","open":true}}"#);
    /// TardisFuns::search().simple_search("test_index", "张三").await.unwrap();
    /// ```
    #[cfg(feature = "web-client")]
    pub fn search() -> &'static TardisSearchClient {
        Self::search_by_module("")
    }

    #[cfg(feature = "web-client")]
    pub fn search_by_module(code: &str) -> &'static TardisSearchClient {
        let code = code.to_lowercase();
        let code = code.as_str();
        unsafe {
            match &TARDIS_INST.search {
                None => panic!("[Tardis.Config] Search instance doesn't exist"),
                Some(t) => match t.get(code) {
                    None => panic!("[Tardis.Config] Search {code} instance doesn't exist"),
                    Some(t) => t,
                },
            }
        }
    }

    #[cfg(feature = "web-client")]
    pub fn search_by_module_or_default(code: &str) -> &'static TardisSearchClient {
        let code = code.to_lowercase();
        let code = code.as_str();
        unsafe {
            match &TARDIS_INST.search {
                None => panic!("[Tardis.Config] Search instance doesn't exist"),
                Some(t) => match t.get(code) {
                    None => Self::search(),
                    Some(t) => t,
                },
            }
        }
    }

    #[cfg(feature = "mail")]
    pub fn mail() -> &'static TardisMailClient {
        Self::mail_by_module("")
    }

    #[cfg(feature = "mail")]
    pub fn mail_by_module(code: &str) -> &'static TardisMailClient {
        let code = code.to_lowercase();
        let code = code.as_str();
        unsafe {
            match &TARDIS_INST.mail {
                None => panic!("[Tardis.Config] Mail instance doesn't exist"),
                Some(t) => match t.get(code) {
                    None => panic!("[Tardis.Config] Mail {code} instance doesn't exist"),
                    Some(t) => t,
                },
            }
        }
    }

    #[cfg(feature = "mail")]
    pub fn mail_by_module_or_default(code: &str) -> &'static TardisMailClient {
        let code = code.to_lowercase();
        let code = code.as_str();
        unsafe {
            match &TARDIS_INST.mail {
                None => panic!("[Tardis.Config] Mail instance doesn't exist"),
                Some(t) => match t.get(code) {
                    None => Self::mail(),
                    Some(t) => t,
                },
            }
        }
    }

    #[cfg(feature = "os")]
    pub fn os() -> &'static TardisOSClient {
        Self::os_by_module("")
    }

    #[cfg(feature = "os")]
    pub fn os_by_module(code: &str) -> &'static TardisOSClient {
        let code = code.to_lowercase();
        let code = code.as_str();
        unsafe {
            match &TARDIS_INST.os {
                None => panic!("[Tardis.Config] Object Storage instance doesn't exist"),
                Some(t) => match t.get(code) {
                    None => panic!("[Tardis.Config] Object Storage {code} instance doesn't exist"),
                    Some(t) => t,
                },
            }
        }
    }

    #[cfg(feature = "os")]
    pub fn os_by_module_or_default(code: &str) -> &'static TardisOSClient {
        let code = code.to_lowercase();
        let code = code.as_str();
        unsafe {
            match &TARDIS_INST.os {
                None => panic!("[Tardis.Config] Object Storage instance doesn't exist"),
                Some(t) => match t.get(code) {
                    None => Self::os(),
                    Some(t) => t,
                },
            }
        }
    }

    /// subscribe shutdown signal / 订阅关闭信号 which is a None value.
    #[must_use]
    #[allow(dead_code)]
    pub(crate) fn subscribe_shutdown_signal() -> Option<broadcast::Receiver<()>> {
        unsafe { TARDIS_INST.shutdown_signal_sender.as_ref().map(broadcast::Sender::subscribe) }
    }

    /// # Parameters
    /// - `clean: bool`: if use clean mode, it will cleanup all user setted configs like webserver modules
    async fn shutdown_internal(clean: bool) -> TardisResult<()> {
        tracing::info!("[Tardis] Shutdown...");
        // using a join set to collect async task, because `&TARDIS_INST` is not `Send`
        let mut set = tokio::task::JoinSet::new();
        #[cfg(feature = "web-client")]
        unsafe {
            let _ = &TARDIS_INST.web_client.take();
        }
        #[cfg(feature = "cache")]
        unsafe {
            let _ = &TARDIS_INST.cache.take();
        }
        #[cfg(feature = "mail")]
        unsafe {
            let _ = &TARDIS_INST.mail.take();
        }
        #[cfg(feature = "os")]
        unsafe {
            let _ = &TARDIS_INST.os.take();
        }
        #[cfg(feature = "reldb-core")]
        unsafe {
            // reldb needn't shutdown
            // connection will be closed by drop calling
            // see: https://www.sea-ql.org/SeaORM/docs/install-and-config/connection/
            let _ = &TARDIS_INST.reldb.take();
        }
        #[cfg(feature = "mq")]
        unsafe {
            if let Some(t) = &TARDIS_INST.mq {
                for v in t.values() {
                    set.spawn(async {
                        if let Err(e) = v.close().await {
                            tracing::error!("[Tardis] Encounter an error while shutting down MQClient: {}", e);
                        }
                    });
                }
            }
            let _ = &TARDIS_INST.mq.take();
        }
        #[cfg(feature = "web-server")]
        unsafe {
            if let Some(web_server) = &TARDIS_INST.web_server {
                set.spawn(async {
                    if let Err(e) = web_server.shutdown().await {
                        tracing::error!("[Tardis] Encounter an error while shutting down webserver: {}", e);
                    }
                });
            }
        }
        
        while let Some(join_result) = set.join_next().await {
            if let Err(e) = join_result {
                tracing::error!("[Tardis] Fail to join async shutdown task: {}", e);
            }
        }

        // cleanup
        // # Safety
        // we shall **do** ensure all the async tasks finished here
        // if not, we may drop memories which being used by async tasks
        {
            #[cfg(feature = "web-server")]
            unsafe {
                if clean {
                    let _ = &TARDIS_INST.web_server.take();
                }
            }
            #[cfg(feature = "mq")]
            unsafe {
                if clean {
                    let _ = &TARDIS_INST.mq.take();
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

    /// shutdown with inherit mode
    /// 
    /// this shutdown function will retain some user setted configs like webserver moudules for next init
    pub async fn shutdown_inherit() -> TardisResult<()> {
        Self::shutdown_internal(false).await
    }

    /// hot reload tardis instance
    pub async fn hot_reload(conf: TardisConfig) -> TardisResult<()>{
        let old_config = unsafe {
            let cs = replace(&mut TARDIS_INST.custom_config, Some(conf.cs)).expect("hot reload before tardis initialized");
            replace(&mut TARDIS_INST._custom_config_cached, Some(HashMap::new()));
            let fw = replace(&mut TARDIS_INST.framework_config, Some(conf.fw)).expect("hot reload before tardis initialized");
            TardisConfig {
                cs,
                fw,
            }
        };
        
        let fw_config = TardisFuns::fw_config();


        #[cfg(feature = "reldb-core")]
        {
            if fw_config.db.enabled && fw_config.db != old_config.fw.db {
                let reldb_clients = TardisRelDBClient::init_by_conf(TardisFuns::fw_config()).await?;
                unsafe {
                    replace(&mut TARDIS_INST.reldb, Some(reldb_clients));
                };
            }
        }
        #[cfg(feature = "web-server")]
        {
            if fw_config.web_server.enabled && old_config.fw.web_server != fw_config.web_server {
                let web_server = TardisWebServer::init_by_conf(fw_config)?;
                unsafe {
                    // take out previous webserver first, because TARDIS_INST is not send and can't live cross an `await` point
                    let inherit = {
                        TARDIS_INST.web_server.take()
                        // `&mut TARDIS_INST` dropped here
                    };
                    // if there's some inherit webserver
                    if let Some(inherit) = inherit {
                        // 1. shutdown webserver
                        inherit.shutdown().await?;
                        // 2. load initializers
                        web_server.load_initializer(inherit).await;
                        // 3. restart webserver
                        web_server.start().await?;
                    }
                    replace(&mut TARDIS_INST.web_server, Some(web_server));
                };
            }
        }
        #[cfg(feature = "web-client")]
        {
            let web_clients = TardisWebClient::init_by_conf(fw_config)?;
            unsafe {
                replace(&mut TARDIS_INST.web_client, Some(web_clients));
            };
        }
        #[cfg(feature = "cache")]
        {
            if fw_config.cache.enabled {
                let cache_clients = TardisCacheClient::init_by_conf(fw_config).await?;
                unsafe {
                    replace(&mut TARDIS_INST.cache, Some(cache_clients));
                };
            }
        }
        #[cfg(feature = "mq")]
        {
            if fw_config.mq.enabled && fw_config.mq != old_config.fw.mq {
                unsafe {
                    let mq_clients = TARDIS_INST.mq.take();
                    if let Some(mq_clients) = mq_clients {
                        for v in mq_clients.values() {
                            if let Err(e) = v.close().await {
                                tracing::error!("[Tardis] Encounter an error while shutting down MQClient: {}", e);
                            }
                        }
                    }
                    let mq_clients = TardisMQClient::init_by_conf(fw_config).await?;
                    replace(&mut TARDIS_INST.mq, Some(mq_clients));
                }
            }
        }
        #[cfg(feature = "mail")]
        {
            if fw_config.mail.enabled {
                let mail_clients = TardisMailClient::init_by_conf(fw_config)?;
                unsafe {
                    replace(&mut TARDIS_INST.mail, Some(mail_clients));
                };
            }
        }
        #[cfg(feature = "os")]
        {
            if fw_config.os.enabled {
                let os_clients = TardisOSClient::init_by_conf(fw_config)?;
                unsafe {
                    replace(&mut TARDIS_INST.os, Some(os_clients));
                };
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

    pub fn module_code(&self) -> &str {
        &self.module_code
    }

    pub fn conf<T: 'static + DeserializeOwned>(&self) -> &T {
        TardisFuns::cs_config(&self.module_code)
    }

    pub fn err(&self) -> &TardisErrorWithExt {
        &self.err
    }

    #[cfg(feature = "reldb-core")]
    pub fn reldb(&self) -> &'static TardisRelDBClient {
        TardisFuns::reldb_by_module_or_default(&self.module_code)
    }

    #[cfg(feature = "reldb-core")]
    pub fn db(&self) -> &db::reldb_client::TardisRelDBlConnection {
        self.db.as_ref().expect("db is not initialized")
    }

    #[cfg(feature = "reldb-core")]
    pub async fn begin(&mut self) -> TardisResult<()> {
        self.db.as_mut().expect("db is not initialized").begin().await
    }

    #[cfg(feature = "reldb-core")]
    pub async fn commit(self) -> TardisResult<()> {
        self.db.expect("db is not initialized").commit().await
    }

    #[cfg(feature = "reldb-core")]
    pub async fn rollback(self) -> TardisResult<()> {
        self.db.expect("db is not initialized").rollback().await
    }

    #[cfg(feature = "cache")]
    pub fn cache(&self) -> &'static TardisCacheClient {
        TardisFuns::cache_by_module_or_default(&self.module_code)
    }

    #[cfg(feature = "mq")]
    pub fn mq(&self) -> &'static TardisMQClient {
        TardisFuns::mq_by_module_or_default(&self.module_code)
    }

    #[cfg(feature = "web-client")]
    pub fn web_client(&self) -> &'static TardisWebClient {
        TardisFuns::web_client_by_module_or_default(&self.module_code)
    }

    #[cfg(feature = "web-client")]
    pub fn search(&self) -> &'static TardisSearchClient {
        TardisFuns::search_by_module_or_default(&self.module_code)
    }

    #[cfg(feature = "mail")]
    pub fn mail(&self) -> &'static TardisMailClient {
        TardisFuns::mail_by_module_or_default(&self.module_code)
    }

    #[cfg(feature = "os")]
    pub fn os(&self) -> &'static TardisOSClient {
        TardisFuns::os_by_module_or_default(&self.module_code)
    }
}

pub mod basic;
#[cfg(feature = "cache")]
#[cfg_attr(docsrs, doc(cfg(feature = "cache")))]
pub mod cache;
pub mod config;
#[cfg(feature = "crypto")]
#[cfg_attr(docsrs, doc(cfg(feature = "crypto")))]
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
