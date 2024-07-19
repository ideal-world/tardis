use std::{
    collections::HashMap,
    fmt,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    basic::{json::TardisJson, result::TardisResult},
    TardisFuns,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::hash::Hash;
use tokio::sync::RwLock;

use super::{
    cluster_processor::{peer_count, ClusterEventTarget, ClusterHandler, TardisClusterMessageReq},
    cluster_publish::{publish_event_no_response, ClusterEvent},
    cluster_receive::listen::Stream,
};

// Cshm = ClusterStaticHashMap
#[derive(Clone)]
pub struct ClusterStaticHashMap<K, V> {
    pub map: Arc<RwLock<HashMap<K, V>>>,
    pub ident: &'static str,
    pub cluster_sync: bool,
    pub modify_handler: Arc<HashMap<String, Box<dyn Fn(&mut V, &Value) + Send + Sync>>>,
}

impl<K, V> fmt::Debug for ClusterStaticHashMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClusterStaticHashMap").field("ident", &self.ident).field("cluster_sync", &self.cluster_sync).finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum CshmEvent<K, V> {
    Insert(Vec<(K, V)>),
    Remove { keys: Vec<K> },
    Get { key: K },
    Modify { key: K, mapper: String, modify: Value },
}

pub struct ClusterStaticHashMapBuilder<K, V> {
    ident: &'static str,
    cluster_sync: bool,
    modify_handler: HashMap<String, Box<dyn Fn(&mut V, &Value) + Send + Sync>>,
    _phantom: std::marker::PhantomData<(K, V)>,
}

impl<K, V> ClusterStaticHashMapBuilder<K, V> {
    pub fn new(ident: &'static str) -> Self {
        Self {
            ident,
            cluster_sync: true,
            modify_handler: HashMap::new(),
            _phantom: std::marker::PhantomData,
        }
    }
    pub fn sync(mut self, cluster_sync: bool) -> Self {
        self.cluster_sync = cluster_sync;
        self
    }
    pub fn modify_handler(mut self, mapper: &'static str, handler: impl Fn(&mut V, &Value) + Send + Sync + 'static) -> Self {
        self.modify_handler.insert(mapper.to_string(), Box::new(handler));
        self
    }
    pub fn build(self) -> ClusterStaticHashMap<K, V> {
        ClusterStaticHashMap {
            map: Arc::new(RwLock::new(HashMap::new())),
            ident: self.ident,
            cluster_sync: self.cluster_sync,
            modify_handler: Arc::new(self.modify_handler),
        }
    }
}

