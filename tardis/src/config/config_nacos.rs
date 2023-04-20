use std::collections::HashMap;

use config::ConfigError;
use serde::Deserialize;
use tokio::task::JoinHandle;

use super::{config_dto::ConfCenterConfig, config_processor::ConfCenterProcess};
use crate::basic::result::TardisResult;
use crate::config::config_processor::HttpSource;

#[derive(Debug)]
/// Config from Nacos,
/// A handle corresponding to a remote config
pub(crate) struct ConfNacosConfigHandle {
    pub base_url: String,
    pub profile: String,
    pub app_id: String,
    pub access_token: String,
    pub tenant: Option<String>,
    pub group: String,
    /// md5 reciever of remote config
    pub md5_watcher: Option<tokio::sync::watch::Receiver<Option<String>>>,
}

impl ConfNacosConfigHandle {
    /// get config url from nacos, use to get remote config
    /// apidoc: https://nacos.io/zh-cn/docs/open-api.html
    fn get_url(&self) -> String {
        let mut url = format!(
            "{}/v1/cs/configs?accessToken={}&dataId={}-{}&group={}",
            self.base_url, self.access_token, self.app_id, self.profile, self.group
        );
        url.extend(self.tenant.as_ref().map(|tenant| format!("&tenant={tenant}")));
        url
    }

    /// get listener url from nacos, used to watch remote config change
    /// apidoc: https://nacos.io/zh-cn/docs/open-api.html
    fn get_listener_url(&self, content_md5: Option<&str>) -> String {
        let content_md5 = content_md5.unwrap_or("");
        let mut url = format!(
            "{}/v1/cs/configs/listener?accessToken={}&Listening-Configs={}-{}%02{}%02{}",
            self.base_url, self.access_token, self.app_id, self.profile, self.group, content_md5
        );
        url.extend(self.tenant.as_ref().map(|tenant| format!("%02{tenant}")));
        url.push_str("%01");
        url
    }

    /// get `HttpSource` instance, which is used to get remote config for crate `config`
    fn get_http_source<F: config::Format>(&mut self, format: F) -> HttpSource<F> {
        let source = HttpSource::new(self.get_url(), format);
        self.subscribe_change(&source);
        source
    }

    /// subscribe md5 change of remote config, from a `HttpSource` instance
    fn subscribe_change<F: config::Format>(&mut self, source: &HttpSource<F>) {
        self.md5_watcher = Some(source.md5_tx.subscribe());
    }

    /// watch remote config change, and send a message to `update_notifier` when remote config changed
    fn watch(self, update_notifier: tokio::sync::broadcast::Sender<()>) -> JoinHandle<()> {
        const POLL_PERIOD: std::time::Duration = std::time::Duration::from_secs(30);

        let task = async move {
            // TODO: use tokio::sync::onshot instead of tokio::sync::watch
            if let Some(md5_watcher) = &self.md5_watcher {
                loop {

                    let url = {
                        // borrowed value md5 cant alive across next await
                        let md5 = md5_watcher.borrow();
                        self.get_listener_url(md5.as_deref())
                    };
                    // if request failed, wait for next poll
                    // if response is empty, remote config not yet updated, wait for next poll
                    let new_cfg = {
                        let response = reqwest::Client::new()
                            .get(&url)
                            .header("Long-Pulling-Timeout", POLL_PERIOD.as_millis() as u64)
                            .send()
                            .await
                            .map_err(|error| ConfigError::Foreign(Box::new(error)));
                        match response {
                            Ok(response) => response.text().await.map_err(|error| ConfigError::Foreign(Box::new(error))),
                            Err(error) => Err(error),
                        }
                    };

                    if new_cfg.as_ref().map(String::is_empty).unwrap_or(true) {
                        tokio::time::sleep(POLL_PERIOD).await;
                        continue;
                    } else {
                        match update_notifier.send(()) {
                            Ok(_) => {
                                log::info!("[Tardis.config] Nacos Remote config updated, tardis is going to restart");
                            }
                            Err(_) => {
                                // if receiver dropped, stop watching
                                log::debug!("[Tardis.config] Nacos Remote config updated, but no receiver found, stop watching");
                                break;
                            }
                        }
                    }
                }
            }
        };
        tokio::spawn(task)
    }
}
#[derive(Debug)]
pub(crate) struct ConfNacosProcessor<'a> {
    pub(crate) conf_center_config: &'a ConfCenterConfig,
    pub(crate) default_config_handle: ConfNacosConfigHandle,
    pub(crate) config_handle: Option<ConfNacosConfigHandle>,
}

impl<'a> ConfNacosProcessor<'a> {
    pub async fn init(config: &'a ConfCenterConfig, profile: &'a str, app_id: &'a str) -> TardisResult<ConfNacosProcessor<'a>> {
        // let config = &self.config;
        let auth_url = format!("{}/v1/auth/login", config.url);
        let mut params = HashMap::new();
        params.insert("username", &config.username);
        params.insert("password", &config.password);
        let access_token = reqwest::Client::new()
            .post(&auth_url)
            .form(&params)
            .send()
            .await
            .map_err(|error| ConfigError::Foreign(Box::new(error)))?
            .json::<AuthResponse>()
            .await
            .map_err(|error| ConfigError::Foreign(Box::new(error)))?
            .access_token;
        let group = config.group.as_deref().unwrap_or("DEFAULT_GROUP");
        let tenant = config.namespace.as_deref();
        let default_config_handle = ConfNacosConfigHandle {
            base_url: config.url.to_owned(),
            profile: "default".to_owned(),
            app_id: app_id.to_owned(),
            access_token: access_token.clone(),
            tenant: tenant.map(|s| s.to_owned()),
            group: group.to_owned(),
            md5_watcher: None,
        };
        let config_handle = if !profile.is_empty() {
            // let (tx, rx) = tokio::sync::watch::channel(None);
            Some(ConfNacosConfigHandle {
                base_url: config.url.to_owned(),
                profile: profile.to_owned(),
                app_id: app_id.to_owned(),
                access_token,
                tenant: tenant.map(|s| s.to_owned()),
                group: group.to_owned(),
                md5_watcher: None,
            })
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
impl<'a> ConfCenterProcess for ConfNacosProcessor<'a> {
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
    fn get_sources(&mut self, format: config::FileFormat) -> Vec<HttpSource<config::FileFormat>> {
        let default_src = self.default_config_handle.get_http_source(format);
        let mut sources = vec![default_src];
        sources.extend(self.config_handle.as_ref().map(|handle| HttpSource::new(handle.get_url(), format)));
        sources
    }
}

#[derive(Deserialize)]
struct AuthResponse {
    #[serde(rename(deserialize = "accessToken"))]
    pub access_token: String,
}
