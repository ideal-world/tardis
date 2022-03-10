//! **Elegant, Clean Rust development framework🛸**
//!
//! > TARDIS([tɑːrdɪs] "Time And Relative Dimension In Space") From "Doctor Who".
//!
//! ## 💖 Core functions
//!
//! * Relational database client for MySQL, PostgresSQL
//! * Web service and web client for OpenAPI v3.x
//! * Distributed cache client for Redis protocol
//! * RabbitMQ client for AMQP protocol
//! * Mainstream encryption algorithms and SM2/3/4 algorithms
//! * Containerized unit testing of mainstream middleware
//! * Multi-environment configuration
//! * Commonly used operations (E.g. uniform error handling, encryption and decryption, regular
//! checksums)
//!
//! ## ⚙️Feature description
//!
//! * ``trace`` tracing operation
//! * ``crypto`` Encryption, decryption and digest operations
//! * ``future`` asynchronous operations
//! * ``reldb`` relational database operations(based on [SeaORM](https://github.com/SeaQL/sea-orm))
//! * ``web-server`` web service operations(based on [Poem](https://github.com/poem-web/poem))
//! * ``web-client`` web client operations
//! * ``cache`` cache operations
//! * ``mq`` message queue operations
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
//! poem-openapi = { version = "^1"}
//! ```
//!
//! Processor Configuration
//! ```rust
//! pub struct Api;
//!
//! #[OpenApi]
//! impl Api {
//!     #[oai(path = "/hello", method = "get")]
//!     async fn index(&self, name: Query<Option<String>>) -> TardisResult<String> {
//!         match name.0 {
//!             Some(name) => TardisResp::ok(format!("hello, {}!", name)),
//!             None => TardisResp::err(TardisError::NotFound("name does not exist".to_string())),
//!         }
//!     }
//! }
//! ```
//!
//! Startup class configuration
//! ```rust
//! #[tokio::main]
//! async fn main() -> TardisResult<()> {
//!     // Initial configuration
//!     TardisFuns::init::<NoneConfig>("config").await?;
//!     // Register the processor and start the web service
//!     TardisFuns::web_server().add_module("", Api).start().await
//! }
//! ```
//!
//! ### More examples
//!
//!> |-- examples  
//!>   |-- reldb         Relational database usage example  
//!>   |-- web-basic     Web service Usage Example  
//!>   |-- web-client    Web client Usage Example  
//!>   |-- webscoket     WebSocket Usage Example  
//!>   |-- cache         Cache Usage Example  
//!>   |-- mq            Message Queue Usage Example  
//!>   |-- todo          A complete project usage example  
//!>   |-- perf-test     Performance test case  
//!

#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate core;
#[macro_use]
extern crate lazy_static;

use std::any::Any;
use std::ptr::replace;

pub use chrono;
pub use log;
pub use serde;
pub use serde_json;
#[cfg(feature = "rt_tokio")]
pub use tokio;

use basic::result::TardisResult;

use crate::basic::config::{FrameworkConfig, TardisConfig};
use crate::basic::field::TardisField;
use crate::basic::json::TardisJson;
use crate::basic::logger::TardisLogger;
use crate::basic::uri::TardisUri;
#[cfg(feature = "cache")]
use crate::cache::cache_client::TardisCacheClient;
#[cfg(feature = "reldb")]
use crate::db::reldb_client::TardisRelDBClient;
#[cfg(feature = "mq")]
use crate::mq::mq_client::TardisMQClient;
use crate::serde::Deserialize;
#[cfg(feature = "web-client")]
use crate::web::web_client::TardisWebClient;
#[cfg(feature = "web-server")]
use crate::web::web_server::TardisWebServer;

