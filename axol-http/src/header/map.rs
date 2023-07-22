use std::{borrow::Cow, ops::Index};

use http::{header::ToStrError, HeaderName, HeaderValue};
use smallvec::SmallVec;
use thiserror::Error;

use super::{header_name, TypedHeader};

/// This is a multimap representing HTTP headers.
/// Not that this is not a true hashmap, as the count of headers is generally too small to be worth representing as a map.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct HeaderMap {
    items: Vec<(Cow<'static, str>, Cow<'static, str>)>,
}

#[cfg(feature = "otel")]
impl opentelemetry_api::propagation::Extractor for &HeaderMap {
    /// Get a value for a key from the HeaderMap.  If the value is not valid ASCII, returns None.
    fn get(&self, key: &str) -> Option<&str> {
        (&**self).get(key)
    }

    /// Collect all the keys from the HeaderMap.
    fn keys(&self) -> Vec<&str> {
        self.iter().map(|x| x.0).collect()
    }
}

#[cfg(feature = "otel")]
impl opentelemetry_api::propagation::Injector for &mut HeaderMap {
    fn set(&mut self, key: &str, value: String) {
        self.append(key, value);
    }
}

impl HeaderMap {
    /// Create an empty `HeaderMap`.
    ///
    /// The map will be created without any capacity. This function will not
    /// allocate.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// let map = HeaderMap::new();
    ///
    /// assert!(map.is_empty());
    /// assert_eq!(0, map.capacity());
    /// ```
    pub fn new() -> Self {
        Default::default()
    }

