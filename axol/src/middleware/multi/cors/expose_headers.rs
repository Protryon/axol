use axol_http::{
    header::TypedHeader, request::RequestPartsRef, typed_headers::AccessControlExposeHeaders,
};

use super::Any;

/// Holds configuration for how to set the [`Access-Control-Expose-Headers`][mdn] header.
///
/// See [`CorsLayer::expose_headers`] for more details.
///
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Expose-Headers
/// [`CorsLayer::expose_headers`]: super::CorsLayer::expose_headers
#[derive(Clone, Default, Debug)]
#[must_use]
pub struct ExposeHeaders(pub Option<String>);

impl ExposeHeaders {
    /// Expose any / all headers by sending a wildcard (`*`)
    ///
    /// See [`CorsLayer::expose_headers`] for more details.
    ///
    /// [`CorsLayer::expose_headers`]: super::CorsLayer::expose_headers
    pub fn any() -> Self {
        Self(Some("*".to_string()))
    }

    /// Set multiple exposed header names
    ///
    /// See [`CorsLayer::expose_headers`] for more details.
    ///
    /// [`CorsLayer::expose_headers`]: super::CorsLayer::expose_headers
    pub fn list<'a, I: IntoIterator<Item = &'a str>>(methods: I) -> Self {
        let raw = methods.into_iter().collect::<Vec<_>>().join(",");
        if raw.is_empty() {
            return Self(None);
        }
        Self(Some(raw))
    }

    pub(super) fn is_wildcard(&self) -> bool {
        matches!(self, Self(Some(x)) if x == "*")
    }

    pub(super) fn to_header(
        &self,
        _parts: RequestPartsRef<'_>,
    ) -> Option<AccessControlExposeHeaders> {
        self.0
            .as_ref()
            .map(|x| AccessControlExposeHeaders::decode(x).unwrap())
    }
}

impl From<Any> for ExposeHeaders {
    fn from(_: Any) -> Self {
        Self::any()
    }
}

impl<const N: usize> From<[&str; N]> for ExposeHeaders {
    fn from(arr: [&str; N]) -> Self {
        Self::list(arr)
    }
}

impl From<Vec<&str>> for ExposeHeaders {
    fn from(vec: Vec<&str>) -> Self {
        Self::list(vec)
    }
}