impl<K, V> ClusterStaticHashMap<K, V>
where
    K: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned + Hash + Eq,
    V: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned,
{
    pub fn builder(ident: &'static str) -> ClusterStaticHashMapBuilder<K, V> {
        ClusterStaticHashMapBuilder::new(ident)
    }

    pub fn new(ident: &'static str) -> Self {
        Self {
            map: Arc::new(RwLock::new(HashMap::new())),
            ident,
            cluster_sync: true,
            modify_handler: Arc::new(HashMap::new()),
        }
    }
    pub fn new_standalone(ident: &'static str) -> Self {
        Self {
            map: Arc::new(RwLock::new(HashMap::new())),
            ident,
            cluster_sync: false,
            modify_handler: Arc::new(HashMap::new()),
        }
    }
    pub fn is_cluster(&self) -> bool {
        self.cluster_sync && TardisFuns::fw_config().cluster.is_some()
    }
    pub fn event_name(&self) -> String {
        format!("tardis/hashmap/{ident}", ident = self.ident)
    }
    pub fn local(&self) -> &RwLock<HashMap<K, V>> {
        &self.map
    }
    pub async fn insert(&self, key: K, value: V) -> TardisResult<()> {
        self.map.write().await.insert(key.clone(), value.clone());
        if self.is_cluster() {
            let event = CshmEvent::<K, V>::Insert(vec![(key, value)]);
            let json = TardisJson.obj_to_json(&event)?;
            let event_name = self.event_name();
            tokio::spawn(async move {
                let _result = publish_event_no_response(event_name, json, ClusterEventTarget::Broadcast).await;
            });
        }
        Ok(())
    }
    pub async fn batch_insert(&self, pairs: Vec<(K, V)>) -> TardisResult<()> {
        {
            let mut wg = self.map.write().await;
            for (key, value) in pairs.iter() {
                wg.insert(key.clone(), value.clone());
            }
        }
        if self.is_cluster() {
            let event = CshmEvent::<K, V>::Insert(pairs);
            let json = TardisJson.obj_to_json(&event)?;
            let event_name = self.event_name();
            tokio::spawn(async move {
                let _result = publish_event_no_response(event_name, json, ClusterEventTarget::Broadcast).await;
            });
        }
        Ok(())
    }
    pub async fn remove(&self, key: K) -> TardisResult<()> {
        self.map.write().await.remove(&key);
        if self.is_cluster() {
            let event = CshmEvent::<K, V>::Remove { keys: vec![key] };
            let json = TardisJson.obj_to_json(&event)?;
            let event_name = self.event_name();
            tokio::spawn(async move {
                let _result = publish_event_no_response(event_name, json, ClusterEventTarget::Broadcast).await;
            });
        }
        Ok(())
    }
    pub async fn batch_remove(&self, keys: Vec<K>) -> TardisResult<()> {
        {
            let mut wg = self.map.write().await;
            for key in keys.iter() {
                wg.remove(key);
            }
        }
        if self.is_cluster() {
            let event = CshmEvent::<K, V>::Remove { keys };
            let json = TardisJson.obj_to_json(&event)?;
            let event_name = self.event_name();
            tokio::spawn(async move {
                let _result = publish_event_no_response(event_name, json, ClusterEventTarget::Broadcast).await;
            });
        }
        Ok(())
    }
    pub async fn get(&self, key: K) -> TardisResult<Option<V>> {
        if let Some(v) = self.map.read().await.get(&key) {
            Ok(Some(v.clone()))
        } else {
            self.get_remote(key.clone()).await
        }
    }
    async fn get_remote(&self, key: K) -> TardisResult<Option<V>> {
        if !self.is_cluster() {
            return Ok(None);
        }
        let peer_count = peer_count().await;
        if peer_count == 0 {
            return Ok(None);
        }
        let Ok(mut receiver) = ClusterEvent::new(self.event_name())
            .message(&CshmEvent::<K, V>::Get { key })
            .expect("not valid json value")
            .listener(Stream)
            .target(ClusterEventTarget::Broadcast)
            .publish()
            .await
        else {
            return Ok(None);
        };

        let create_time = Instant::now();
        let mut count = 0;
        while let Some(resp) = receiver.recv().await {
            if let Ok(Some(v)) = TardisJson.json_to_obj::<Option<V>>(resp.msg) {
                return Ok(Some(v));
            }
            count += 1;
            if count >= peer_count {
                return Ok(None);
            }
            if create_time.elapsed() > Duration::from_secs(1) {
                return Ok(None);
            }
        }
        Ok(None)
    }
    pub async fn modify(&self, key: K, mapper: &'static str, modify: Value) -> TardisResult<()> {
        let mapper = mapper.to_string();
        let mut wg = self.map.write().await;
        if let Some(v) = wg.get_mut(&key) {
            if let Some(handler) = self.modify_handler.get(&mapper) {
                handler(v, &modify);
            }
        }
        drop(wg);
        if self.is_cluster() {
            let event = CshmEvent::<K, V>::Modify { key, mapper, modify };
            let json = TardisJson.obj_to_json(&event)?;
            let event_name = self.event_name();
            tokio::spawn(async move {
                let _result = publish_event_no_response(event_name, json, ClusterEventTarget::Broadcast).await;
            });
        }
        Ok(())
    }
}

impl<K, V> ClusterHandler for ClusterStaticHashMap<K, V>
where
    K: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned + Hash + Eq,
    V: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned,
{
    async fn handle(self: Arc<Self>, message: TardisClusterMessageReq) -> TardisResult<Option<Value>> {
        let event: CshmEvent<K, V> = TardisJson.json_to_obj(message.msg)?;
        match event {
            CshmEvent::Insert(pairs) => {
                let mut wg = self.map.write().await;
                for (key, value) in pairs {
                    wg.insert(key, value);
                }
                Ok(None)
            }
            CshmEvent::Remove { keys } => {
                let mut wg = self.map.write().await;
                for key in keys {
                    wg.remove(&key);
                }
                Ok(None)
            }
            CshmEvent::Get { key } => {
                let rg = self.map.read().await;
                let value = rg.get(&key);
                Ok(Some(TardisJson.obj_to_json(&value)?))
            }
            CshmEvent::Modify { key, mapper, modify } => {
                let mut wg = self.map.write().await;
                if let Some(v) = wg.get_mut(&key) {
                    if let Some(handler) = self.modify_handler.get(&mapper) {
                        handler(v, &modify);
                    }
                }
                Ok(None)
            }
        }
    }
    fn event_name(&self) -> String {
        ClusterStaticHashMap::event_name(self)
    }
}
