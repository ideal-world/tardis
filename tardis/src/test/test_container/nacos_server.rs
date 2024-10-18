use std::collections::BTreeMap;
use testcontainers::{core::WaitFor, Image};

macro_rules! def_container {
    {$id:ident {$($env:ident: $tp:ty = $default: expr),*}} => {
        pub struct $id {
            pub tag: String,
            pub env_vars: BTreeMap<String, String>,
        }
        impl $id {
            $(
                pub fn $env(&mut self, $env: $tp) -> &mut Self {
                    self.env_vars.insert(stringify!($env).to_uppercase(), $env.to_string());
                    self
                }
            )*
        }
        impl Default for $id {
            fn default() -> Self {
                let mut s =
                Self {
                    tag: "latest".to_string(),
                    env_vars: BTreeMap::new(),
                };
                s$(.$env($default.into()))*;
                s
            }
        }
    };
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum NacosServerMode {
    Standalone,
    Cluster,
}

impl std::fmt::Display for NacosServerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NacosServerMode::Standalone => write!(f, "standalone"),
            NacosServerMode::Cluster => write!(f, "cluster"),
        }
    }
}

def_container! {
    NacosServer {
        nacos_auth_enable:                  bool            = false,
        mode:                               NacosServerMode = NacosServerMode::Cluster,
        nacos_auth_identity_key:            String          = "nacos",
        nacos_auth_identity_value:          String          = "nacos",
        nacos_auth_token:                   String          = "TARDIS-NACOS-SERVER-TEST-CONTAINER",
        nacos_auth_token_expire_seconds:    usize           = 18000_usize
    }
}

impl Image for NacosServer {
    fn name(&self) -> &str {
        "nacos/nacos-server"
    }

    fn tag(&self) -> &str {
        &self.tag
    }

    fn ready_conditions(&self) -> Vec<testcontainers::core::WaitFor> {
        vec![WaitFor::message_on_stdout("Nacos started successfully")]
    }

    fn env_vars(&self) -> impl IntoIterator<Item = (impl Into<std::borrow::Cow<'_, str>>, impl Into<std::borrow::Cow<'_, str>>)> {
        self.env_vars.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}
