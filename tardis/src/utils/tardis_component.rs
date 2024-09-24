use std::{
    collections::HashMap,
    ops::Deref,
    sync::{Arc, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crate::basic::result::TardisResult;

use super::initializer::InitBy;

type ModuleCode = String;

/// A once-rwlock-arc wrapper, used to store multi-thread shared and once-initialized data
#[repr(transparent)]
#[derive(Default)]
pub struct TardisComponent<T>(OnceLock<TardisComponentInner<T>>);
impl<T> TardisComponent<T> {
    pub const fn new() -> Self {
        Self(OnceLock::new())
    }
}

impl<T: Default> TardisComponent<T> {
    /// Get the inner value if it's initialized, otherwise a `None` is returned.
    pub fn get_option(&self) -> Option<Arc<T>> {
        self.0.get().map(|x| x.get())
    }
}

#[repr(transparent)]
#[derive(Default)]
pub struct TardisComponentInner<T> {
    inner: RwLock<Arc<T>>,
}

impl<T: Default> Deref for TardisComponent<T> {
    type Target = TardisComponentInner<T>;
    fn deref(&self) -> &Self::Target {
        self.0.get_or_init(Default::default)
    }
}

impl<T> TardisComponentInner<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: RwLock::new(Arc::new(value)),
        }
    }

    pub fn set(&self, value: T) {
        match self.inner.write() {
            Ok(mut wg) => {
                *wg = Arc::new(value);
            }
            Err(e) => {
                panic!("TardisComponent encounter an poisoned lock {e}")
            }
        }
    }

    pub fn get(&self) -> Arc<T> {
        let rg = self.inner.read().expect("encounter an poisoned lock when trying to read component");
        (*rg).clone()
    }

    pub fn replace(&self, val: impl Into<Arc<T>>) -> Arc<T> {
        let mut wg = self.inner.write().expect("encounter an poisoned lock when trying to read component");
        std::mem::replace(&mut wg, val.into())
    }
}

/// A once-rwlock-hashmap-arc wrapper, used to store multi-thread shared and once-initialized map
#[repr(transparent)]
pub struct TardisComponentMap<T: ?Sized>(OnceLock<TardisComponentMapInner<T>>);
impl<T: ?Sized> TardisComponentMap<T> {
    pub const fn new() -> Self {
        Self(OnceLock::new())
    }
}

impl<T> Default for TardisComponentMap<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T: ?Sized> Deref for TardisComponentMap<T> {
    type Target = TardisComponentMapInner<T>;
    fn deref(&self) -> &Self::Target {
        self.0.get_or_init(Default::default)
    }
}

type ArcMap<T> = HashMap<ModuleCode, Arc<T>>;
#[repr(transparent)]
pub struct TardisComponentMapInner<T: ?Sized> {
    pub(crate) inner: RwLock<ArcMap<T>>,
}
impl<T: ?Sized> Default for TardisComponentMapInner<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TardisComponentMapInner<T> {
    pub fn extend(&self, iter: impl IntoIterator<Item = (String, T)>) {
        let mut wg = self.inner.write().expect(Self::LOCK_EXPECT);
        wg.extend(iter.into_iter().map(|(k, v)| (k, Arc::new(v))))
    }

    pub fn insert(&self, code: impl Into<ModuleCode>, value: T) -> Option<Arc<T>> {
        let mut wg = self.inner.write().expect(Self::LOCK_EXPECT);
        wg.insert(code.into(), value.into())
    }

    pub fn replace_inner(&self, iter: impl IntoIterator<Item = (String, T)>) -> ArcMap<T> {
        let mut wg = self.inner.write().expect(Self::LOCK_EXPECT);
        let new_inner = iter.into_iter().map(|(k, v)| (k, Arc::new(v))).collect::<HashMap<_, _>>();
        std::mem::replace(&mut wg, new_inner)
    }

    pub fn drain(&self) -> ArcMap<T> {
        self.replace_inner(std::iter::empty())
    }

    /// Initialize by an [`ArcMap`] initializer.
    ///
    /// this method will clear the current map and replace it with the new one witch is created from the initializer.
    pub async fn init_by<I>(&self, initializer: &I) -> TardisResult<ArcMap<T>>
    where
        ArcMap<T>: InitBy<I>,
    {
        self.clear();
        let new_inner = HashMap::<ModuleCode, Arc<T>>::init_by(initializer).await?;
        let wg = &mut *self.inner.write().expect(Self::LOCK_EXPECT);
        Ok(std::mem::replace(wg, new_inner))
    }
}

impl<T: ?Sized> TardisComponentMapInner<T> {
    const LOCK_EXPECT: &'static str = "encounter an poisoned lock when trying to lock component";

    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert_arc(&self, code: impl Into<ModuleCode>, value: Arc<T>) -> Option<Arc<T>> {
        let mut wg = self.inner.write().expect(Self::LOCK_EXPECT);
        (*wg).insert(code.into(), value)
    }

    pub fn extend_arc(&self, iter: impl IntoIterator<Item = (String, Arc<T>)>) {
        let mut wg = self.inner.write().expect(Self::LOCK_EXPECT);
        wg.extend(iter)
    }

    pub fn get(&self, code: &str) -> Option<Arc<T>> {
        let rg = self.inner.read().expect(Self::LOCK_EXPECT);
        (*rg).get(code).cloned()
    }

    pub fn remove(&self, code: &str) -> Option<Arc<T>> {
        let mut wg = self.inner.write().expect(Self::LOCK_EXPECT);
        wg.remove(code)
    }

    pub fn clear(&self) {
        let mut wg = self.inner.write().expect(Self::LOCK_EXPECT);
        wg.clear()
    }

    pub fn contains_key(&self, code: &str) -> bool {
        let rg = self.inner.read().expect(Self::LOCK_EXPECT);
        rg.contains_key(code)
    }

    /// # Panic
    /// Panic if the lock is poisoned
    pub fn read(&self) -> RwLockReadGuard<'_, ArcMap<T>> {
        self.inner.read().expect(Self::LOCK_EXPECT)
    }

    /// # Panic
    /// Panic if the lock is poisoned
    pub fn write(&self) -> RwLockWriteGuard<'_, ArcMap<T>> {
        self.inner.write().expect(Self::LOCK_EXPECT)
    }
}
