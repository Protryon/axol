use std::{fmt, sync::Arc};

use axol_http::{
    header::TypedHeader, request::RequestPartsRef, typed_headers::AccessControlAllowOrigin,
};

use super::Any;

/// Holds configuration for how to set the [`Access-Control-Allow-Origin`][mdn] header.
///
/// See [`CorsLayer::allow_origin`] for more details.
///
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Origin
/// [`CorsLayer::allow_origin`]: super::CorsLayer::allow_origin
#[derive(Clone)]
#[must_use]
pub enum AllowOrigin {
    Const(String),
    List(Vec<String>),
    Predicate(Arc<dyn for<'a> Fn(&'a str, RequestPartsRef<'a>) -> bool + Send + Sync + 'static>),
}

impl Default for AllowOrigin {
    fn default() -> Self {
        Self::List(Vec::new())
    }
}

impl AllowOrigin {
    /// Allow any origin by sending a wildcard (`*`)
    ///
    /// See [`CorsLayer::allow_origin`] for more details.
    ///
    /// [`CorsLayer::allow_origin`]: super::CorsLayer::allow_origin
    pub fn any() -> Self {
        Self::Const("*".to_string())
    }

    /// Set a single allowed origin
    ///
    /// See [`CorsLayer::allow_origin`] for more details.
    ///
    /// [`CorsLayer::allow_origin`]: super::CorsLayer::allow_origin
    pub fn exact(origin: impl Into<String>) -> Self {
        Self::Const(origin.into())
    }

    /// Set multiple allowed origins
    ///
    /// See [`CorsLayer::allow_origin`] for more details.
    ///
    /// # Panics
    ///
    /// If the iterator contains a wildcard (`*`).
    ///
    /// [`CorsLayer::allow_origin`]: super::CorsLayer::allow_origin
    pub fn list<S: Into<String>, I: IntoIterator<Item = S>>(origins: I) -> Self {
        let raw = origins.into_iter().map(|x| {
            let x = x.into();
            if x == "*" {
                panic!("Wildcard origin (`*`) cannot be passed to `AllowOrigin::list`. Use `AllowOrigin::any()` instead");
            }
            x
        }).collect::<Vec<_>>();
        Self::List(raw)
    }

    /// Set the allowed origins from a predicate
    ///
    /// See [`CorsLayer::allow_origin`] for more details.
    ///
    /// [`CorsLayer::allow_origin`]: super::CorsLayer::allow_origin
    pub fn predicate<F>(f: F) -> Self
    where
        F: Fn(&str, RequestPartsRef<'_>) -> bool + Send + Sync + 'static,
    {
        AllowOrigin::Predicate(Arc::new(f))
    }

    /// Allow any origin, by mirroring the request origin
    ///
    /// This is equivalent to
    /// [`AllowOrigin::predicate(|_, _| true)`][Self::predicate].
    ///
    /// See [`CorsLayer::allow_origin`] for more details.
    ///
    /// [`CorsLayer::allow_origin`]: super::CorsLayer::allow_origin
    pub fn mirror_request() -> Self {
        Self::predicate(|_, _| true)
    }

    #[allow(clippy::borrow_interior_mutable_const)]
    pub(super) fn is_wildcard(&self) -> bool {
        matches!(self, AllowOrigin::Const(x) if x == "*")
    }

    pub(super) fn to_header(
        &self,
        origin: Option<&str>,
        parts: RequestPartsRef<'_>,
    ) -> Option<AccessControlAllowOrigin> {
        match self {
            Self::Const(v) => Some(AccessControlAllowOrigin::decode(v).unwrap()),
            Self::List(list) => origin
                .filter(|o| list.iter().any(|x| x == *o))
                .map(|x| AccessControlAllowOrigin::decode(x).unwrap()),
            Self::Predicate(predicate) => origin
                .filter(|origin| predicate(origin, parts))
                .map(|x| AccessControlAllowOrigin::decode(x).unwrap()),
        }
    }
}

impl From<Any> for AllowOrigin {
    fn from(_: Any) -> Self {
        Self::any()
    }
}

impl fmt::Debug for AllowOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Const(arg0) => f.debug_tuple("Const").field(arg0).finish(),
            Self::List(arg0) => f.debug_tuple("List").field(arg0).finish(),
            Self::Predicate(_) => f.debug_tuple("Predicate").finish(),
        }
    }
}

impl From<String> for AllowOrigin {
    fn from(arr: String) -> Self {
        Self::Const(arr.into())
    }
}

impl From<&str> for AllowOrigin {
    fn from(arr: &str) -> Self {
        Self::Const(arr.into())
    }
}

impl<const N: usize> From<[&str; N]> for AllowOrigin {
    fn from(arr: [&str; N]) -> Self {
        Self::list(arr.into_iter())
    }
}
