use std::sync::{atomic::AtomicUsize, Arc, RwLock};
#[derive(Debug)]
struct Version(AtomicUsize);

impl Version {
    fn inc(&self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    fn get(&self) -> usize {
        self.0.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// A source to provides hot data
/// ```rust
/// # use hot_sauce::{HotSource, Hot};
/// let source = HotSource::<str>::new("hello world");
/// let mut hot_str = source.get();
/// source.update("hello hotsauce");
/// assert!(hot_str.is_expired());
/// hot_str.sync();
/// assert!(!hot_str.is_expired());
/// assert_eq!(&*hot_str, "hello hotsauce");
/// ```
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct HotSource<T: ?Sized>(Arc<HotSourceInner<T>>);

impl<T: ?Sized> HotSource<T> {
    pub fn new(data: impl Into<Arc<T>>) -> Self {
        HotSource(HotSourceInner::new(data))
    }
}

impl<T: ?Sized> std::ops::Deref for HotSource<T> {
    type Target = Arc<HotSourceInner<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct HotSourceInner<T: ?Sized> {
    /// version is used to check if the data is expired
    version: Version,
    /// data is the actual data
    data: RwLock<Arc<T>>,
}

impl<T: ?Sized> HotSourceInner<T> {
    /// create a new hot source
    pub fn new(data: impl Into<Arc<T>>) -> Arc<Self> {
        Arc::new(Self {
            version: Version(AtomicUsize::new(0)),
            data: RwLock::new(data.into()),
        })
    }

    /// update value from source
    pub fn update(&self, new_data: impl Into<Arc<T>>) {
        {
            *self.data.write().expect("poisoned") = new_data.into();
        }
        self.version.inc();
    }

    /// get a `Hot` pointer to the data
    pub fn get(self: &Arc<Self>) -> Hot<T> {
        // read version first
        let version = self.version.get();
        let data = { self.data.read().expect("poisoned").clone() };
        Hot {
            version,
            data,
            source: self.clone(),
        }
    }
}

impl<'de, T: ?Sized> serde::Deserialize<'de> for HotSource<T>
where
    T: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = T::deserialize(deserializer)?;
        Ok(Self::new(data))
    }
}

/// A `Hot` pointer is used to wrap a dynamically updated data
#[derive(Debug, Clone)]
pub struct Hot<T: ?Sized> {
    version: usize,
    data: Arc<T>,
    source: Arc<HotSourceInner<T>>,
}

impl<T: ?Sized> Hot<T> {
    /// update the pointee content
    pub fn update(&mut self, new_data: impl Into<Arc<T>>) {
        self.source.update(new_data.into());
        *self = self.source.get();
    }

    /// get the cached data (it may not be the newest value)
    pub fn get(&self) -> Arc<T> {
        self.data.clone()
    }

    /// check if current data has the newest version
    pub fn is_expired(&self) -> bool {
        self.version < self.source.version.get()
    }

    /// sync the cached data to newest version
    pub fn sync(&mut self) -> &mut Self {
        *self = self.source.get();
        self
    }

    /// it's a combination of [#method.sync] and
    pub fn get_sync(&mut self) -> Arc<T> {
        if self.is_expired() {
            self.sync().get_sync()
        } else {
            self.get()
        }
    }
}

impl<T: ?Sized> std::ops::Deref for Hot<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: ?Sized> AsRef<T> for Hot<T> {
    fn as_ref(&self) -> &T {
        &self.data
    }
}

impl<T: ?Sized> serde::Serialize for Hot<T>
where
    T: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        self.get().serialize(serializer)
    }
}