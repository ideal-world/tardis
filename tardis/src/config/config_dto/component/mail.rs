use serde::{Deserialize, Serialize};

use typed_builder::TypedBuilder;

use crate::redact::Redact;

/// Mail module configuration / 邮件模块配置
///
#[derive(Serialize, Deserialize, Clone, TypedBuilder)]
#[serde(default)]
pub struct MailModuleConfig {
    /// SMTP host
    #[builder(setter(into), default)]
    pub smtp_host: String,
    /// SMTP port, default by 587
    #[builder(default = 587)]
    pub smtp_port: u16,
    /// SMTP username
    #[builder(setter(into), default)]
    pub smtp_username: String,
    /// SMTP password
    #[builder(setter(into), default)]
    pub smtp_password: String,
    /// default from address
    #[builder(setter(into), default)]
    pub default_from: String,
    /// weather to use STARTTLS, default by false
    #[builder(default = false)]
    pub starttls: bool,
    /// Whether to accept invalid TLS certificates. Defaults to `false`.
    ///
    /// Setting this to `true` disables SMTP TLS certificate verification and should
    /// only be used for local testing; it leaves credentials vulnerable to MITM.
    ///
    /// 是否接受无效 TLS 证书，默认 `false`。仅用于本地测试。
    #[builder(default = false)]
    pub allow_invalid_certs: bool,
    /// Whether to accept invalid TLS hostnames. Defaults to `false`.
    ///
    /// 是否接受无效 TLS 主机名，默认 `false`。仅用于本地测试。
    #[builder(default = false)]
    pub allow_invalid_hostnames: bool,
}

impl std::fmt::Debug for MailModuleConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MailModuleConfig")
            .field("smtp_host", &self.smtp_host)
            .field("smtp_port", &self.smtp_port)
            .field("smtp_username", &self.smtp_username)
            .field("smtp_password", &self.smtp_password.redact())
            .field("default_from", &self.default_from)
            .field("starttls", &self.starttls)
            .field("allow_invalid_certs", &self.allow_invalid_certs)
            .field("allow_invalid_hostnames", &self.allow_invalid_hostnames)
            .finish()
    }
}

impl Default for MailModuleConfig {
    fn default() -> Self {
        MailModuleConfig::builder().build()
    }
}
