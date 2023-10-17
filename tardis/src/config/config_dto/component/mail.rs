use serde::{Deserialize, Serialize};

use typed_builder::TypedBuilder;

/// Mail module configuration / 邮件模块配置
///
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct MailModuleConfig {
    /// SMTP host
    #[builder(setter(into))]
    pub smtp_host: String,
    /// SMTP port, default by 587
    #[builder(default = 587)]
    pub smtp_port: u16,
    /// SMTP username
    #[builder(setter(into))]
    pub smtp_username: String,
    /// SMTP password
    #[builder(setter(into))]
    pub smtp_password: String,
    /// default from address
    #[builder(setter(into))]
    pub default_from: String,
    /// weather to use STARTTLS, default by false
    #[builder(default = false)]
    pub starttls: bool,
}
