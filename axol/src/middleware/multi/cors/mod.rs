//! Middleware which adds headers for [CORS][mdn].
//!
//! # Example
//!
//! ```
//! use http::{Request, Response, Method, header};
//! use hyper::Body;
//! use tower::{ServiceBuilder, ServiceExt, Service};
//! use tower_http::cors::{Any, Cors};
//! use std::convert::Infallible;
//!
//! async fn handle(request: Request<Body>) -> Result<Response<Body>, Infallible> {
//!     Ok(Response::new(Body::empty()))
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let cors = Cors::new()
//!     // allow `GET` and `POST` when accessing the resource
//!     .allow_methods([Method::GET, Method::POST])
//!     // allow requests from any origin
//!     .allow_origin(Any);
//!
//! let mut service = ServiceBuilder::new()
//!     .layer(cors)
//!     .service_fn(handle);
//!
//! let request = Request::builder()
//!     .header(header::ORIGIN, "https://example.com")
//!     .body(Body::empty())
//!     .unwrap();
//!
//! let response = service
//!     .ready()
//!     .await?
//!     .call(request)
//!     .await?;
//!
//! assert_eq!(
//!     response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(),
//!     "*",
//! );
//! # Ok(())
//! # }
//! ```
//!
//! [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS

#![allow(clippy::enum_variant_names)]
use axol_http::{
    header::HeaderMap,
    request::{RequestParts, RequestPartsRef},
    response::Response,
    Method,
};

mod allow_credentials;
mod allow_headers;
mod allow_methods;
mod allow_origin;
mod allow_private_network;
mod expose_headers;
mod max_age;
mod vary;

use crate::{Error, Extension, FromRequestParts, Plugin, Result, Router};

pub use self::{
    allow_credentials::AllowCredentials, allow_headers::AllowHeaders, allow_methods::AllowMethods,
    allow_origin::AllowOrigin, allow_private_network::AllowPrivateNetwork,
    expose_headers::ExposeHeaders, max_age::MaxAge, vary::Vary,
};

/// Layer that applies the [`Cors`] middleware which adds headers for [CORS][mdn].
///
/// See the [module docs](crate::cors) for an example.
///
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS
#[derive(Debug, Clone)]
#[must_use]
pub struct Cors {
    allow_credentials: AllowCredentials,
    allow_headers: AllowHeaders,
    allow_methods: AllowMethods,
    allow_origin: AllowOrigin,
    allow_private_network: AllowPrivateNetwork,
    expose_headers: ExposeHeaders,
    max_age: MaxAge,
    vary: Vary,
}

impl Cors {
    /// Create a new `Cors`.
    ///
    /// No headers are sent by default. Use the builder methods to customize
    /// the behavior.
    ///
    /// You need to set at least an allowed origin for browsers to make
    /// successful cross-origin requests to your service.
    pub fn new() -> Self {
        Self {
            allow_credentials: Default::default(),
            allow_headers: Default::default(),
            allow_methods: Default::default(),
            allow_origin: Default::default(),
            allow_private_network: Default::default(),
            expose_headers: Default::default(),
            max_age: Default::default(),
            vary: Default::default(),
        }
    }

    /// A permissive configuration:
    ///
    /// - All request headers allowed.
    /// - All methods allowed.
    /// - All origins allowed.
    /// - All headers exposed.
    pub fn permissive() -> Self {
        Self::new()
            .allow_headers(Any)
            .allow_methods(Any)
            .allow_origin(Any)
            .expose_headers(Any)
    }

    /// A very permissive configuration:
    ///
    /// - **Credentials allowed.**
    /// - The method received in `Access-Control-Request-Method` is sent back
    ///   as an allowed method.
    /// - The origin of the preflight request is sent back as an allowed origin.
    /// - The header names received in `Access-Control-Request-Headers` are sent
    ///   back as allowed headers.
    /// - No headers are currently exposed, but this may change in the future.
    pub fn very_permissive() -> Self {
        Self::new()
            .allow_credentials(true)
            .allow_headers(AllowHeaders::mirror_request())
            .allow_methods(AllowMethods::mirror_request())
            .allow_origin(AllowOrigin::mirror_request())
    }

