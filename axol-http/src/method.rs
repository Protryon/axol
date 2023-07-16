use std::fmt;
use std::str::FromStr;

use thiserror::Error;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, strum::Display, strum::IntoStaticStr)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum Method {
    #[default]
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Connect,
    Patch,
    Trace,
}

#[derive(Error, Debug)]
pub enum MethodParseError {
    #[error("unknown method: '{0}'")]
    UnknownMethod(String),
}

// we cannot use strum because it won't use our error type
impl FromStr for Method {
    type Err = MethodParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(Method::Get),
            "POST" => Ok(Method::Post),
            "PUT" => Ok(Method::Put),
            "DELETE" => Ok(Method::Delete),
            "HEAD" => Ok(Method::Head),
            "OPTIONS" => Ok(Method::Options),
            "CONNECT" => Ok(Method::Connect),
            "PATCH" => Ok(Method::Patch),
            "TRACE" => Ok(Method::Trace),
            _ => Err(MethodParseError::UnknownMethod(s.to_string())),
        }
    }
}

impl TryFrom<&str> for Method {
    type Error = MethodParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl TryFrom<http::Method> for Method {
    type Error = MethodParseError;
    fn try_from(value: http::Method) -> Result<Self, MethodParseError> {
        Method::from_str(value.as_str())
            .map_err(|_| MethodParseError::UnknownMethod(value.as_str().to_string()))
    }
}

impl Into<http::Method> for Method {
    fn into(self) -> http::Method {
        match self {
            Method::Get => http::Method::GET,
            Method::Post => http::Method::POST,
            Method::Put => http::Method::PUT,
            Method::Delete => http::Method::DELETE,
            Method::Head => http::Method::HEAD,
            Method::Options => http::Method::OPTIONS,
            Method::Connect => http::Method::CONNECT,
            Method::Patch => http::Method::PATCH,
            Method::Trace => http::Method::TRACE,
        }
    }
}

impl Method {
    pub fn as_str(&self) -> &'static str {
        self.into()
    }
}

impl AsRef<str> for Method {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl PartialEq<str> for Method {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_ref() == other
    }
}

impl PartialEq<Method> for str {
    #[inline]
    fn eq(&self, other: &Method) -> bool {
        self == other.as_ref()
    }
}
