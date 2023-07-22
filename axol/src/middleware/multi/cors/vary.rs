use super::preflight_request_headers;

/// Holds configuration for how to set the [`Vary`][mdn] header.
///
/// See [`CorsLayer::vary`] for more details.
///
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Vary
/// [`CorsLayer::vary`]: super::CorsLayer::vary
#[derive(Clone, Debug)]
pub struct Vary(pub Vec<String>);

impl Vary {
    /// Set the list of header names to return as vary header values
    ///
    /// See [`CorsLayer::vary`] for more details.
    ///
    /// [`CorsLayer::vary`]: super::CorsLayer::vary
    pub fn list<S: Into<String>, I: IntoIterator<Item = S>>(headers: I) -> Self {
        let raw = headers.into_iter().map(Into::into).collect::<Vec<_>>();
        Self(raw)
    }

    pub(super) fn values(&self) -> impl Iterator<Item = &str> + '_ {
        self.0.iter().map(|x| &**x)
    }
}

impl Default for Vary {
    fn default() -> Self {
        Self::list(preflight_request_headers())
    }
}

impl<const N: usize, S: Into<String>> From<[S; N]> for Vary {
    fn from(arr: [S; N]) -> Self {
        Self::list(arr)
    }
}

impl<S: Into<String>> From<Vec<S>> for Vary {
    fn from(vec: Vec<S>) -> Self {
        Self::list(vec)
    }
}
