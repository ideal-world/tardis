use crossbeam::sync::ShardedLock;
use std::any::TypeId;
use std::hash::Hash;
use std::{
    any::Any,
    borrow::{Borrow, Cow},
    collections::HashMap,
    sync::Arc,
};
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct ComponentKey {
    pub kind: Cow<'static, [u8]>,
    pub key: Cow<'static, [u8]>,
}

impl Borrow<[u8]> for ComponentKey {
    fn borrow(&self) -> &[u8] {
        self.key.as_ref()
    }
}

impl ComponentKey {
    pub const KIND_NAMED: &'static [u8] = b"named";
    pub const KIND_SINGLETON: &'static [u8] = b"singleton";
    pub fn named(name: impl Into<Cow<'static, [u8]>>) -> Self {
        Self {
            key: name.into(),
            kind: Self::KIND_NAMED.into(),
        }
    }

    pub fn singleton<T: Any>() -> Self {
        let key = type_id_to_bytes(TypeId::of::<T>());
        Self {
            key: Cow::Owned(key.to_vec()),
            kind: Cow::Borrowed(Self::KIND_SINGLETON),
        }
    }
}

#[inline(always)]
const fn type_id_to_bytes(id: TypeId) -> [u8; 16] {
    // Safety: TypeId is 16 bytes long.
    unsafe { std::mem::transmute(id) }
}

impl From<&'static str> for ComponentKey {
    fn from(value: &'static str) -> Self {
        Self::named(value.as_bytes())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ComponentStore {
    inner: Arc<ShardedLock<HashMap<ComponentKey, Box<dyn Any>>>>,
}

/// Safety: It'a actually [`Send`], because the restriction.
unsafe impl Send for ComponentStore {}

/// Safety: It'a actually [`Sync`], because the restriction.
unsafe impl Sync for ComponentStore {}

impl ComponentStore {
    pub fn insert<T: Any + Send + Sync + Clone>(&self, key: impl Into<ComponentKey>, value: T) {
        let mut wg = self.inner.write().expect("shouldn't poisoned");
        wg.insert(key.into(), Box::new(value));
    }

    #[inline]
    pub fn insert_singleton<T: Any + Send + Sync + Clone>(&self, value: T) {
        self.insert::<T>(ComponentKey::singleton::<T>(), value);
    }

    pub fn get<T, Q>(&self, key: &Q) -> Option<T>
    where
        T: Any + Send + Sync + Clone,
        ComponentKey: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let rg = self.inner.read().expect("shouldn't poisoned");
        rg.get(key).map(Box::as_ref).and_then(<dyn Any>::downcast_ref).cloned()
    }

    #[inline]
    pub fn get_singleton<T>(&self) -> Option<T>
    where
        T: Any + Send + Sync + Clone,
    {
        self.get::<T, _>(&ComponentKey::singleton::<T>())
    }

    pub fn get_all_of<T>(&self) -> HashMap<ComponentKey, T>
    where
        T: Any + Send + Sync + Clone,
    {
        let rg = self.inner.read().expect("shouldn't poisoned");
        rg.iter()
            .filter_map(|(key, val)| {
                if (**val).type_id() == TypeId::of::<T>() {
                    if let Some(val) = val.as_ref().downcast_ref().cloned() {
                        return Some((key.clone(), val));
                    }
                }
                None
            })
            .collect::<HashMap<ComponentKey, T>>()
    }

    pub fn delete<T, Q>(&self, key: &Q)
    where
        T: Any + Send + Sync + Clone,
        ComponentKey: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let mut wg = self.inner.write().expect("shouldn't poisoned");
        wg.remove(key);
    }

    #[inline]
    pub fn delete_singleton<T: Any + Send + Sync + Clone>(&self) {
        self.delete::<T, _>(&ComponentKey::singleton::<T>())
    }
}
