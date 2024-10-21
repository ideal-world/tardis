use std::env;
use std::future::Future;

use testcontainers::core::Mount;
use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync as Container;
use testcontainers::ImageExt;
use testcontainers_modules::elastic_search::ElasticSearch;
use testcontainers_modules::minio::MinIO;
use testcontainers_modules::mysql::Mysql;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::rabbitmq::RabbitMq;
use testcontainers_modules::redis::Redis;

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;

impl From<testcontainers::TestcontainersError> for TardisError {
    fn from(e: testcontainers::TestcontainersError) -> Self {
        TardisError::internal_error(&e.to_string(), "testcontainers-error")
    }
}

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
            let node = TardisTestContainer::redis_custom().await?;
            let port = node.get_host_port_ipv4(6379).await?;
            fun(format!("redis://127.0.0.1:{port}/0")).await
        }
    }

    pub async fn redis_custom() -> TardisResult<Container<Redis>> {
        let result = Redis::default().start().await?;
        Ok(result)
    }

    pub async fn rabbit<F, T>(fun: F) -> TardisResult<()>
    where
        F: Fn(String) -> T + Send + Sync + 'static,
        T: Future<Output = TardisResult<()>> + Send + 'static,
    {
        if std::env::var_os("TARDIS_TEST_DISABLED_DOCKER").is_some() {
            fun("amqp://guest:guest@127.0.0.1:5672/%2f".to_string()).await
        } else {
            let node = TardisTestContainer::rabbit_custom().await?;
            let port = node.get_host_port_ipv4(5672).await?;
            fun(format!("amqp://guest:guest@127.0.0.1:{port}/%2f")).await
        }
    }

    pub async fn rabbit_custom() -> TardisResult<Container<RabbitMq>> {
        let rabbit_mq = RabbitMq::default().start().await?;
        Ok(rabbit_mq)
    }

    pub async fn mysql<F, T>(init_script_path: Option<&str>, fun: F) -> TardisResult<()>
    where
        F: Fn(String) -> T + Send + Sync + 'static,
        T: Future<Output = TardisResult<()>> + Send + 'static,
    {
        if std::env::var_os("TARDIS_TEST_DISABLED_DOCKER").is_some() {
            fun("mysql://root:123456@127.0.0.1:3306/test".to_string()).await
        } else {
            let node = TardisTestContainer::mysql_custom(init_script_path).await?;
            let port = node.get_host_port_ipv4(3306).await?;
            fun(format!("mysql://root:123456@127.0.0.1:{port}/test")).await
        }
    }

    pub async fn mysql_custom(init_script_path: Option<&str>) -> TardisResult<Container<Mysql>> {
        let mut mysql = Mysql::default().with_env_var("MYSQL_ROOT_PASSWORD", "123456").with_env_var("MYSQL_DATABASE", "test");
        mysql = if let Some(init_script_path) = init_script_path {
            let path = env::current_dir()
                .expect("[Tardis.Test_Container] Current path get error")
                .join(std::path::Path::new(init_script_path))
                .to_str()
                .unwrap_or_else(|| panic!("[Tardis.Test_Container] Script Path [{init_script_path}] get error"))
                .to_string();
            mysql.with_mount(Mount::bind_mount(path, "/docker-entrypoint-initdb.d/"))
        } else {
            mysql
        };
        Ok(mysql.start().await?)
    }

    pub async fn postgres<F, T>(init_script_path: Option<&str>, fun: F) -> TardisResult<()>
    where
        F: Fn(String) -> T + Send + Sync + 'static,
        T: Future<Output = TardisResult<()>> + Send + 'static,
    {
        if std::env::var_os("TARDIS_TEST_DISABLED_DOCKER").is_some() {
            fun("postgres://postgres:123456@127.0.0.1:5432/test".to_string()).await
        } else {
            let node = TardisTestContainer::postgres_custom(init_script_path).await?;
            let port = node.get_host_port_ipv4(5432).await?;
            fun(format!("postgres://postgres:123456@127.0.0.1:{port}/test")).await
        }
    }

    pub async fn postgres_custom(init_script_path: Option<&str>) -> TardisResult<Container<Postgres>> {
        let mut postgres = Postgres::default().with_env_var("POSTGRES_PASSWORD", "123456").with_env_var("POSTGRES_DB", "test");

        postgres = if let Some(init_script_path) = init_script_path {
            let path = env::current_dir()
                .expect("[Tardis.Test_Container] Current path get error")
                .join(std::path::Path::new(init_script_path))
                .to_str()
                .unwrap_or_else(|| panic!("[Tardis.Test_Container] Script Path [{init_script_path}] get error"))
                .to_string();
            postgres.with_mount(Mount::bind_mount(path, "/docker-entrypoint-initdb.d/"))
        } else {
            postgres
        };
        let postgres = postgres.start().await?;
        Ok(postgres)
    }

    pub async fn es<F, T>(fun: F) -> TardisResult<()>
    where
        F: Fn(String) -> T + Send + Sync + 'static,
        T: Future<Output = TardisResult<()>> + Send + 'static,
    {
        if std::env::var_os("TARDIS_TEST_DISABLED_DOCKER").is_some() {
            fun("http://127.0.0.1:9200".to_string()).await
        } else {
            let node = TardisTestContainer::es_custom().await?;
            let port = node.get_host_port_ipv4(9200).await?;
            fun(format!("http://127.0.0.1:{port}")).await
        }
    }

    pub async fn es_custom() -> TardisResult<Container<ElasticSearch>> {
        let es = ElasticSearch::default().with_env_var("ELASTICSEARCH_HEAP_SIZE", "128m").start().await?;
        Ok(es)
    }

    pub async fn minio<F, T>(fun: F) -> TardisResult<()>
    where
        F: Fn(String) -> T + Send + Sync + 'static,
        T: Future<Output = TardisResult<()>> + Send + 'static,
    {
        if std::env::var_os("TARDIS_TEST_DISABLED_DOCKER").is_some() {
            fun("http://127.0.0.1:9000".to_string()).await
        } else {
            let node = TardisTestContainer::minio_custom().await?;
            let port = node.get_host_port_ipv4(9000).await?;
            fun(format!("http://127.0.0.1:{port}")).await
        }
    }

    pub async fn minio_custom() -> TardisResult<Container<MinIO>> {
        let min_io = MinIO::default().start().await?;
        Ok(min_io)
    }
}

pub mod nacos_server;