    /// Create an empty `HeaderMap` with the specified capacity.
    ///
    /// The returned map will allocate internal storage in order to hold about
    /// `capacity` elements without reallocating. However, this is a "best
    /// effort" as there are usage patterns that could cause additional
    /// allocations before `capacity` headers are stored in the map.
    ///
    /// More capacity than requested may be allocated.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// let map: HeaderMap<u32> = HeaderMap::with_capacity(10);
    ///
    /// assert!(map.is_empty());
    /// assert_eq!(12, map.capacity());
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
        }
    }

    /// Returns the number of headers the map can hold without reallocating.
    ///
    /// This number is an approximation as certain usage patterns could cause
    /// additional allocations before the returned capacity is filled.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::new();
    ///
    /// assert_eq!(0, map.capacity());
    ///
    /// map.insert(HOST, "hello.world".parse().unwrap());
    /// assert_eq!(6, map.capacity());
    /// ```
    pub fn capacity(&self) -> usize {
        self.items.capacity()
    }

    /// Reserves capacity for at least `additional` more headers to be inserted
    /// into the `HeaderMap`.
    ///
    /// The header map may reserve more space to avoid frequent reallocations.
    /// Like with `with_capacity`, this will be a "best effort" to avoid
    /// allocations until `additional` more headers are inserted. Certain usage
    /// patterns could cause additional allocations before the number is
    /// reached.
    ///
    /// # Panics
    ///
    /// Panics if the new allocation size overflows `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::new();
    /// map.reserve(10);
    /// # map.insert(HOST, "bar".parse().unwrap());
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        self.items.reserve(additional);
    }

    /// Clears the map, removing all key-value pairs. Keeps the allocated memory
    /// for reuse.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::new();
    /// map.insert(HOST, "hello.world".parse().unwrap());
    ///
    /// map.clear();
    /// assert!(map.is_empty());
    /// assert!(map.capacity() > 0);
    /// ```
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Returns true if the map contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::new();
    ///
    /// assert!(map.is_empty());
    ///
    /// map.insert(HOST, "hello.world".parse().unwrap());
    ///
    /// assert!(!map.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.items.len() == 0
    }

    /// Returns the number of headers stored in the map.
    ///
    /// This number represents the total number of **values** stored in the map.
    /// This number can be greater than or equal to the number of **keys**
    /// stored given that a single key may have more than one associated value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::{ACCEPT, HOST};
    /// let mut map = HeaderMap::new();
    ///
    /// assert_eq!(0, map.len());
    ///
    /// map.insert(ACCEPT, "text/plain".parse().unwrap());
    /// map.insert(HOST, "localhost".parse().unwrap());
    ///
    /// assert_eq!(2, map.len());
    ///
    /// map.append(ACCEPT, "text/html".parse().unwrap());
    ///
    /// assert_eq!(3, map.len());
    /// ```
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Appends a key-value pair into the map.
    ///
    /// If the map did have this key present, the new value is pushed to the end
    /// of the list of values currently associated with the key. The key is not
    /// updated, though; this matters for types that can be `==` without being
    /// identical.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::new();
    /// assert!(map.insert(HOST, "world".parse().unwrap()).is_none());
    /// assert!(!map.is_empty());
    ///
    /// map.append(HOST, "earth".parse().unwrap());
    ///
    /// let values = map.get_all("host");
    /// let mut i = values.iter();
    /// assert_eq!("world", *i.next().unwrap());
    /// assert_eq!("earth", *i.next().unwrap());
    /// ```
    pub fn append(&mut self, name: impl AsRef<str>, value: impl Into<String>) {
        let name = header_name(name.as_ref());
        self.items.push((name, Cow::Owned(value.into())));
    }

    /// Appends a key-value pair into the map with a static value.
    ///
    /// If the map did have this key present, the new value is pushed to the end
    /// of the list of values currently associated with the key. The key is not
    /// updated, though; this matters for types that can be `==` without being
    /// identical.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::new();
    /// assert!(map.insert(HOST, "world".parse().unwrap()).is_none());
    /// assert!(!map.is_empty());
    ///
    /// map.append(HOST, "earth".parse().unwrap());
    ///
    /// let values = map.get_all("host");
    /// let mut i = values.iter();
    /// assert_eq!("world", *i.next().unwrap());
    /// assert_eq!("earth", *i.next().unwrap());
    /// ```
    pub fn append_static(&mut self, name: impl AsRef<str>, value: impl Into<&'static str>) {
        let name = header_name(name.as_ref());
        self.items.push((name, Cow::Borrowed(value.into())));
    }

    /// Appends a typed key-value pair into the map.
    ///
    /// If the map did have this key present, the new value is pushed to the end
    /// of the list of values currently associated with the key. The key is not
    /// updated, though; this matters for types that can be `==` without being
    /// identical.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::new();
    /// assert!(map.insert(HOST, "world".parse().unwrap()).is_none());
    /// assert!(!map.is_empty());
    ///
    /// map.append(HOST, "earth".parse().unwrap());
    ///
    /// let values = map.get_all("host");
    /// let mut i = values.iter();
    /// assert_eq!("world", *i.next().unwrap());
    /// assert_eq!("earth", *i.next().unwrap());
    /// ```
    pub fn append_typed<H: TypedHeader>(&mut self, header: &H) {
        header.encode(self);
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did have this key present, the new value is associated with
    /// the key and all previous values are removed. **Note** that only a single
    /// one of the previous values is returned. If there are multiple values
    /// that have been previously associated with the key, then the first one is
    /// returned. See `insert_mult` on `OccupiedEntry` for an API that returns
    /// all values.
    ///
    /// The key is not updated, though; this matters for types that can be `==`
    /// without being identical.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::new();
    /// assert!(map.insert(HOST, "world".parse().unwrap()).is_none());
    /// assert!(!map.is_empty());
    ///
    /// let mut prev = map.insert(HOST, "earth".parse().unwrap()).unwrap();
    /// assert_eq!("world", prev);
    /// ```
    pub fn insert(
        &mut self,
        name: impl AsRef<str>,
        value: impl Into<String>,
    ) -> Option<Cow<'static, str>> {
        let name = header_name(name.as_ref());
        match self.get_mut(&name) {
            Some(old) => Some(std::mem::replace(old, Cow::Owned(value.into()))),
            None => {
                self.items.push((name, Cow::Owned(value.into())));
                None
            }
        }
    }

    /// Inserts a key-value pair into the map with a static value.
    ///
    /// If the map did have this key present, the new value is associated with
    /// the key and all previous values are removed. **Note** that only a single
    /// one of the previous values is returned. If there are multiple values
    /// that have been previously associated with the key, then the first one is
    /// returned. See `insert_mult` on `OccupiedEntry` for an API that returns
    /// all values.
    ///
    /// The key is not updated, though; this matters for types that can be `==`
    /// without being identical.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::new();
    /// assert!(map.insert(HOST, "world".parse().unwrap()).is_none());
    /// assert!(!map.is_empty());
    ///
    /// let mut prev = map.insert(HOST, "earth".parse().unwrap()).unwrap();
    /// assert_eq!("world", prev);
    /// ```
    pub fn insert_static(
        &mut self,
        name: impl AsRef<str>,
        value: impl Into<&'static str>,
    ) -> Option<Cow<'static, str>> {
        let name = header_name(name.as_ref());
        match self.get_mut(&name) {
            Some(old) => Some(std::mem::replace(old, Cow::Borrowed(value.into()))),
            None => {
                self.items.push((name, Cow::Borrowed(value.into())));
                None
            }
        }
    }

    /// Inserts a typed key-value pair into the map.
    ///
    /// Note that if the header is a TypedHeader with multiple values returned, only the last value is used.
    ///
    /// If the map did have this key present, the new value is associated with
    /// the key and all previous values are removed. **Note** that only a single
    /// one of the previous values is returned. If there are multiple values
    /// that have been previously associated with the key, then the first one is
    /// returned. See `insert_mult` on `OccupiedEntry` for an API that returns
    /// all values.
    ///
    /// The key is not updated, though; this matters for types that can be `==`
    /// without being identical.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::new();
    /// assert!(map.insert(HOST, "world".parse().unwrap()).is_none());
    /// assert!(!map.is_empty());
    ///
    /// let mut prev = map.insert(HOST, "earth".parse().unwrap()).unwrap();
    /// assert_eq!("world", prev);
    /// ```
    pub fn insert_typed<H: TypedHeader>(&mut self, header: &H) -> Option<Cow<'static, str>> {
        let name = H::name();
        let value = header
            .encode_to_string()
            .into_iter()
            .rev()
            .next()
            .expect("header encoded to empty value");
        match self.get_mut(name) {
            Some(old) => Some(std::mem::replace(old, Cow::Owned(value))),
            None => {
                self.items.push((Cow::Borrowed(name), Cow::Owned(value)));
                None
            }
        }
    }

    /// Returns true if the map contains a value for the specified key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::new();
    /// assert!(!map.contains_key(HOST));
    ///
    /// map.insert(HOST, "world".parse().unwrap());
    /// assert!(map.contains_key("host"));
    /// ```
    pub fn contains_key(&self, name: &str) -> bool {
        self.get_all(name).next().is_some()
    }

    /// Returns a reference to the value associated with the key.
    ///
    /// If there are multiple values associated with the key, then the first one
    /// is returned. Use `get_all` to get all values associated with a given
    /// key. Returns `None` if there are no values associated with the key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::new();
    /// assert!(map.get("host").is_none());
    ///
    /// map.insert(HOST, "hello".parse().unwrap());
    /// assert_eq!(map.get(HOST).unwrap(), &"hello");
    /// assert_eq!(map.get("host").unwrap(), &"hello");
    ///
    /// map.append(HOST, "world".parse().unwrap());
    /// assert_eq!(map.get("host").unwrap(), &"hello");
    /// ```
    pub fn get(&self, name: &str) -> Option<&str> {
        self.items
            .iter()
            .find(|(entry_name, _)| entry_name.eq_ignore_ascii_case(name))
            .map(|x| &*x.1)
    }

    /// Returns a reference to the value associated with the key.
    ///
    /// The key is defined with an associated type referenced a TypedHeader.
    /// If the header is malformed, None is transparently returned
    ///
    /// Note that this causes an allocation depending on the implementation of the TypedHeader.
    ///
    /// If there are multiple values associated with the key, then the first one
    /// is returned. Use `get_all` to get all values associated with a given
    /// key. Returns `None` if there are no values associated with the key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::new();
    /// assert!(map.get::<Host>().is_none());
    ///
    /// map.insert(HOST, "hello".parse().unwrap());
    /// assert_eq!(map.get::<Host>().unwrap(), &"hello");
    ///
    /// map.append(HOST, "world".parse().unwrap());
    /// assert_eq!(map.get::<Host>().unwrap(), &"hello");
    /// ```
    pub fn get_typed<H: TypedHeader>(&self) -> Option<H> {
        let raw = self.get(H::name())?;
        H::decode(raw).ok()
    }

    /// Returns a view of all values associated with a key.
    ///
    /// The returned view does not incur any allocations and allows iterating
    /// the values associated with the key.  See [`GetAll`] for more details.
    /// Returns `None` if there are no values associated with the key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::new();
    ///
    /// map.insert(HOST, "hello".parse().unwrap());
    /// map.append(HOST, "goodbye".parse().unwrap());
    ///
    /// let view = map.get_all("host");
    ///
    /// let mut iter = view.iter();
    /// assert_eq!(&"hello", iter.next().unwrap());
    /// assert_eq!(&"goodbye", iter.next().unwrap());
    /// assert!(iter.next().is_none());
    /// ```
    pub fn get_all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> {
        self.items
            .iter()
            .filter(|(entry_name, _)| entry_name.eq_ignore_ascii_case(name))
            .map(|x| &*x.1)
    }

    /// Returns a view of all values associated with a key.
    ///
    /// The key is defined with an associated type referenced a TypedHeader.
    /// If the header is malformed, it is skipped.
    ///
    /// Note that this causes an allocation for each header value returned depending on the implementation of the TypedHeader.
    ///
    /// The returned view does not incur any allocations and allows iterating
    /// the values associated with the key.  See [`GetAll`] for more details.
    /// Returns `None` if there are no values associated with the key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::new();
    ///
    /// map.insert(HOST, "hello".parse().unwrap());
    /// map.append(HOST, "goodbye".parse().unwrap());
    ///
    /// let view = map.get_all("host");
    ///
    /// let mut iter = view.iter();
    /// assert_eq!(&"hello", iter.next().unwrap());
    /// assert_eq!(&"goodbye", iter.next().unwrap());
    /// assert!(iter.next().is_none());
    /// ```
    pub fn get_all_typed<'a, H: TypedHeader>(&'a self) -> impl Iterator<Item = H> + 'a {
        let name = H::name();
        self.items
            .iter()
            .filter(|(entry_name, _)| entry_name.eq_ignore_ascii_case(name))
            .filter_map(|x| H::decode(&*x.1).ok())
    }

    /// Returns a mutable reference to the value associated with the key.
    ///
    /// If there are multiple values associated with the key, then the first one
    /// is returned. Use `entry` to get all values associated with a given
    /// key. Returns `None` if there are no values associated with the key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::default();
    /// map.insert(HOST, "hello".to_string());
    /// map.get_mut("host").unwrap().push_str("-world");
    ///
    /// assert_eq!(map.get(HOST).unwrap(), &"hello-world");
    /// ```
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Cow<'static, str>> {
        self.items
            .iter_mut()
            .find(|(entry_name, _)| entry_name.eq_ignore_ascii_case(name))
            .map(|x| &mut x.1)
    }

    /// Removes a key from the map, returning the value associated with the key.
    ///
    /// Returns an empty vec if the map does not contain the key. If there are
    /// multiple values associated with the key, then all are returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::HOST;
    /// let mut map = HeaderMap::new();
    /// map.insert(HOST, "hello.world".parse().unwrap());
    ///
    /// let prev = map.remove(HOST);
    /// assert_eq!("hello.world", &prev[0]);
    ///
    /// assert!(map.remove(HOST).is_empty());
    /// ```
    pub fn remove(&mut self, name: impl AsRef<str>) -> Vec<Cow<'static, str>> {
        let name = name.as_ref();
        let mut out = vec![];
        self.items.retain_mut(|(entry_name, value)| {
            if entry_name.eq_ignore_ascii_case(name) {
                out.push(std::mem::take(value));
                false
            } else {
                true
            }
        });
        out
    }

    /// An iterator visiting all key-value pairs.
    ///
    /// The iteration order is in insertion order.
    ///
    /// # Examples
    ///
    /// ```
    /// # use axol_http::HeaderMap;
    /// # use axol_http::header::{CONTENT_LENGTH, HOST};
    /// let mut map = HeaderMap::new();
    ///
    /// map.insert(HOST, "hello".parse().unwrap());
    /// map.append(HOST, "goodbye".parse().unwrap());
    /// map.insert(CONTENT_LENGTH, "123".parse().unwrap());
    ///
    /// for (key, value) in map.iter() {
    ///     println!("{:?}: {:?}", key, value);
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.items.iter().map(|(name, value)| (&**name, &**value))
    }

    pub fn grouped(&self) -> Vec<(&str, SmallVec<[&str; 2]>)> {
        let mut names = self.iter().collect::<Vec<_>>();
        names.sort_by_key(|x| x.0);
        let mut out: Vec<(&str, SmallVec<[&str; 2]>)> = vec![];
        for (name, value) in names {
            if out.last().map(|x| x.0) == Some(name) {
                out.last_mut().unwrap().1.push(value);
            } else {
                out.push((name, smallvec::smallvec![value]))
            }
        }

        out
    }
}

