//! Common DTOs / 常用的DTO
use std::{collections::HashMap, fmt, pin::Pin, sync::Arc};

use tokio::sync::{Mutex, RwLock};
use tracing::error;

use crate::{serde::{Deserialize, Serialize}, TardisFuns};

use super::result::TardisResult;

type SyncFn = dyn FnOnce() -> Pin<Box<dyn std::future::Future<Output = TardisResult<()>> + Send + Sync>> + Send + 'static;
type AsyncFn = dyn FnOnce() -> Pin<Box<dyn std::future::Future<Output = TardisResult<()>> + Send + Sync>> + Send + 'static;
/// Tardis context / Tardis上下文
///
/// Used to bring in some authentication information when a web request is received.
///
/// 用于Web请求时带入一些认证信息.
///
/// This information needs to be supported by the IAM service.
///
/// 该信息需要与 IAM 服务对应.
///
#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct TardisContext {
    /// The requested own paths / 请求的所属路径
    pub own_paths: String,
    /// The requested Ak / 请求的Ak
    pub ak: String,
    /// The requested owner/ 请求的所属者
    pub owner: String,
    /// List of requested role ids / 请求的角色Id列表
    pub roles: Vec<String>,
    /// List of requested group ids / 请求的群组Id列表
    pub groups: Vec<String>,
    /// Extension information / 扩展信息
    #[serde(skip)]
    pub ext: Arc<RwLock<HashMap<String, String>>>,
    /// Synchronous task method in context / 上下文中的同步任务方法
    /// ```ignore
    /// let _ = ctx
    ///     .add_sync_task(Box::new(|| {
    ///         Box::pin(async move {
    ///             println!("Starting sync background task");
    ///             sleep(Duration::from_secs(1)).await;
    ///             println!("Finished sync background task");
    ///             Ok(())
    ///         })
    ///     }))
    ///     .await;
    /// ```
    #[serde(skip)]
    pub sync_task_fns: Arc<Mutex<Vec<Box<SyncFn>>>>,
    /// Asynchronous task method in context / 上下文中的异步任务方法
    /// ```ignore
    ///let _ = ctx
    ///     .add_async_task(Box::new(|| {
    ///         Box::pin(async move {
    ///             println!("Starting async background task");
    ///             sleep(Duration::from_secs(1)).await;
    ///             println!("Finished async background task");
    ///             Ok(())
    ///         })
    ///     }))
    ///     .await;
    /// ```
    #[serde(skip)]
    pub async_task_fns: Arc<Mutex<Vec<Box<AsyncFn>>>>,
}

impl fmt::Debug for TardisContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TardisContext")
            .field("own_paths", &self.own_paths)
            .field("ak", &self.ak)
            .field("roles", &self.roles)
            .field("groups", &self.groups)
            .field("ext", &self.ext)
            .finish()
    }
}

impl Default for TardisContext {
    fn default() -> Self {
        TardisContext {
            own_paths: "".to_string(),
            ak: "".to_string(),
            owner: "".to_string(),
            roles: vec![],
            groups: vec![],
            ext: Default::default(),
            sync_task_fns: Default::default(),
            async_task_fns: Default::default(),
        }
    }
}

impl TardisContext {
    pub fn to_json(&self) -> TardisResult<String> {
        TardisFuns::json.obj_to_string(self)
    }
    
    pub fn to_base64(&self) -> TardisResult<String> {
        let ctx = TardisContext::default();
        let ctx = TardisFuns::json.obj_to_string(&ctx)?;
        Ok(TardisFuns::crypto.base64.encode(ctx))
    }

    pub async fn add_ext(&self, key: &str, value: &str) -> TardisResult<()> {
        self.ext.write().await.insert(key.to_string(), value.to_string());
        Ok(())
    }

    pub async fn remove_ext(&self, key: &str) -> TardisResult<()> {
        self.ext.write().await.remove(key);
        Ok(())
    }

    pub async fn get_ext(&self, key: &str) -> TardisResult<Option<String>> {
        Ok(self.ext.read().await.get(key).cloned())
    }

    pub async fn add_sync_task(&self, task: Box<SyncFn>) -> TardisResult<()> {
        self.sync_task_fns.lock().await.push(task);
        Ok(())
    }

    pub async fn add_async_task(&self, task: Box<AsyncFn>) -> TardisResult<()> {
        self.async_task_fns.lock().await.push(task);
        Ok(())
    }

    pub async fn execute_task(&self) -> TardisResult<()> {
        let mut sync_task_fns = self.sync_task_fns.lock().await;
        while let Some(sync_task_fn) = sync_task_fns.pop() {
            let result = sync_task_fn().await;
            match result {
                Ok(_) => {}
                Err(e) => {
                    error!("Sync task process error:{:?}", e);
                }
            }
        }
        let mut async_task_fns = self.async_task_fns.lock().await;
        while let Some(async_task_fn) = async_task_fns.pop() {
            tokio::spawn(async move {
                let result = async_task_fn().await;
                match result {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Async task process error:{:?}", e);
                    }
                }
            });
        }
        Ok(())
    }
}
