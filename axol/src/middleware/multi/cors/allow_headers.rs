use axol_http::{
    header::TypedHeader, request::RequestPartsRef, typed_headers::AccessControlAllowHeaders,
};

use super::Any;

/// Holds configuration for how to set the [`Access-Control-Allow-Headers`][mdn] header.
///
/// See [`CorsLayer::allow_headers`] for more details.
///
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Headers
/// [`CorsLayer::allow_headers`]: super::CorsLayer::allow_headers
#[derive(Clone, Debug)]
#[must_use]
pub enum AllowHeaders {
    Const(Option<String>),
    MirrorRequest,
}

impl Default for AllowHeaders {
    fn default() -> Self {
        Self::Const(None)
    }
}

impl AllowHeaders {
    /// Allow any headers by sending a wildcard (`*`)
    ///
    /// See [`CorsLayer::allow_headers`] for more details.
    ///
    /// [`CorsLayer::allow_headers`]: super::CorsLayer::allow_headers
    pub fn any() -> Self {
        Self::Const(Some("*".to_string()))
    }

    /// Set multiple allowed headers
    ///
    /// See [`CorsLayer::allow_headers`] for more details.
    ///
    /// [`CorsLayer::allow_headers`]: super::CorsLayer::allow_headers
    pub fn list<'a, I: IntoIterator<Item = &'a str>>(headers: I) -> Self {
        let raw = headers.into_iter().collect::<Vec<_>>().join(",");
        if raw.is_empty() {
            return Self::Const(None);
        }
        Self::Const(Some(raw))
    }

    /// Allow any headers, by mirroring the preflight [`Access-Control-Request-Headers`][mdn]
    /// header.
    ///
    /// See [`CorsLayer::allow_headers`] for more details.
    ///
    /// [`CorsLayer::allow_headers`]: super::CorsLayer::allow_headers
    ///
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Request-Headers
    pub fn mirror_request() -> Self {
        Self::MirrorRequest
    }

    pub(super) fn is_wildcard(&self) -> bool {
        matches!(self, AllowHeaders::Const(Some(x)) if x == "*")
    }

    pub(super) fn to_header(
        &self,
        parts: RequestPartsRef<'_>,
    ) -> Option<AccessControlAllowHeaders> {
        match self {
            AllowHeaders::Const(Some(v)) => Some(AccessControlAllowHeaders::decode(v).unwrap()),
            AllowHeaders::Const(None) => None,
            AllowHeaders::MirrorRequest => parts
                .headers
                .get_typed::<AccessControlAllowHeaders>()
                .clone(),
        }
    }
}

impl From<Any> for AllowHeaders {
    fn from(_: Any) -> Self {
        Self::any()
    }
}

impl From<String> for AllowHeaders {
    fn from(arr: String) -> Self {
        Self::Const(Some(arr.into()))
    }
}

impl From<&str> for AllowHeaders {
    fn from(arr: &str) -> Self {
        Self::Const(Some(arr.into()))
    }
}

impl<const N: usize> From<[&str; N]> for AllowHeaders {
    fn from(arr: [&str; N]) -> Self {
        Self::list(arr.into_iter())
    }
}
