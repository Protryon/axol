use std::{fmt, sync::Arc, time::Duration};

use axol_http::{request::RequestPartsRef, typed_headers::AccessControlMaxAge};

/// Holds configuration for how to set the [`Access-Control-Max-Age`][mdn] header.
///
/// See [`CorsLayer::max_age`][super::CorsLayer::max_age] for more details.
///
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Max-Age
#[must_use]
#[derive(Clone)]
pub enum MaxAge {
    Exact(Option<Duration>),
    Fn(Arc<dyn for<'a> Fn(&'a str, RequestPartsRef<'a>) -> Duration + Send + Sync + 'static>),
}

impl Default for MaxAge {
    fn default() -> Self {
        Self::Exact(None)
    }
}

impl MaxAge {
    /// Set a static max-age value
    ///
    /// See [`CorsLayer::max_age`][super::CorsLayer::max_age] for more details.
    pub fn exact(max_age: Duration) -> Self {
        Self::Exact(Some(max_age))
    }

    /// Set the max-age based on the preflight request parts
    ///
    /// See [`CorsLayer::max_age`][super::CorsLayer::max_age] for more details.
    pub fn dynamic<F>(f: F) -> Self
    where
        F: Fn(&str, RequestPartsRef<'_>) -> Duration + Send + Sync + 'static,
    {
        Self::Fn(Arc::new(f))
    }

    pub(super) fn to_header(
        &self,
        origin: Option<&str>,
        parts: RequestPartsRef<'_>,
    ) -> Option<AccessControlMaxAge> {
        let max_age = match self {
            Self::Exact(v) => (*v)?,
            Self::Fn(c) => c(origin?, parts),
        };

        Some(max_age.into())
    }
}

impl From<Duration> for MaxAge {
    fn from(max_age: Duration) -> Self {
        Self::exact(max_age)
    }
}

impl fmt::Debug for MaxAge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(arg0) => f.debug_tuple("Exact").field(arg0).finish(),
            Self::Fn(_) => f.debug_tuple("Fn").finish(),
        }
    }
}