impl<K: Into<Cow<'static, str>>, V: Into<String>> Extend<(K, V)> for HeaderMap {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        for (name, value) in iter {
            let name = name.into();
            self.append(name, value);
        }
    }
}

impl<K: Into<Cow<'static, str>>, V: Into<String>> FromIterator<(K, V)> for HeaderMap {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut out = Self::default();
        out.extend(iter);
        out
    }
}

impl<K: Into<Cow<'static, str>>> Index<K> for HeaderMap {
    type Output = str;

    fn index(&self, index: K) -> &Self::Output {
        self.get(index.into().as_ref()).expect("header missing")
    }
}

impl IntoIterator for HeaderMap {
    type Item = (Cow<'static, str>, Cow<'static, str>);

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a> IntoIterator for &'a HeaderMap {
    type Item = &'a (Cow<'static, str>, Cow<'static, str>);

    type IntoIter = std::slice::Iter<'a, (Cow<'static, str>, Cow<'static, str>)>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<'a> IntoIterator for &'a mut HeaderMap {
    type Item = &'a mut (Cow<'static, str>, Cow<'static, str>);

    type IntoIter = std::slice::IterMut<'a, (Cow<'static, str>, Cow<'static, str>)>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter_mut()
    }
}

#[derive(Error, Debug)]
pub enum HeaderMapConvertError {
    #[error("header value not utf8: '{0}'")]
    Utf8(#[from] ToStrError),
}

impl TryFrom<http::HeaderMap> for HeaderMap {
    type Error = HeaderMapConvertError;

    fn try_from(value: http::HeaderMap) -> Result<Self, Self::Error> {
        let mut out = Self::with_capacity(value.len());
        let mut last_header_name = None::<http::HeaderName>;
        for (name, value) in value.into_iter() {
            let name_ref = match &name {
                Some(x) => x,
                None => last_header_name.as_ref().unwrap(),
            };

            out.append(name_ref.as_str(), value.to_str()?);

            if let Some(name) = name {
                last_header_name = Some(name);
            }
        }
        Ok(out)
    }
}

impl Into<http::HeaderMap> for HeaderMap {
    fn into(self) -> http::HeaderMap {
        self.into_iter()
            .map(|(name, value)| {
                (
                    match name {
                        Cow::Borrowed(x) => HeaderName::from_static(x),
                        Cow::Owned(x) => {
                            HeaderName::from_bytes(x.as_bytes()).expect("invalid header name")
                        }
                    },
                    match value {
                        Cow::Borrowed(x) => HeaderValue::from_static(x),
                        Cow::Owned(x) => {
                            HeaderValue::from_bytes(x.as_bytes()).expect("invalid header value")
                        }
                    },
                )
            })
            .collect()
    }
}
