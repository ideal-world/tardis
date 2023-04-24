use std::sync::Arc;

use config::ConfigError;
use tokio::task::JoinHandle;

use self::nacos_client::NacosClientError;

use super::{config_dto::ConfCenterConfig, config_processor::ConfCenterProcess};
use crate::basic::result::TardisResult;
use crate::config::config_utils::config_foreign_err;
pub mod nacos_client;
#[derive(Debug)]
/// Config from Nacos,
/// A source corresponding to a remote config
pub(crate) struct ConfNacosConfigSource<F: config::Format> {
    data_id: String,
    tenant: Option<String>,
    group: String,
    /// nacos client
    nacos_client: Arc<nacos_client::NacosClient>,
    format: Arc<F>,
    /// md5 of config content
    md5: Arc<tokio::sync::Mutex<Option<String>>>,
}

impl<F: config::Format> std::fmt::Display for ConfNacosConfigSource<F>
where
    F: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ConfNacosConfigSource {{ data_id: {}, tenant: {:?}, group: {}, nacos_client: {}, format: {:?} }}",
            self.data_id, self.tenant, self.group, self.nacos_client, self.format,
        )
    }
}

impl<F: config::Format> Clone for ConfNacosConfigSource<F> {
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

impl<F: config::Format> ConfNacosConfigSource<F>
where
    F: Send + Sync + std::fmt::Debug + 'static,
{
    /// create a new config source
    fn new(profile: Option<&str>, app_id: &str, tenant: Option<&str>, group: &str, format: Arc<F>, nacos_client: &Arc<nacos_client::NacosClient>) -> Self {
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

    /// get a nacos config descriptor of this source
    fn get_nacos_config_descriptor(&self) -> nacos_client::NacosConfigDescriptor<'_> {
        nacos_client::NacosConfigDescriptor {
            data_id: &self.data_id,
            group: &self.group,
            tenant: self.tenant.as_deref(),
            md5: self.md5.clone(),
        }
    }
    fn listen_update(self, update_notifier: tokio::sync::mpsc::Sender<()>) -> JoinHandle<()> {
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
                    match update_notifier.send(()).await {
                        Ok(_) => {
                            // tardis will be reboot, stop watching
                            log::debug!("[Tardis.config] Nacos Remote config updated, send update notifier")
                        }
                        Err(e) => {
                            // if receiver dropped, stop watching, since tardis wont be reboot anyway
                            log::debug!("[Tardis.config] Nacos Remote config updated, but no receiver found, stop watching, error: {e}");
                        }
                    }
                    break;
                }
            }
        };
        tokio::spawn(task)
    }
}

#[async_trait::async_trait]
impl<F: config::Format> config::AsyncSource for ConfNacosConfigSource<F>
where
    F: Send + Sync + std::fmt::Debug + 'static,
{
    async fn collect(&self) -> Result<config::Map<String, config::Value>, ConfigError> {
        log::debug!("[Tardis.config] Nacos Remote config server response: {}", &self);
        match self.nacos_client.get_config(&self.get_nacos_config_descriptor()).await {
            Ok((status, config_text)) => match status.as_u16() {
                200 => {
                    log::trace!("[Tardis.config] Nacos Remote config server response: {}", config_text);
                    self.format.parse(None, &config_text).map_err(|error| ConfigError::Foreign(error))
                }
                _ => {
                    log::warn!("[Tardis.config] Nacos Remote config server response with status: {}", status);
                    return Ok(config::Map::new());
                }
            },
            Err(NacosClientError::ReqwestError(e)) => {
                if e.status().map(|s| u16::from(s) == 404).unwrap_or(false) {
                    log::warn!("[Tardis.config] Nacos Remote config server error: {}", e);
                    return Ok(config::Map::new());
                } else {
                    return Err(ConfigError::Foreign(Box::new(e)));
                }
            }
            Err(e) => return Err(ConfigError::Foreign(Box::new(e))),
        }
    }
}

/// # Nacos config processor
/// implement ConfProcess for Naocs
#[derive(Debug)]
pub(crate) struct ConfNacosProcessor<F: config::Format>
where
    F: Send + Sync + std::fmt::Debug + 'static,
{
    /// *-default config source
    pub(crate) default_config_source: ConfNacosConfigSource<F>,
    /// *-{profile} config source, it could be none
    pub(crate) config_source: Option<ConfNacosConfigSource<F>>,
}

impl<F: config::Format> ConfNacosProcessor<F>
where
    F: Send + Sync + std::fmt::Debug + 'static,
{
    /// create a new nacos config processor
    pub async fn init(config: &ConfCenterConfig, profile: &str, app_id: &str, format: &Arc<F>) -> TardisResult<ConfNacosProcessor<F>> {
        let mut client = nacos_client::NacosClient::new(&config.url);
        // set polling interval, default to 5s
        client.poll_period = std::time::Duration::from_millis(config.config_change_polling_interval.unwrap_or(5000));
        client.login(&config.username, &config.password).await.map_err(|error| ConfigError::Foreign(Box::new(error)))?;
        let nacos_client = Arc::new(client);
        // default group is DEFAULT_GROUP
        let group = config.group.as_deref().unwrap_or("DEFAULT_GROUP");
        let tenant = config.namespace.as_deref();
        // there are two config source, *-{profile} could be empty
        let default_config_source = ConfNacosConfigSource::new(None, app_id, tenant, group, format.clone(), &nacos_client);
        let config_source = if !profile.is_empty() {
            Some(ConfNacosConfigSource::new(Some(profile), app_id, tenant, group, format.clone(), &nacos_client))
        } else {
            None
        };
        Ok(Self {
            default_config_source,
            config_source,
        })
    }
}

impl<F: config::Format> ConfCenterProcess for ConfNacosProcessor<F>
where
    F: Send + Sync + std::fmt::Debug + 'static,
{
    fn listen_update(self, reload_notifier: &tokio::sync::mpsc::Sender<()>) -> JoinHandle<()> {
        let ConfNacosProcessor {
            default_config_source,
            config_source,
        } = self;
        let h1 = default_config_source.listen_update(reload_notifier.clone());
        let maybe_h2 = config_source.map(|h| h.listen_update(reload_notifier.clone()));
        tokio::spawn(async move {
            h1.await.unwrap();
            if let Some(h2) = maybe_h2 {
                h2.await.unwrap();
            }
        })
    }
    fn get_sources(&mut self) -> Vec<ConfNacosConfigSource<F>> {
        let default_src = self.default_config_source.clone();
        let mut sources = vec![default_src];
        sources.extend(self.config_source.as_ref().map(Clone::clone));
        sources
    }

    type Source = ConfNacosConfigSource<F>;
}