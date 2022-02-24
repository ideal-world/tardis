use std::collections::HashMap;

use redis::aio::Connection;
use redis::{AsyncCommands, RedisError, RedisResult};
use url::Url;

use crate::basic::config::FrameworkConfig;
use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::log::info;

pub struct TardisCacheClient {
    con: Connection,
}

impl TardisCacheClient {
    pub async fn init_by_conf(conf: &FrameworkConfig) -> TardisResult<TardisCacheClient> {
        TardisCacheClient::init(&conf.cache.url).await
    }

    pub async fn init(str_url: &str) -> TardisResult<TardisCacheClient> {
        let url = Url::parse(str_url).unwrap_or_else(|_| panic!("[Tardis.CacheClient] Invalid url {}", str_url));
        info!(
            "[Tardis.CacheClient] Initializing, host:{}, port:{}, db:{}",
            url.host_str().unwrap_or(""),
            url.port().unwrap_or(0),
            if url.path().is_empty() { "" } else { &url.path()[1..] },
        );
        let client = redis::Client::open(str_url)?;
        let con = client.get_tokio_connection().await?;
        info!(
            "[Tardis.CacheClient] Initialized, host:{}, port:{}, db:{}",
            url.host_str().unwrap_or(""),
            url.port().unwrap_or(0),
            if url.path().is_empty() { "" } else { &url.path()[1..] },
        );
        Ok(TardisCacheClient { con })
    }

    // basic operations

    pub async fn set(&mut self, key: &str, value: &str) -> RedisResult<()> {
        self.con.set(key, value).await
    }

    pub async fn set_ex(&mut self, key: &str, value: &str, ex_sec: usize) -> RedisResult<()> {
        self.con.set_ex(key, value, ex_sec).await
    }

    pub async fn set_nx(&mut self, key: &str, value: &str) -> RedisResult<bool> {
        self.con.set_nx(key, value).await
    }

    pub async fn get(&mut self, key: &str) -> RedisResult<Option<String>> {
        self.con.get(key).await
    }

    pub async fn getset(&mut self, key: &str, value: &str) -> RedisResult<Option<String>> {
        self.con.getset(key, value).await
    }

    pub async fn incr(&mut self, key: &str, delta: isize) -> RedisResult<usize> {
        self.con.incr(key, delta).await
    }

    pub async fn del(&mut self, key: &str) -> RedisResult<()> {
        self.con.del(key).await
    }

    pub async fn exists(&mut self, key: &str) -> RedisResult<bool> {
        self.con.exists(key).await
    }

    pub async fn expire(&mut self, key: &str, ex_sec: usize) -> RedisResult<()> {
        self.con.expire(key, ex_sec).await
    }

    pub async fn expire_at(&mut self, key: &str, timestamp_sec: usize) -> RedisResult<()> {
        self.con.expire_at(key, timestamp_sec).await
    }

    pub async fn ttl(&mut self, key: &str) -> RedisResult<usize> {
        self.con.ttl(key).await
    }

    // hash operations

    pub async fn hget(&mut self, key: &str, field: &str) -> RedisResult<Option<String>> {
        self.con.hget(key, field).await
    }

    pub async fn hset(&mut self, key: &str, field: &str, value: &str) -> RedisResult<()> {
        self.con.hset(key, field, value).await
    }

    pub async fn hset_nx(&mut self, key: &str, field: &str, value: &str) -> RedisResult<bool> {
        self.con.hset_nx(key, field, value).await
    }

    pub async fn hdel(&mut self, key: &str, field: &str) -> RedisResult<()> {
        self.con.hdel(key, field).await
    }

    pub async fn hincr(&mut self, key: &str, field: &str, delta: isize) -> RedisResult<usize> {
        self.con.hincr(key, field, delta).await
    }

    pub async fn hexists(&mut self, key: &str, field: &str) -> RedisResult<bool> {
        self.con.hexists(key, field).await
    }

    pub async fn hkeys(&mut self, key: &str) -> RedisResult<Vec<String>> {
        self.con.hkeys(key).await
    }

    pub async fn hvals(&mut self, key: &str) -> RedisResult<Vec<String>> {
        self.con.hvals(key).await
    }

    pub async fn hgetall(&mut self, key: &str) -> RedisResult<HashMap<String, String>> {
        self.con.hgetall(key).await
    }

    pub async fn hlen(&mut self, key: &str) -> RedisResult<usize> {
        self.con.hlen(key).await
    }

    // custom

    pub fn cmd(&mut self) -> &mut Connection {
        &mut self.con
    }
}

impl From<RedisError> for TardisError {
    fn from(error: RedisError) -> Self {
        TardisError::Box(Box::new(error))
    }
}
