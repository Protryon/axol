use std::{ops::Deref, sync::Arc};

use axol_http::{request::RequestPartsRef, Body};
use serde::{de::DeserializeOwned, Deserialize};

use crate::{Error, FromRequest, FromRequestParts, Result};

use super::path_de::{self, PathDeserializationError};

pub struct RawPathExt(pub Vec<(Arc<str>, String)>);

#[derive(Debug, Clone)]
pub struct RawPath<'a>(pub &'a [(Arc<str>, String)]);

impl<'a> Deref for RawPath<'a> {
    type Target = [(Arc<str>, String)];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait::async_trait]
impl<'a> FromRequestParts<'a> for RawPath<'a> {
    async fn from_request_parts(request: RequestPartsRef<'a>) -> Result<Self> {
        match request.extensions.get::<RawPathExt>() {
            Some(values) => Ok(Self(&values.0[..])),
            None => Ok(Self(&[])),
        }
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for RawPath<'a> {
    async fn from_request(request: RequestPartsRef<'a>, _body: Body) -> Result<Self> {
        <Self as FromRequestParts<'a>>::from_request_parts(request).await
    }
}

#[derive(Debug, Clone)]
pub struct Path<T>(pub T);

impl<T> Deref for Path<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait::async_trait]
impl<'a, T: Deserialize<'a> + Send + Sync + 'a> FromRequestParts<'a> for Path<T> {
    async fn from_request_parts(request: RequestPartsRef<'a>) -> Result<Self> {
        let params = request
            .extensions
            .get::<RawPathExt>()
            .ok_or_else(|| Error::internal(anyhow::anyhow!("missing RawPathExt extension")))?;
        T::deserialize(path_de::PathDeserializer::new(&params.0))
            .map_err(|err| match err {
                PathDeserializationError::Message(_)
                | PathDeserializationError::ParseError { .. }
                | PathDeserializationError::ParseErrorAtIndex { .. }
                | PathDeserializationError::ParseErrorAtKey { .. } => {
                    Error::bad_request(format!("Invalid URL: {}", err))
                }
                PathDeserializationError::WrongNumberOfParameters { .. }
                | PathDeserializationError::UnsupportedType { .. } => Error::internal(err),
            })
            .map(Path)
    }
}

#[async_trait::async_trait]
impl<'a, T: DeserializeOwned + Send + Sync + 'a> FromRequest<'a> for Path<T> {
    async fn from_request(request: RequestPartsRef<'a>, _body: Body) -> Result<Self> {
        <Self as FromRequestParts<'a>>::from_request_parts(request).await
    }
}
