#[macro_use]
extern crate lazy_static;
pub extern crate serde;
pub extern crate serde_json;
pub extern crate tokio;

use std::any::Any;
use std::ptr::replace;

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
    pub async fn init<T: 'static + Deserialize<'static>>(relative_path: &str) -> TardisResult<()> {
        TardisLogger::init()?;
        let config = TardisConfig::<T>::init(relative_path)?;
        TardisFuns::init_conf::<T>(config).await
    }

    pub fn init_log() -> TardisResult<()> {
        TardisLogger::init()
    }

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
pub mod cache;
#[cfg(feature = "reldb")]
pub mod db;
#[cfg(feature = "mq")]
pub mod mq;
#[cfg(feature = "test")]
pub mod test;
pub mod web;

pub mod log {
    pub use log::*;
}