/// The operational portal for Tardis core functions / Tardis核心功能的操作入口
///
/// # Initialization / 初始化
///
/// ## Define project-level configuration objects / 定义项目级配置对象
///
/// ```rust
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
/// ```rust
/// use tardis::TardisFuns;
/// TardisFuns::init::<ExampleConfig>("proj/config").await?;
/// ```
///
/// More examples of initialization can be found in: `test_basic_config.rs` .
///
/// 更多初始化的示例可参考： [`test_basic_config`] `test_basic_config.rs` .
///
/// # 使用
///
/// ```rust
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
///
///
pub struct TardisFuns {
    workspace_config: Option<Box<dyn Any>>,
    framework_config: Option<FrameworkConfig>,
    #[cfg(feature = "reldb")]
    reldb: Option<TardisRelDBClient>,
    #[cfg(feature = "web-server")]
    web_server: Option<TardisWebServer>,
    #[cfg(feature = "web-client")]
    web_client: Option<TardisWebClient>,
    #[cfg(feature = "cache")]
    cache: Option<TardisCacheClient>,
    #[cfg(feature = "mq")]
    mq: Option<TardisMQClient>,
}

static mut TARDIS_INST: TardisFuns = TardisFuns {
    workspace_config: None,
    framework_config: None,
    #[cfg(feature = "reldb")]
    reldb: None,
    #[cfg(feature = "web-server")]
    web_server: None,
    #[cfg(feature = "web-client")]
    web_client: None,
    #[cfg(feature = "cache")]
    cache: None,
    #[cfg(feature = "mq")]
    mq: None,
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
    /// ```rust
    /// env::set_var("PROFILE", "test");
    /// TardisFuns::init::<ExampleConfig>("proj/config").await;
    /// ```
    pub async fn init<T: 'static + Deserialize<'static>>(relative_path: &str) -> TardisResult<()> {
        TardisLogger::init()?;
        let config = TardisConfig::<T>::init(relative_path)?;
        TardisFuns::init_conf::<T>(config).await
    }

    /// Initialize log / 初始化日志
    ///
    /// The [init](Self::init) function will automatically call this function
    ///
    /// [init](Self::init) 函数时会自动调用此函数
    pub fn init_log() -> TardisResult<()> {
        TardisLogger::init()
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
    /// ```rust
    /// TardisFuns::init_conf(TardisConfig {
    ///             ws: NoneConfig {},
    ///             fw: FrameworkConfig {
    ///                 app: Default::default(),
    ///                 web_server: WebServerConfig {
    ///                     enabled: false,
    ///                     ..Default::default()
    ///                 },
    ///                 web_client: Default::default(),
    ///                 cache: CacheConfig { enabled: true, url },
    ///                 db: DBConfig {
    ///                     enabled: false,
    ///                     ..Default::default()
    ///                 },
    ///                 mq: MQConfig {
    ///                     enabled: false,
    ///                     ..Default::default()
    ///                 },
    ///                 adv: Default::default(),
    ///             },
    ///         })
    ///         .await
    /// ```
    pub async fn init_conf<T: 'static>(conf: TardisConfig<T>) -> TardisResult<()> {
        TardisLogger::init()?;
        unsafe {
            replace(&mut TARDIS_INST.workspace_config, Some(Box::new(conf.ws)));
            replace(&mut TARDIS_INST.framework_config, Some(conf.fw));
        };
        #[cfg(feature = "reldb")]
        {
            if TardisFuns::fw_config().db.enabled {
                let reldb_client = TardisRelDBClient::init_by_conf(TardisFuns::fw_config()).await?;
                unsafe {
                    replace(&mut TARDIS_INST.reldb, Some(reldb_client));
                };
            }
        }
        #[cfg(feature = "web-server")]
        {
            if TardisFuns::fw_config().web_server.enabled {
                let web_server = TardisWebServer::init_by_conf(TardisFuns::fw_config()).await?;
                unsafe {
                    replace(&mut TARDIS_INST.web_server, Some(web_server));
                };
            }
        }
        #[cfg(feature = "web-client")]
        {
            let web_client = TardisWebClient::init_by_conf(TardisFuns::fw_config())?;
            unsafe {
                replace(&mut TARDIS_INST.web_client, Some(web_client));
            };
        }
        #[cfg(feature = "cache")]
        {
            if TardisFuns::fw_config().cache.enabled {
                let cache_client = TardisCacheClient::init_by_conf(TardisFuns::fw_config()).await?;
                unsafe {
                    replace(&mut TARDIS_INST.cache, Some(cache_client));
                };
            }
        }
        #[cfg(feature = "mq")]
        {
            if TardisFuns::fw_config().mq.enabled {
                let mq_client = TardisMQClient::init_by_conf(TardisFuns::fw_config()).await?;
                unsafe {
                    replace(&mut TARDIS_INST.mq, Some(mq_client));
                };
            }
        }
        TardisResult::Ok(())
    }

    pub fn ws_config<T>() -> &'static T {
        unsafe {
            match &TARDIS_INST.workspace_config {
                None => panic!("[Tardis.Config] Raw Workspace Config doesn't exist"),
                Some(conf) => match conf.downcast_ref::<T>() {
                    None => panic!("[Tardis.Config] Workspace Config doesn't exist"),
                    Some(t) => t,
                },
            }
        }
    }

    pub fn fw_config() -> &'static FrameworkConfig {
        unsafe {
            match &TARDIS_INST.framework_config {
                None => panic!("[Tardis.Config] Framework Config doesn't exist"),
                Some(t) => t,
            }
        }
    }

    #[allow(non_upper_case_globals)]
    pub const field: TardisField = TardisField {};

    #[allow(non_upper_case_globals)]
    pub const json: TardisJson = TardisJson {};

    #[allow(non_upper_case_globals)]
    pub const uri: TardisUri = TardisUri {};

    #[allow(non_upper_case_globals)]
    #[cfg(feature = "crypto")]
    pub const crypto: crate::basic::crypto::TardisCrypto = crate::basic::crypto::TardisCrypto {
        base64: crate::basic::crypto::TardisCryptoBase64 {},
        aes: crate::basic::crypto::TardisCryptoAes {},
        sm4: crate::basic::crypto::TardisCryptoSm4 {},
        rsa: crate::basic::crypto::TardisCryptoRsa {},
        sm2: crate::basic::crypto::TardisCryptoSm2 {},
        digest: crate::basic::crypto::TardisCryptoDigest {},
        key: crate::basic::crypto::TardisCryptoKey {},
    };

    #[cfg(feature = "reldb")]
    pub fn reldb() -> &'static TardisRelDBClient {
        unsafe {
            match &TARDIS_INST.reldb {
                None => panic!("[Tardis.Config] RelDB default instance doesn't exist"),
                Some(t) => t,
            }
        }
    }

    #[cfg(feature = "web-server")]
    pub fn web_server() -> &'static mut TardisWebServer {
        unsafe {
            match &mut TARDIS_INST.web_server {
                None => panic!("[Tardis.Config] Web Server default instance doesn't exist"),
                Some(t) => t,
            }
        }
    }

    #[cfg(feature = "web-client")]
    pub fn web_client() -> &'static TardisWebClient {
        unsafe {
            match &TARDIS_INST.web_client {
                None => panic!("[Tardis.Config] Web Client default instance doesn't exist"),
                Some(t) => t,
            }
        }
    }

    #[cfg(feature = "cache")]
    pub fn cache() -> &'static mut TardisCacheClient {
        unsafe {
            match &mut TARDIS_INST.cache {
                None => panic!("[Tardis.Config] Cache default instance doesn't exist"),
                Some(t) => t,
            }
        }
    }

    #[cfg(feature = "mq")]
    pub fn mq() -> &'static mut TardisMQClient {
        unsafe {
            match &mut TARDIS_INST.mq {
                None => panic!("[Tardis.Config] MQ default instance doesn't exist"),
                Some(t) => t,
            }
        }
    }

    pub async fn shutdown() -> TardisResult<()> {
        log::info!("[Tardis] Shutdown...");
        #[cfg(feature = "mq")]
        TardisFuns::mq().close().await?;
        Ok(())
    }
}

pub mod basic;
#[cfg(feature = "cache")]
#[cfg_attr(docsrs, doc(cfg(feature = "cache")))]
pub mod cache;
#[cfg(feature = "reldb")]
#[cfg_attr(docsrs, doc(cfg(feature = "reldb")))]
pub mod db;
#[cfg(feature = "mq")]
#[cfg_attr(docsrs, doc(cfg(feature = "mq")))]
pub mod mq;
#[cfg(feature = "test")]
#[cfg_attr(docsrs, doc(cfg(feature = "test")))]
pub mod test;
pub mod web;
