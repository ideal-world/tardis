use std::collections::HashMap;
use std::sync::Arc;

use config::ConfigError;
use serde::Deserialize;
use tokio::task::JoinHandle;

use self::nacos_client::NacosClientError;

use super::{config_dto::ConfCenterConfig, config_processor::ConfCenterProcess};
use crate::basic::result::TardisResult;
use crate::config::config_processor::HttpSource;
use crate::config::config_utils::config_foreign_err;
use crate::TARDIS_INST;

pub mod nacos_client;
#[derive(Debug)]
/// Config from Nacos,
/// A handle corresponding to a remote config
pub(crate) struct ConfNacosConfigHandle<F: config::Format> 
{
    // pub base_url: String,
    // pub profile: String,
    // pub app_id: String,
    pub data_id: String,
    // pub access_token: String,
    pub tenant: Option<String>,
    pub group: String,
    pub nacos_client: Arc<nacos_client::NacosClient>,
    pub format: Arc<F>,
    /// md5 reciever of remote config
    pub md5: Arc<tokio::sync::Mutex<Option<String>>>,
}

impl<F: config::Format> Clone for ConfNacosConfigHandle<F> {
    fn clone(&self) -> Self {
        Self {
            data_id: self.data_id.clone(),
            tenant: self.tenant.clone(),
            group: self.group.clone(),
            nacos_client: self.nacos_client.clone(),
            format: self.format.clone(),
            md5: self.md5.clone(),
        }
    }
}

impl<F: config::Format> ConfNacosConfigHandle<F>
where F: Send + Sync + std::fmt::Debug + 'static,
 {
    fn new(profile: Option<&str>, app_id: &str, tenant: Option<&str>, group: &str, format:Arc<F>, nacos_client: &Arc<nacos_client::NacosClient>) -> Self {
        let data_id = format!("{}-{}", app_id, profile.unwrap_or("default"));
        Self {
            data_id,
            tenant: tenant.map(str::to_string),
            group: group.to_string(),
            nacos_client: nacos_client.clone(),
            format,
            md5: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
    fn get_nacos_config_descriptor(&self) -> nacos_client::NacosConfigDescriptor<'_> {
        nacos_client::NacosConfigDescriptor {
            data_id: &self.data_id,
            group: &self.group,
            tenant: self.tenant.as_deref(),
            md5: self.md5.clone(),
        }
    }
    fn watch(self, update_notifier: tokio::sync::broadcast::Sender<()>) -> JoinHandle<()> {
        let task = async move {
            loop {
                log::debug!("[Tardis.config] Nacos Remote Lisener start for {:?}", &self);
                // if request failed, wait for next poll
                // if response is empty, remote config not yet updated, wait for next poll
                let updated = self.nacos_client.listen_config(&self.get_nacos_config_descriptor()).await.map_err(config_foreign_err);

                if !updated.unwrap_or(false) {
                    tokio::time::sleep(self.nacos_client.poll_period).await;
                    continue;
                } else {
                    match update_notifier.send(()) {
                        Ok(_) => {
                            log::debug!("[Tardis.config] Nacos Remote config updated, send update notifier")
                        }
                        Err(_) => {
                            // if receiver dropped, stop watching
                            log::debug!("[Tardis.config] Nacos Remote config updated, but no receiver found, stop watching");
                            break;
                        }
                    }
                }
            }
        };
        tokio::spawn(task)
    }
}

#[async_trait::async_trait]
impl<F: config::Format> config::AsyncSource for ConfNacosConfigHandle<F>
where
    F: Send + Sync + std::fmt::Debug + 'static,
{
    async fn collect(&self) -> Result<config::Map<String, config::Value>, ConfigError> {
        match self.nacos_client.get_config(&self.get_nacos_config_descriptor()).await {
            Ok((status, config_text)) => {
                match status.as_u16() {
                    200 => {
                        log::debug!("[Tardis.config] Nacos Remote config server response: {}", config_text);
                        self.format.parse(None, &config_text).map_err(|error| ConfigError::Foreign(error))
                    },
                    404 => {
                        log::warn!("[Tardis.config] Nacos Remote config not found");
                        return Ok(config::Map::new());
                    }
                    _ => {
                        log::warn!("[Tardis.config] Nacos Remote config server error: {}", status);
                        return Ok(config::Map::new());
                    }
                }
            }
            Err(NacosClientError::ReqwestError(e)) => {
                if e.status().map(|s| u16::from(s) == 404).unwrap_or(false) {
                    log::warn!("[Tardis.config] Nacos Remote config server error: {}", e);
                    return Ok(config::Map::new());
                } else {
                    return Err(ConfigError::Foreign(Box::new(e)));
                }
            },
            Err(e) => return Err(ConfigError::Foreign(Box::new(e))),
        }
    }
}
#[derive(Debug)]
pub(crate) struct ConfNacosProcessor<'a, F: config::Format>
where F: Send + Sync + std::fmt::Debug + 'static,
 {
    pub(crate) conf_center_config: &'a ConfCenterConfig,
    pub(crate) default_config_handle: ConfNacosConfigHandle<F>,
    pub(crate) config_handle: Option<ConfNacosConfigHandle<F>>,
}

impl<'a, F: config::Format> ConfNacosProcessor<'a, F>
where F: Send + Sync + std::fmt::Debug + 'static,
 {
    pub async fn init(config: &'a ConfCenterConfig, profile: &'a str, app_id: &'a str, format: &Arc<F>) -> TardisResult<ConfNacosProcessor<'a, F>> {
        let mut client = nacos_client::NacosClient::new(&config.url);
        client.login(&config.username, &config.password).await.map_err(|error| ConfigError::Foreign(Box::new(error)))?;
        let nacos_client = Arc::new(client);
        let group = config.group.as_deref().unwrap_or("DEFAULT_GROUP");
        let tenant = config.namespace.as_deref();
        let default_config_handle = ConfNacosConfigHandle::new(None, app_id, tenant, group, format.clone(), &nacos_client);
        let config_handle = if !profile.is_empty() {
            // let (tx, rx) = tokio::sync::watch::channel(None);
            Some(ConfNacosConfigHandle::new(Some(profile), app_id, tenant, group, format.clone(), &nacos_client))
        } else {
            None
        };
        Ok(Self {
            conf_center_config: config,
            default_config_handle,
            config_handle,
        })
    }
}

// #[async_trait]
impl<'a, F: config::Format> ConfCenterProcess for ConfNacosProcessor<'a, F>
where
    F: Send + Sync + std::fmt::Debug + 'static,
{
    fn watch(self) -> JoinHandle<()> {
        let ConfNacosProcessor {
            conf_center_config,
            default_config_handle,
            config_handle,
        } = self;
        let update_notifier = conf_center_config.update_listener.clone();
        let h1 = default_config_handle.watch(update_notifier);
        let update_notifier = conf_center_config.update_listener.clone();

        let maybe_h2 = config_handle.map(|h| h.watch(update_notifier));
        tokio::spawn(async move {
            h1.await.unwrap();
            if let Some(h2) = maybe_h2 {
                h2.await.unwrap();
            }
        })
    }
    fn get_sources(&mut self) -> Vec<ConfNacosConfigHandle<F>> {
        let default_src = self.default_config_handle.clone();
        let mut sources = vec![default_src];
        sources.extend(self.config_handle.as_ref().map(Clone::clone));
        sources
    }


    type Source = ConfNacosConfigHandle<F>;
}

#[derive(Deserialize)]
struct AuthResponse {
    #[serde(rename(deserialize = "accessToken"))]
    pub access_token: String,
}
