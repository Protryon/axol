use axol_http::{
    header::TypedHeader, request::RequestPartsRef, typed_headers::AccessControlAllowMethods, Method,
};

use super::Any;

/// Holds configuration for how to set the [`Access-Control-Allow-Methods`][mdn] header.
///
/// See [`CorsLayer::allow_methods`] for more details.
///
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Methods
/// [`CorsLayer::allow_methods`]: super::CorsLayer::allow_methods
#[derive(Clone, Debug)]
#[must_use]
pub enum AllowMethods {
    Const(Option<String>),
    MirrorRequest,
}

impl Default for AllowMethods {
    fn default() -> Self {
        Self::Const(None)
    }
}

impl AllowMethods {
    /// Allow any method by sending a wildcard (`*`)
    ///
    /// See [`CorsLayer::allow_methods`] for more details.
    ///
    /// [`CorsLayer::allow_methods`]: super::CorsLayer::allow_methods
    pub fn any() -> Self {
        Self::Const(Some("*".to_string()))
    }

    /// Set a single allowed method
    ///
    /// See [`CorsLayer::allow_methods`] for more details.
    ///
    /// [`CorsLayer::allow_methods`]: super::CorsLayer::allow_methods
    pub fn exact(method: Method) -> Self {
        Self::Const(Some(method.to_string()))
    }

    /// Set multiple allowed methods
    ///
    /// See [`CorsLayer::allow_methods`] for more details.
    ///
    /// [`CorsLayer::allow_methods`]: super::CorsLayer::allow_methods
    pub fn list<I: IntoIterator<Item = Method>>(methods: I) -> Self {
        let raw = methods
            .into_iter()
            .map(|x| x.as_str())
            .collect::<Vec<_>>()
            .join(",");
        if raw.is_empty() {
            return Self::Const(None);
        }
        Self::Const(Some(raw))
    }

    /// Allow any method, by mirroring the preflight [`Access-Control-Request-Method`][mdn]
    /// header.
    ///
    /// See [`CorsLayer::allow_methods`] for more details.
    ///
    /// [`CorsLayer::allow_methods`]: super::CorsLayer::allow_methods
    ///
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Request-Method
    pub fn mirror_request() -> Self {
        Self::MirrorRequest
    }

    pub(super) fn is_wildcard(&self) -> bool {
        matches!(self, AllowMethods::Const(Some(x)) if x == "*")
    }

    pub(super) fn to_header(
        &self,
        parts: RequestPartsRef<'_>,
    ) -> Option<AccessControlAllowMethods> {
        match self {
            Self::Const(Some(v)) => Some(AccessControlAllowMethods::decode(v).unwrap()),
            Self::Const(None) => None,
            Self::MirrorRequest => parts
                .headers
                .get_typed::<AccessControlAllowMethods>()
                .clone(),
        }
    }
}

impl From<Any> for AllowMethods {
    fn from(_: Any) -> Self {
        Self::any()
    }
}

impl From<Method> for AllowMethods {
    fn from(method: Method) -> Self {
        Self::exact(method)
    }
}

impl<const N: usize> From<[Method; N]> for AllowMethods {
    fn from(arr: [Method; N]) -> Self {
        Self::list(arr)
    }
}

impl From<Vec<Method>> for AllowMethods {
    fn from(vec: Vec<Method>) -> Self {
        Self::list(vec)
    }
}
