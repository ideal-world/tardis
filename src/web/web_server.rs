use std::collections::HashMap;

use log::info;
use poem::listener::{Listener, RustlsConfig, TcpListener};
use poem::middleware::{Cors, CorsEndpoint};
use poem::{EndpointExt, Route};
use poem_openapi::{OpenApi, OpenApiService, ServerObject};

use crate::basic::config::{FrameworkConfig, WebServerConfig};
use crate::basic::result::TardisResult;
use crate::web::web_resp::{UniformError, UniformErrorImpl};

pub struct TardisWebServer {
    app_name: String,
    config: WebServerConfig,
    routes: HashMap<String, UniformErrorImpl<CorsEndpoint<Route>>>,
}

impl TardisWebServer {
    pub async fn init_by_conf(conf: &FrameworkConfig) -> TardisResult<TardisWebServer> {
        Ok(TardisWebServer {
            app_name: conf.app.name.clone(),
            config: conf.web_server.clone(),
            routes: HashMap::new(),
        })
    }

    pub fn add_module<T>(&mut self, code: &str, apis: T) -> &mut Self
    where
        T: OpenApi + 'static,
    {
        let module = self.config.modules.iter().find(|m| m.code == code);
        if module.is_none() {
            panic!("[Tardis.WebServer] Module {} not found", code);
        }
        let module = module.unwrap();
        info!("[Tardis.WebServer] Add module {}", module.code);
        let mut api_serv = OpenApiService::new(apis, &module.title, &module.version);
        for (env, url) in &module.doc_urls {
            let url = if !url.ends_with('/') {
                format!("{}/{}", url, module.code)
            } else {
                format!("{}{}", url, module.code)
            };
            api_serv = api_serv.server(ServerObject::new(url).description(env));
        }
        let ui_serv = api_serv.rapidoc();
        let spec_serv = api_serv.spec();
        let mut route = Route::new();
        route = route.nest("/", api_serv);
        if let Some(ui_path) = &module.ui_path {
            route = route.nest(format!("/{}", ui_path), ui_serv);
        }
        if let Some(spec_path) = &module.spec_path {
            route = route.at(format!("/{}", spec_path), poem::endpoint::make_sync(move |_| spec_serv.clone()));
        }
        let cors = if &self.config.allowed_origin == "*" {
            // https://github.com/poem-web/poem/issues/161
            Cors::new()
        } else {
            Cors::new().allow_origin(&self.config.allowed_origin)
        };
        let route = route.with(cors).with(UniformError);
        self.routes.insert(module.code.clone(), route);
        self
    }

    pub async fn start(&'static self) -> TardisResult<()> {
        let output_info = format!(
            r#"
=================
[Tardis.WebServer] The {app} application has been launched. Visited at: {protocol}://{host}:{port}
================="#,
            app = self.app_name,
            host = self.config.host,
            port = self.config.port,
            protocol = if self.config.tls_key.is_some() { "https" } else { "http" }
        );

        let mut routes = Route::new();
        for (code, route) in self.routes.iter() {
            routes = routes.nest(format!("/{}", code), route);
        }
        if self.config.tls_key.is_some() {
            let bind = TcpListener::bind(format!("{}:{}", self.config.host, self.config.port))
                .rustls(RustlsConfig::new().key(self.config.tls_key.clone().unwrap()).cert(self.config.tls_cert.clone().unwrap()));
            let server = poem::Server::new(bind).run(routes);
            info!("{}", output_info);
            server.await?;
        } else {
            let bind = TcpListener::bind(format!("{}:{}", self.config.host, self.config.port));
            let server = poem::Server::new(bind).run(routes);
            info!("{}", output_info);
            server.await?;
        };
        Ok(())
    }
}
