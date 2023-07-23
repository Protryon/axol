use axol_http::{
    header::{HeaderMap, COOKIE, SET_COOKIE},
    response::ResponsePartsRef,
    Body, RequestPartsRef,
};
pub use cookie::{Cookie, Expiration, SameSite};

#[cfg(any(feature = "cookie-signed", feature = "cookie-private"))]
pub use cookie::Key;

use crate::{FromRequest, FromRequestParts, IntoResponseParts, Result};

/// Extractor that grabs cookies from the request and manages the jar.
///
/// Note that methods like [`CookieJar::add`], [`CookieJar::remove`], etc updates the [`CookieJar`]
/// and returns it. This value _must_ be returned from the handler as part of the response for the
/// changes to be propagated.
#[derive(Debug, Default, Clone)]
pub struct CookieJar {
    jar: cookie::CookieJar,
}

#[async_trait::async_trait]
impl<'a> FromRequestParts<'a> for CookieJar {
    async fn from_request_parts(request: RequestPartsRef<'a>) -> Result<Self> {
        Ok(Self::from_headers(&request.headers))
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for CookieJar {
    async fn from_request(request: RequestPartsRef<'a>, _: Body) -> Result<Self> {
        Self::from_request_parts(request).await
    }
}

fn cookies_from_request(headers: &HeaderMap) -> impl Iterator<Item = Cookie<'static>> + '_ {
    headers
        .get_all(COOKIE)
        .into_iter()
        .flat_map(|value| value.split(';'))
        .filter_map(|cookie| Cookie::parse_encoded(cookie.to_owned()).ok())
}

impl CookieJar {
    /// Create a new `CookieJar` from a map of request headers.
    ///
    /// The cookies in `headers` will be added to the jar.
    ///
    /// This is intended to be used in middleware and other places where it might be difficult to
    /// run extractors. Normally you should create `CookieJar`s through [`FromRequestParts`].
    ///
    /// [`FromRequestParts`]: axum::extract::FromRequestParts
    pub fn from_headers(headers: &HeaderMap) -> Self {
        let mut jar = cookie::CookieJar::new();
        for cookie in cookies_from_request(headers) {
            jar.add_original(cookie);
        }
        Self { jar }
    }

    /// Create a new empty `CookieJar`.
    ///
    /// This is inteded to be used in middleware and other places where it might be difficult to
    /// run extractors. Normally you should create `CookieJar`s through [`FromRequestParts`].
    ///
    /// If you need a jar that contains the headers from a request use `impl From<&HeaderMap> for
    /// CookieJar`.
    ///
    /// [`FromRequestParts`]: axum::extract::FromRequestParts
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a cookie from the jar.
    ///
    /// # Example
    ///
    /// ```rust
    /// use axum_extra::extract::cookie::CookieJar;
    /// use axum::response::IntoResponse;
    ///
    /// async fn handle(jar: CookieJar) {
    ///     let value: Option<String> = jar
    ///         .get("foo")
    ///         .map(|cookie| cookie.value().to_owned());
    /// }
    /// ```
    pub fn get(&self, name: &str) -> Option<&Cookie<'static>> {
        self.jar.get(name)
    }

    /// Remove a cookie from the jar.
    ///
    /// # Example
    ///
    /// ```rust
    /// use axum_extra::extract::cookie::{CookieJar, Cookie};
    /// use axum::response::IntoResponse;
    ///
    /// async fn handle(jar: CookieJar) -> CookieJar {
    ///     jar.remove(Cookie::named("foo"))
    /// }
    /// ```
    #[must_use]
    pub fn remove(mut self, cookie: Cookie<'static>) -> Self {
        self.jar.remove(cookie);
        self
    }

    /// Add a cookie to the jar.
    ///
    /// The value will automatically be percent-encoded.
    ///
    /// # Example
    ///
    /// ```rust
    /// use axum_extra::extract::cookie::{CookieJar, Cookie};
    /// use axum::response::IntoResponse;
    ///
    /// async fn handle(jar: CookieJar) -> CookieJar {
    ///     jar.add(Cookie::new("foo", "bar"))
    /// }
    /// ```
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, cookie: Cookie<'static>) -> Self {
        self.jar.add(cookie);
        self
    }

    /// Get an iterator over all cookies in the jar.
    pub fn iter(&self) -> impl Iterator<Item = &'_ Cookie<'static>> {
        self.jar.iter()
    }
}

impl IntoResponseParts for CookieJar {
    fn into_response_parts(self, res: &mut ResponsePartsRef<'_>) -> Result<()> {
        set_cookies(self.jar, &mut res.headers);
        Ok(())
    }
}

fn set_cookies(jar: cookie::CookieJar, headers: &mut HeaderMap) {
    for cookie in jar.delta() {
        headers.append(SET_COOKIE, cookie.encoded().to_string());
    }

    // we don't need to call `jar.reset_delta()` because `into_response_parts` consumes the cookie
    // jar so it cannot be called multiple times.
}
