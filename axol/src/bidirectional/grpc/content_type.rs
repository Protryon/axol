use std::str::FromStr;

use axol_http::mime::Mime;

use crate::Result;
use axol_http::typed_headers::{
    ContentType, Error as HeaderError, Header, HeaderName, HeaderValue,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GrpcContentType {
    Invalid,
    Proto,
    Json,
    Other(String),
}

impl Default for GrpcContentType {
    fn default() -> Self {
        GrpcContentType::Proto
    }
}

impl GrpcContentType {
    fn as_str(&self) -> &str {
        match self {
            GrpcContentType::Invalid => unimplemented!(),
            GrpcContentType::Proto => "proto",
            GrpcContentType::Json => "json",
            GrpcContentType::Other(x) => x,
        }
    }
}

static CONTENT_TYPE: HeaderName = HeaderName::from_static("content-type");

impl Header for GrpcContentType {
    fn name() -> &'static HeaderName {
        &CONTENT_TYPE
    }

    fn decode<'i, I: Iterator<Item = &'i HeaderValue>>(values: &mut I) -> Result<Self, HeaderError>
    where
        Self: Sized,
    {
        let content_type = ContentType::decode(values)?;
        let content_type: Mime = content_type.into();
        if content_type.type_() != "application" || content_type.subtype() != "grpc" {
            return Ok(Self::Invalid);
        }

        Ok(match content_type.suffix().map(|x| x.as_str()) {
            Some("proto") | None => GrpcContentType::Proto,
            Some("json") => GrpcContentType::Json,
            Some(x) => GrpcContentType::Other(x.to_string()),
        })
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        let content_type: ContentType =
            Mime::from_str(&format!("application/grpc+{}", self.as_str()))
                .unwrap()
                .into();
        content_type.encode(values);
    }
}
