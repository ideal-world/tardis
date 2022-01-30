use std::env;
use std::future::Future;

use testcontainers::clients::Cli;
use testcontainers::images::generic::{GenericImage, WaitFor};
use testcontainers::images::redis::Redis;
use testcontainers::{clients, images, Container, Docker};

use crate::basic::result::TardisResult;

pub struct TardisTestContainer;

impl TardisTestContainer {
    pub async fn redis<F, T>(fun: F) -> TardisResult<()>
    where
        F: Fn(String) -> T + Send + Sync + 'static,
        T: Future<Output = TardisResult<()>> + Send + 'static,
    {
        let docker = clients::Cli::default();
        let node = TardisTestContainer::redis_custom(&docker);
        let port = node.get_host_port(6379).expect("Test port acquisition error");
        fun(format!("redis://127.0.0.1:{}/0", port)).await
    }

    pub fn redis_custom(docker: &Cli) -> Container<Cli, Redis> {
        docker.run(images::redis::Redis::default())
    }

    pub async fn rabbit<F, T>(fun: F) -> TardisResult<()>
    where
        F: Fn(String) -> T + Send + Sync + 'static,
        T: Future<Output = TardisResult<()>> + Send + 'static,
    {
        let docker = clients::Cli::default();
        let node = TardisTestContainer::rabbit_custom(&docker);
        let port = node.get_host_port(5672).expect("Test port acquisition error");
        fun(format!("amqp://guest:guest@127.0.0.1:{}/%2f", port)).await
    }

    pub fn rabbit_custom(docker: &Cli) -> Container<Cli, GenericImage> {
        docker.run(images::generic::GenericImage::new("rabbitmq:management").with_wait_for(WaitFor::message_on_stdout("Server startup complete")))
    }

    pub async fn mysql<F, T>(init_script_path: Option<&str>, fun: F) -> TardisResult<()>
    where
        F: Fn(String) -> T + Send + Sync + 'static,
        T: Future<Output = TardisResult<()>> + Send + 'static,
    {
        let docker = clients::Cli::default();
        let node = TardisTestContainer::mysql_custom(init_script_path, &docker);
        let port = node.get_host_port(3306).expect("Test port acquisition error");
        fun(format!("mysql://root:123456@localhost:{}/test", port)).await
    }

    pub fn mysql_custom<'a>(init_script_path: Option<&str>, docker: &'a Cli) -> Container<'a, Cli, GenericImage> {
        if let Some(init_script_path) = init_script_path {
            let path = env::current_dir().unwrap().join(std::path::Path::new(init_script_path)).to_str().unwrap().to_string();
            docker.run(
                images::generic::GenericImage::new("mysql")
                    .with_env_var("MYSQL_ROOT_PASSWORD", "123456")
                    .with_env_var("MYSQL_DATABASE", "test")
                    .with_volume(path, "/docker-entrypoint-initdb.d/")
                    .with_wait_for(WaitFor::message_on_stderr("port: 3306  MySQL Community Server - GPL")),
            )
        } else {
            docker.run(
                images::generic::GenericImage::new("mysql")
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
        let docker = clients::Cli::default();
        let node = TardisTestContainer::postgres_custom(init_script_path, &docker);
        let port = node.get_host_port(5432).expect("Test port acquisition error");
        fun(format!("postgres://postgres:123456@localhost:{}/test", port)).await
    }

    pub fn postgres_custom<'a>(init_script_path: Option<&str>, docker: &'a Cli) -> Container<'a, Cli, GenericImage> {
        if let Some(init_script_path) = init_script_path {
            let path = env::current_dir().unwrap().join(std::path::Path::new(init_script_path)).to_str().unwrap().to_string();
            docker.run(
                images::generic::GenericImage::new("postgres:alpine")
                    .with_env_var("POSTGRES_PASSWORD", "123456")
                    .with_env_var("POSTGRES_DB", "test")
                    .with_volume(path, "/docker-entrypoint-initdb.d/")
                    .with_wait_for(WaitFor::message_on_stderr("database system is ready to accept connections")),
            )
        } else {
            docker.run(
                images::generic::GenericImage::new("postgres")
                    .with_env_var("POSTGRES_PASSWORD", "123456")
                    .with_env_var("POSTGRES_DB", "test")
                    .with_wait_for(WaitFor::message_on_stderr("database system is ready to accept connections")),
            )
        }
    }
}