    /// Set the [`Access-Control-Allow-Credentials`][mdn] header.
    ///
    /// ```
    /// use tower_http::cors::Cors;
    ///
    /// let layer = Cors::new().allow_credentials(true);
    /// ```
    ///
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Credentials
    pub fn allow_credentials<T>(mut self, allow_credentials: T) -> Self
    where
        T: Into<AllowCredentials>,
    {
        self.allow_credentials = allow_credentials.into();
        self
    }

    /// Set the value of the [`Access-Control-Allow-Headers`][mdn] header.
    ///
    /// ```
    /// use tower_http::cors::Cors;
    /// use http::header::{AUTHORIZATION, ACCEPT};
    ///
    /// let layer = Cors::new().allow_headers([AUTHORIZATION, ACCEPT]);
    /// ```
    ///
    /// All headers can be allowed with
    ///
    /// ```
    /// use tower_http::cors::{Any, Cors};
    ///
    /// let layer = Cors::new().allow_headers(Any);
    /// ```
    ///
    /// Note that multiple calls to this method will override any previous
    /// calls.
    ///
    /// Also note that `Access-Control-Allow-Headers` is required for requests that have
    /// `Access-Control-Request-Headers`.
    ///
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Headers
    pub fn allow_headers<T>(mut self, headers: T) -> Self
    where
        T: Into<AllowHeaders>,
    {
        self.allow_headers = headers.into();
        self
    }

    /// Set the value of the [`Access-Control-Max-Age`][mdn] header.
    ///
    /// ```
    /// use std::time::Duration;
    /// use tower_http::cors::Cors;
    ///
    /// let layer = Cors::new().max_age(Duration::from_secs(60) * 10);
    /// ```
    ///
    /// By default the header will not be set which disables caching and will
    /// require a preflight call for all requests.
    ///
    /// Note that each browser has a maximum internal value that takes
    /// precedence when the Access-Control-Max-Age is greater. For more details
    /// see [mdn].
    ///
    /// If you need more flexibility, you can use supply a function which can
    /// dynamically decide the max-age based on the origin and other parts of
    /// each preflight request:
    ///
    /// ```
    /// # struct MyServerConfig { cors_max_age: Duration }
    /// use std::time::Duration;
    ///
    /// use http::{request::Parts as RequestParts, HeaderValue};
    /// use tower_http::cors::{Cors, MaxAge};
    ///
    /// let layer = Cors::new().max_age(MaxAge::dynamic(
    ///     |_origin: &HeaderValue, parts: &RequestParts| -> Duration {
    ///         // Let's say you want to be able to reload your config at
    ///         // runtime and have another middleware that always inserts
    ///         // the current config into the request extensions
    ///         let config = parts.extensions.get::<MyServerConfig>().unwrap();
    ///         config.cors_max_age
    ///     },
    /// ));
    /// ```
    ///
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Max-Age
    pub fn max_age<T>(mut self, max_age: T) -> Self
    where
        T: Into<MaxAge>,
    {
        self.max_age = max_age.into();
        self
    }

    /// Set the value of the [`Access-Control-Allow-Methods`][mdn] header.
    ///
    /// ```
    /// use tower_http::cors::Cors;
    /// use http::Method;
    ///
    /// let layer = Cors::new().allow_methods([Method::GET, Method::POST]);
    /// ```
    ///
    /// All methods can be allowed with
    ///
    /// ```
    /// use tower_http::cors::{Any, Cors};
    ///
    /// let layer = Cors::new().allow_methods(Any);
    /// ```
    ///
    /// Note that multiple calls to this method will override any previous
    /// calls.
    ///
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Methods
    pub fn allow_methods<T>(mut self, methods: T) -> Self
    where
        T: Into<AllowMethods>,
    {
        self.allow_methods = methods.into();
        self
    }

