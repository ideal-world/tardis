use std::{
    borrow::Cow,
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::basic::{json::TardisJson, result::TardisResult};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::hash::Hash;
use tokio::sync::{Mutex, RwLock};

use super::{
    cluster_processor::{peer_count, ClusterEventTarget, TardisClusterMessageReq, TardisClusterSubscriber},
    cluster_publish::{publish_event_no_response, ClusterEvent},
    cluster_receive::listen::Stream,
};

// Cshm = ClusterStaticHashMap
#[derive(Debug, Clone)]
pub struct ClusterStaticHashMap<K, V> {
    pub map: Arc<RwLock<HashMap<K, V>>>,
    pub ident: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum CshmEvent<K, V> {
    Insert(Vec<(K, V)>),
    Remove { keys: Vec<K> },
    Get { key: K },
}

impl<K, V> ClusterStaticHashMap<K, V>
where
    K: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned + Hash + Eq,
    V: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned,
{
    pub fn new(ident: &'static str) -> Self {
        Self {
            map: Arc::new(RwLock::new(HashMap::new())),
            ident,
        }
    }
    pub fn event_name(&self) -> String {
        format!("tardis/hashmap/{ident}", ident = self.ident)
    }
    pub fn local(&self) -> &RwLock<HashMap<K, V>> {
        &self.map
    }
    pub async fn insert(&self, key: K, value: V) -> TardisResult<()> {
        self.map.write().await.insert(key.clone(), value.clone());
        let event = CshmEvent::<K, V>::Insert(vec![(key, value)]);
        let json = TardisJson.obj_to_json(&event)?;
        dbg!(&json);
        let _result = publish_event_no_response(self.event_name(), json, ClusterEventTarget::Broadcast).await;
        Ok(())
    }
    pub async fn batch_insert(&self, pairs: Vec<(K, V)>) -> TardisResult<()> {
        {
            let mut wg = self.map.write().await;
            for (key, value) in pairs.iter() {
                wg.insert(key.clone(), value.clone());
            }
        }
        let event = CshmEvent::<K, V>::Insert(pairs);
        let json = TardisJson.obj_to_json(&event)?;
        let _result = publish_event_no_response(self.event_name(), json, ClusterEventTarget::Broadcast).await;
        Ok(())
    }
    pub async fn remove(&self, key: K) -> TardisResult<()> {
        self.map.write().await.remove(&key);
        let event = CshmEvent::<K, V>::Remove { keys: vec![key] };
        let json = TardisJson.obj_to_json(&event)?;
        let _result = publish_event_no_response(self.event_name(), json, ClusterEventTarget::Broadcast).await;
        Ok(())
    }
    pub async fn batch_remove(&self, keys: Vec<K>) -> TardisResult<()> {
        {
            let mut wg = self.map.write().await;
            for key in keys.iter() {
                wg.remove(key);
            }
        }
        let event = CshmEvent::<K, V>::Remove { keys };
        let json = TardisJson.obj_to_json(&event)?;
        let _result = publish_event_no_response(self.event_name(), json, ClusterEventTarget::Broadcast).await;
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
}

#[async_trait::async_trait]
impl<K, V> TardisClusterSubscriber for ClusterStaticHashMap<K, V>
where
    K: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned + Hash + Eq,
    V: Send + Sync + 'static + Clone + serde::Serialize + serde::de::DeserializeOwned,
{
    async fn subscribe(&self, message: TardisClusterMessageReq) -> TardisResult<Option<Value>> {
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
        }
    }
    fn event_name(&self) -> Cow<'static, str> {
        ClusterStaticHashMap::event_name(self).into()
    }
}
