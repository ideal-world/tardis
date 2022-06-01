use std::env;
use std::future::Future;

use testcontainers::clients::Cli;
use testcontainers::core::Container;
use testcontainers::core::WaitFor;
use testcontainers::images::generic::GenericImage;
use testcontainers::images::redis::Redis;
use testcontainers::{clients, images};

use crate::basic::result::TardisResult;

pub struct TardisTestContainer;

impl TardisTestContainer {
    pub async fn redis<F, T>(fun: F) -> TardisResult<()>
    where
        F: Fn(String) -> T + Send + Sync + 'static,
        T: Future<Output = TardisResult<()>> + Send + 'static,
    {
        if std::env::var_os("TARDIS_TEST_DISABLED_DOCKER").is_some() {
            fun("redis://127.0.0.1:6379/0".to_string()).await
        } else {
            let docker = clients::Cli::default();
            let node = TardisTestContainer::redis_custom(&docker);
            let port = node.get_host_port_ipv4(6379);
            fun(format!("redis://127.0.0.1:{}/0", port)).await
        }
    }

    pub fn redis_custom(docker: &Cli) -> Container<Redis> {
        docker.run(images::redis::Redis::default())
    }

    pub async fn rabbit<F, T>(fun: F) -> TardisResult<()>
    where
        F: Fn(String) -> T + Send + Sync + 'static,
        T: Future<Output = TardisResult<()>> + Send + 'static,
    {
        if std::env::var_os("TARDIS_TEST_DISABLED_DOCKER").is_some() {
            fun("amqp://guest:guest@127.0.0.1:5672/%2f".to_string()).await
        } else {
            let docker = clients::Cli::default();
            let node = TardisTestContainer::rabbit_custom(&docker);
            let port = node.get_host_port_ipv4(5672);
            fun(format!("amqp://guest:guest@127.0.0.1:{}/%2f", port)).await
        }
    }

    pub fn rabbit_custom(docker: &Cli) -> Container<GenericImage> {
        docker.run(images::generic::GenericImage::new("rabbitmq", "management").with_wait_for(WaitFor::message_on_stdout("Server startup complete")))
    }

    pub async fn mysql<F, T>(init_script_path: Option<&str>, fun: F) -> TardisResult<()>
    where
        F: Fn(String) -> T + Send + Sync + 'static,
        T: Future<Output = TardisResult<()>> + Send + 'static,
    {
        if std::env::var_os("TARDIS_TEST_DISABLED_DOCKER").is_some() {
            fun("mysql://root:123456@localhost:3306/test".to_string()).await
        } else {
            let docker = clients::Cli::default();
            let node = TardisTestContainer::mysql_custom(init_script_path, &docker);
            let port = node.get_host_port_ipv4(3306);
            fun(format!("mysql://root:123456@localhost:{}/test", port)).await
        }
    }

    pub fn mysql_custom<'a>(init_script_path: Option<&str>, docker: &'a Cli) -> Container<'a, GenericImage> {
        if let Some(init_script_path) = init_script_path {
            let path = env::current_dir()
                .expect("[Tardis.Test_Container] Current path get error")
                .join(std::path::Path::new(init_script_path))
                .to_str()
                .unwrap_or_else(|| panic!("[Tardis.Test_Container] Script Path [{}] get error", init_script_path))
                .to_string();
            docker.run(
                images::generic::GenericImage::new("mysql", "8")
                    .with_env_var("MYSQL_ROOT_PASSWORD", "123456")
                    .with_env_var("MYSQL_DATABASE", "test")
                    .with_volume(path, "/docker-entrypoint-initdb.d/")
                    .with_wait_for(WaitFor::message_on_stderr("port: 3306  MySQL Community Server - GPL")),
            )
        } else {
            docker.run(
                images::generic::GenericImage::new("mysql", "8")
                    .with_env_var("MYSQL_ROOT_PASSWORD", "123456")
                    .with_env_var("MYSQL_DATABASE", "test")
                    .with_wait_for(WaitFor::message_on_stderr("port: 3306  MySQL Community Server - GPL")),
            )
        }
    }

    pub async fn postgres<F, T>(init_script_path: Option<&str>, fun: F) -> TardisResult<()>
    where
        F: Fn(String) -> T + Send + Sync + 'static,
        T: Future<Output = TardisResult<()>> + Send + 'static,
    {
        if std::env::var_os("TARDIS_TEST_DISABLED_DOCKER").is_some() {
            fun("postgres://postgres:123456@localhost:5432/test".to_string()).await
        } else {
            let docker = clients::Cli::default();
            let node = TardisTestContainer::postgres_custom(init_script_path, &docker);
            let port = node.get_host_port_ipv4(5432);
            fun(format!("postgres://postgres:123456@localhost:{}/test", port)).await
        }
    }

    pub fn postgres_custom<'a>(init_script_path: Option<&str>, docker: &'a Cli) -> Container<'a, GenericImage> {
        if let Some(init_script_path) = init_script_path {
            let path = env::current_dir()
                .expect("[Tardis.Test_Container] Current path get error")
                .join(std::path::Path::new(init_script_path))
                .to_str()
                .unwrap_or_else(|| panic!("[Tardis.Test_Container] Script Path [{}] get error", init_script_path))
                .to_string();
            docker.run(
                images::generic::GenericImage::new("postgres", "alpine")
                    .with_env_var("POSTGRES_PASSWORD", "123456")
                    .with_env_var("POSTGRES_DB", "test")
                    .with_volume(path, "/docker-entrypoint-initdb.d/")
                    .with_wait_for(WaitFor::message_on_stderr("database system is ready to accept connections")),
            )
        } else {
            docker.run(
                images::generic::GenericImage::new("postgres", "alpine")
                    .with_env_var("POSTGRES_PASSWORD", "123456")
                    .with_env_var("POSTGRES_DB", "test")
                    .with_wait_for(WaitFor::message_on_stderr("database system is ready to accept connections")),
            )
        }
    }

    pub async fn es<F, T>(fun: F) -> TardisResult<()>
    where
        F: Fn(String) -> T + Send + Sync + 'static,
        T: Future<Output = TardisResult<()>> + Send + 'static,
    {
        if std::env::var_os("TARDIS_TEST_DISABLED_DOCKER").is_some() {
            fun("https://elastic:123456@127.0.0.1:9200".to_string()).await
        } else {
            let docker = clients::Cli::default();
            let node = TardisTestContainer::es_custom(&docker);
            let port = node.get_host_port_ipv4(9200);
            fun(format!("https://elastic:123456@127.0.0.1:{}", port)).await
        }
    }

    pub fn es_custom(docker: &Cli) -> Container<GenericImage> {
        docker.run(
            images::generic::GenericImage::new("elasticsearch", "8.1.0")
                .with_env_var("ELASTIC_PASSWORD", "123456")
                .with_env_var("discovery.type", "single-node")
                .with_wait_for(WaitFor::message_on_stdout("successfully loaded geoip database file")),
        )
    }
}