    /// Set the value of the [`Access-Control-Allow-Origin`][mdn] header.
    ///
    /// ```
    /// use http::HeaderValue;
    /// use tower_http::cors::Cors;
    ///
    /// let layer = Cors::new().allow_origin(
    ///     "http://example.com".parse::<HeaderValue>().unwrap(),
    /// );
    /// ```
    ///
    /// Multiple origins can be allowed with
    ///
    /// ```
    /// use tower_http::cors::Cors;
    ///
    /// let origins = [
    ///     "http://example.com".parse().unwrap(),
    ///     "http://api.example.com".parse().unwrap(),
    /// ];
    ///
    /// let layer = Cors::new().allow_origin(origins);
    /// ```
    ///
    /// All origins can be allowed with
    ///
    /// ```
    /// use tower_http::cors::{Any, Cors};
    ///
    /// let layer = Cors::new().allow_origin(Any);
    /// ```
    ///
    /// You can also use a closure
    ///
    /// ```
    /// use tower_http::cors::{Cors, AllowOrigin};
    /// use http::{request::Parts as RequestParts, HeaderValue};
    ///
    /// let layer = Cors::new().allow_origin(AllowOrigin::predicate(
    ///     |origin: &HeaderValue, _request_parts: &RequestParts| {
    ///         origin.as_bytes().ends_with(b".rust-lang.org")
    ///     },
    /// ));
    /// ```
    ///
    /// Note that multiple calls to this method will override any previous
    /// calls.
    ///
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Origin
    pub fn allow_origin<T>(mut self, origin: T) -> Self
    where
        T: Into<AllowOrigin>,
    {
        self.allow_origin = origin.into();
        self
    }

    /// Set the value of the [`Access-Control-Expose-Headers`][mdn] header.
    ///
    /// ```
    /// use tower_http::cors::Cors;
    /// use http::header::CONTENT_ENCODING;
    ///
    /// let layer = Cors::new().expose_headers([CONTENT_ENCODING]);
    /// ```
    ///
    /// All headers can be allowed with
    ///
    /// ```
    /// use tower_http::cors::{Any, Cors};
    ///
    /// let layer = Cors::new().expose_headers(Any);
    /// ```
    ///
    /// Note that multiple calls to this method will override any previous
    /// calls.
    ///
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Expose-Headers
    pub fn expose_headers<T>(mut self, headers: T) -> Self
    where
        T: Into<ExposeHeaders>,
    {
        self.expose_headers = headers.into();
        self
    }

    /// Set the value of the [`Access-Control-Allow-Private-Network`][wicg] header.
    ///
    /// ```
    /// use tower_http::cors::Cors;
    ///
    /// let layer = Cors::new().allow_private_network(true);
    /// ```
    ///
    /// [wicg]: https://wicg.github.io/private-network-access/
    pub fn allow_private_network<T>(mut self, allow_private_network: T) -> Self
    where
        T: Into<AllowPrivateNetwork>,
    {
        self.allow_private_network = allow_private_network.into();
        self
    }

    /// Set the value(s) of the [`Vary`][mdn] header.
    ///
    /// In contrast to the other headers, this one has a non-empty default of
    /// [`preflight_request_headers()`].
    ///
    /// You only need to set this is you want to remove some of these defaults,
    /// or if you use a closure for one of the other headers and want to add a
    /// vary header accordingly.
    ///
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Vary
    pub fn vary<T>(mut self, headers: T) -> Self
    where
        T: Into<Vary>,
    {
        self.vary = headers.into();
        self
    }
}

/// Represents a wildcard value (`*`) used with some CORS headers such as
/// [`Cors::allow_methods`].
#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct Any;

impl Default for Cors {
    fn default() -> Self {
        Self::new()
    }
}

struct OptionsFilter;

#[async_trait::async_trait]
impl<'a> FromRequestParts<'a> for OptionsFilter {
    async fn from_request_parts(request: RequestPartsRef<'a>) -> Result<Self> {
        if request.method != Method::Options {
            return Err(Error::SkipMiddleware);
        }
        Ok(Self)
    }
}

