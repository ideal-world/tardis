use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use typed_builder::TypedBuilder;
use url::Url;


#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct MailModuleConfig {
    #[builder(setter(into))]
    pub smtp_host: String,
    #[builder(default = 587)]
    pub smtp_port: u16,
    #[builder(setter(into))]
    pub smtp_username: String,
    #[builder(setter(into))]
    pub smtp_password: String,
    #[builder(setter(into))]
    pub default_from: String,
    #[builder(default = false)]
    pub starttls: bool,
}
