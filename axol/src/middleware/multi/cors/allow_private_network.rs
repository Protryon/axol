use std::{fmt, sync::Arc};

use axol_http::request::RequestPartsRef;

/// Holds configuration for how to set the [`Access-Control-Allow-Private-Network`][wicg] header.
///
/// See [`CorsLayer::allow_private_network`] for more details.
///
/// [wicg]: https://wicg.github.io/private-network-access/
/// [`CorsLayer::allow_private_network`]: super::CorsLayer::allow_private_network
#[derive(Clone)]
#[must_use]
pub enum AllowPrivateNetwork {
    /// Allow requests via a more private network than the one used to access the origin
    ///
    /// See [`CorsLayer::allow_private_network`] for more details.
    ///
    /// [`CorsLayer::allow_private_network`]: super::CorsLayer::allow_private_network
    Yes,
    No,
    Predicate(Arc<dyn for<'a> Fn(&'a str, RequestPartsRef<'a>) -> bool + Send + Sync + 'static>),
}

impl Default for AllowPrivateNetwork {
    fn default() -> Self {
        Self::No
    }
}

impl AllowPrivateNetwork {
    /// Allow requests via private network for some requests, based on a given predicate
    ///
    /// The first argument to the predicate is the request origin.
    ///
    /// See [`CorsLayer::allow_private_network`] for more details.
    ///
    /// [`CorsLayer::allow_private_network`]: super::CorsLayer::allow_private_network
    pub fn predicate<F>(f: F) -> Self
    where
        F: Fn(&str, RequestPartsRef<'_>) -> bool + Send + Sync + 'static,
    {
        Self::Predicate(Arc::new(f))
    }

    pub(super) fn to_header(
        &self,
        origin: Option<&str>,
        parts: RequestPartsRef<'_>,
    ) -> Option<(&'static str, &'static str)> {
        // #[allow(clippy::declare_interior_mutable_const)]
        // const REQUEST_PRIVATE_NETWORK: HeaderName =
        //     HeaderName::from_static("access-control-request-private-network");

        // #[allow(clippy::declare_interior_mutable_const)]
        // const ALLOW_PRIVATE_NETWORK: HeaderName =
        //     HeaderName::from_static("access-control-allow-private-network");

        // Cheapest fallback: allow_private_network hasn't been set
        if let Self::No = self {
            return None;
        }

        // Access-Control-Allow-Private-Network is only relevant if the request
        // has the Access-Control-Request-Private-Network header set, else skip
        if parts.headers.get("access-control-request-private-network") != Some("true") {
            return None;
        }

        let allow_private_network = match self {
            Self::Yes => true,
            Self::No => false, // unreachable, but not harmful
            Self::Predicate(predicate) => predicate(origin?, parts),
        };

        allow_private_network.then(|| ("access-control-allow-private-network", "true"))
    }
}

impl From<bool> for AllowPrivateNetwork {
    fn from(v: bool) -> Self {
        match v {
            true => Self::Yes,
            false => Self::No,
        }
    }
}

impl fmt::Debug for AllowPrivateNetwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yes => write!(f, "Yes"),
            Self::No => write!(f, "No"),
            Self::Predicate(_) => f.debug_tuple("Predicate").finish(),
        }
    }
}