struct NotOptionsFilter;

#[async_trait::async_trait]
impl<'a> FromRequestParts<'a> for NotOptionsFilter {
    async fn from_request_parts(request: RequestPartsRef<'a>) -> Result<Self> {
        if request.method == Method::Options {
            return Err(Error::SkipMiddleware);
        }
        Ok(Self)
    }
}

impl Cors {
    async fn options_intercept(
        _: OptionsFilter,
        Extension(cors): Extension<Cors>,
        parts: RequestParts,
    ) -> Result<Option<HeaderMap>> {
        let origin = parts.headers.get("origin");
        let mut headers = HeaderMap::new();
        if let Some(header) = cors.allow_origin.to_header(origin, parts.as_ref()) {
            headers.append_typed(&header);
        }
        if let Some(header) = cors.allow_credentials.to_header(origin, parts.as_ref()) {
            headers.append_typed(&header);
        }
        if let Some((name, value)) = cors.allow_private_network.to_header(origin, parts.as_ref()) {
            headers.append(name, value);
        }
        for value in cors.vary.values() {
            headers.append("vary", value);
        }
        if let Some(header) = cors.allow_methods.to_header(parts.as_ref()) {
            headers.append_typed(&header);
        }
        if let Some(header) = cors.allow_headers.to_header(parts.as_ref()) {
            headers.append_typed(&header);
        }
        if let Some(header) = cors.max_age.to_header(origin, parts.as_ref()) {
            headers.append_typed(&header);
        }

        Ok(Some(headers))
    }

    async fn response_augment(
        _: NotOptionsFilter,
        Extension(cors): Extension<Cors>,
        parts: RequestParts,
        mut response: Response,
    ) -> Response {
        let origin = parts.headers.get("origin");

        if let Some(header) = cors.allow_origin.to_header(origin, parts.as_ref()) {
            response.headers.append_typed(&header);
        }
        if let Some(header) = cors.allow_credentials.to_header(origin, parts.as_ref()) {
            response.headers.append_typed(&header);
        }
        if let Some((name, value)) = cors.allow_private_network.to_header(origin, parts.as_ref()) {
            response.headers.append(name, value);
        }
        for value in cors.vary.values() {
            response.headers.append("vary", value);
        }
        if let Some(header) = cors.expose_headers.to_header(parts.as_ref()) {
            response.headers.append_typed(&header);
        }

        response
    }
}

impl Plugin for Cors {
    fn apply(self, router: Router, path: &str) -> Router {
        ensure_usable_cors_rules(&self);
        router
            .extension(path, self)
            .request_hook(path, Cors::options_intercept)
            .late_response_hook(path, Cors::response_augment)
    }
}

fn ensure_usable_cors_rules(layer: &Cors) {
    if layer.allow_credentials.is_true() {
        assert!(
            !layer.allow_headers.is_wildcard(),
            "Invalid CORS configuration: Cannot combine `Access-Control-Allow-Credentials: true` \
             with `Access-Control-Allow-Headers: *`"
        );

        assert!(
            !layer.allow_methods.is_wildcard(),
            "Invalid CORS configuration: Cannot combine `Access-Control-Allow-Credentials: true` \
             with `Access-Control-Allow-Methods: *`"
        );

        assert!(
            !layer.allow_origin.is_wildcard(),
            "Invalid CORS configuration: Cannot combine `Access-Control-Allow-Credentials: true` \
             with `Access-Control-Allow-Origin: *`"
        );

        assert!(
            !layer.expose_headers.is_wildcard(),
            "Invalid CORS configuration: Cannot combine `Access-Control-Allow-Credentials: true` \
             with `Access-Control-Expose-Headers: *`"
        );
    }
}

/// Returns an iterator over the three request headers that may be involved in a CORS preflight request.
///
/// This is the default set of header names returned in the `vary` header
pub fn preflight_request_headers() -> impl Iterator<Item = &'static str> {
    [
        "origin",
        "access-control-request-method",
        "access-control-request-headers",
    ]
    .into_iter()
}
