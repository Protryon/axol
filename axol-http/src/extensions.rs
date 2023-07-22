use std::{
    any::{Any, TypeId},
    collections::HashMap,
    hash::{BuildHasherDefault, Hasher},
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug, Default)]
pub struct Extensions {
    inner: Arc<Mutex<ExtensionInner>>,
}

#[derive(Clone, Debug, Default)]
struct ExtensionInner {
    map: HashMap<TypeId, ExtensionItem, BuildHasherDefault<IdHasher>>,
    values: Vec<Option<Arc<dyn Any + Send + Sync>>>,
}

#[derive(Debug, Clone)]
struct ExtensionItem {
    index: usize,
    ever_fetched: bool,
}

pub enum InsertEffect {
    Replaced,
    /// No previous value
    New,
}

pub enum Removed<T> {
    /// Value was fully removed and unshelled
    Removed(T),
    /// Value was fully removed, but could not be unshelled (still referenced?)
    Referenced(Arc<T>),
    /// Value had previously been returned from a `get` call and can no longer be removed.
    Invalidated,
}

impl<T> Removed<T> {
    pub fn unwrap(self) -> T {
        match self {
            Removed::Removed(x) => x,
            Removed::Referenced(_) => panic!("extension is referenced"),
            Removed::Invalidated => panic!("extension is invalidated (was referenced)"),
        }
    }
}

impl Extensions {
    pub fn new() -> Self {
        Default::default()
    }

    /// Inserts a new value into the extension map. It will replace any existing value with the same TypeId.
    /// Note that any outstanding `get`/`get_arc` on the type will not be altered.
    pub fn insert<T: Send + Sync + 'static>(&self, val: T) -> InsertEffect {
        let type_id = TypeId::of::<T>();
        let mut inner = self.inner.lock().unwrap();
        let target_index = inner.values.len();
        let old_index = inner.map.insert(
            type_id,
            ExtensionItem {
                index: target_index,
                ever_fetched: false,
            },
        );
        inner.values.push(Some(Arc::new(val)));
        if old_index.is_some() {
            return InsertEffect::Replaced;
        }
        InsertEffect::New
    }

    /// Gets a reference to an extension value.
    /// This will invalidate that value from ever being manually removed.
    pub fn get<'a, T: Send + Sync + 'static>(&'a self) -> Option<&'a T> {
        let mut inner = self.inner.lock().unwrap();
        let index = inner.map.get_mut(&TypeId::of::<T>())?;
        index.ever_fetched = true;
        let index = index.index;
        // SAFETY: we never remove things from Extensions until its dropped, so the reference to the interior is always valid for self's lifetime
        // Furthermore, we prevent calling `remove` unless a value has **never** been "get"ted before.
        // ... look, I really want to be able to reference this data and remove elements. I know it's overkill.
        let value: &T = (&**inner.values.get(index)?.as_ref()?).downcast_ref()?;
        Some(unsafe { std::mem::transmute(value) })
    }

    /// Gets a reference to an extension value.
    /// Since it returns an `Arc` and tracks it's deallocation, it does not prevent a value from being manually removed.
    /// However, while the `Arc` is alive, it cannot be removed.
    /// Take care that the `Arc` doesn't outlive the Request/Response, otherwise there will be a panic.
    pub fn get_arc<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        let inner = self.inner.lock().unwrap();
        let index = inner.map.get(&TypeId::of::<T>())?;
        let index = index.index;
        let value: Arc<T> = Arc::downcast(inner.values.get(index)?.as_ref()?.clone()).ok()?;
        Some(value)
    }

    /// Removes a non-invalidated entry from the Extensions map
    pub fn remove<T: Send + Sync + 'static>(&self) -> Option<Removed<T>> {
        let mut inner = self.inner.lock().unwrap();
        let index = inner.map.get(&TypeId::of::<T>())?;
        if index.ever_fetched {
            return Some(Removed::Invalidated);
        }
        let index = index.index;
        let value = std::mem::replace(inner.values.get_mut(index)?, None)?;
        let value: Arc<T> = Arc::downcast(value).ok()?;
        match Arc::try_unwrap(value) {
            Ok(x) => Some(Removed::Removed(x)),
            Err(e) => Some(Removed::Referenced(e)),
        }
    }

    pub fn extend(&self, other: Extensions) {
        let mut inner = other.inner.lock().unwrap();
        let mut this = self.inner.lock().unwrap();
        let inner_map = std::mem::take(&mut inner.map);
        for (type_id, index) in inner_map {
            let Some(item) = inner.values.get_mut(index.index) else {
                continue;
            };
            let Some(item) = std::mem::take(item) else {
                continue;
            };
            let ext_item = ExtensionItem {
                index: this.values.len(),
                // the old lifetime is necessarily over since it's being dropped
                ever_fetched: false,
            };
            this.map.insert(type_id, ext_item);
            this.values.push(Some(item));
        }
    }
}

#[derive(Default)]
struct IdHasher(u64);

impl Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("TypeId calls write_u64");
    }

    #[inline]
    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

type AnyMap = HashMap<TypeId, Box<dyn Any + Send + Sync>, BuildHasherDefault<IdHasher>>;
struct HttpExtensions {
    map: Option<Box<AnyMap>>,
}

impl From<http::Extensions> for Extensions {
    fn from(value: http::Extensions) -> Self {
        let value: HttpExtensions = unsafe { std::mem::transmute(value) };
        let mut inner = ExtensionInner {
            map: Default::default(),
            values: Default::default(),
        };
        if let Some(value) = value.map {
            for (type_id, value) in value.into_iter() {
                let item = ExtensionItem {
                    index: inner.values.len(),
                    ever_fetched: false,
                };
                inner.map.insert(type_id, item);
                inner.values.push(Some(Arc::from(value)));
            }
        }
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }
}

// not possible in stable rust atm (converting Arc<T> -> Box<T> while ?Sized)
// maybe possible with lots of assumptions and asm?

// impl Into<http::Extensions> for Extensions {
//     fn into(self) -> http::Extensions {
//         let mut out = http::Extensions::new();
//         let mut inner = self.inner.lock().unwrap();
//         for (type_id, index) in std::mem::take(&mut inner.map) {
//             let Some(item) = inner.values.get_mut(index.index) else {
//                 continue;
//             };
//             let Some(item) = std::mem::take(item) else {
//                 continue;
//             };
//             // item = Arc::try_int
//         }
//         out
//     }
// }
