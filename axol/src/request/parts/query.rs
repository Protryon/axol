use std::ops::Deref;

use axol_http::{request::RequestPartsRef, Body};
use serde::Deserialize;

use crate::{Error, FromRequest, FromRequestParts, Result};

#[derive(Debug, Clone)]
pub struct RawQuery<'a>(pub &'a str);

impl<'a> Deref for RawQuery<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait::async_trait]
impl<'a> FromRequestParts<'a> for RawQuery<'a> {
    async fn from_request_parts(request: RequestPartsRef<'a>) -> Result<Self> {
        request
            .uri
            .query()
            .map(RawQuery)
            .ok_or_else(|| Error::bad_request("missing query string"))
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for RawQuery<'a> {
    async fn from_request(request: RequestPartsRef<'a>, _body: Body) -> Result<Self> {
        <Self as FromRequestParts<'a>>::from_request_parts(request).await
    }
}

#[derive(Debug, Clone)]
pub struct Query<T>(pub T);

impl<T> Deref for Query<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait::async_trait]
impl<'a, T: Deserialize<'a> + Send + Sync + 'a> FromRequestParts<'a> for Query<T> {
    async fn from_request_parts(request: RequestPartsRef<'a>) -> Result<Self> {
        let query = request.uri.query().unwrap_or_default();
        Ok(Query(serde_urlencoded::from_str(query).map_err(|e| {
            Error::bad_request(format!("Failed to deserialize query string: {e}"))
        })?))
    }
}

#[async_trait::async_trait]
impl<'a, T: Deserialize<'a> + Send + Sync + 'a> FromRequest<'a> for Query<T> {
    async fn from_request(request: RequestPartsRef<'a>, _body: Body) -> Result<Self> {
        <Self as FromRequestParts<'a>>::from_request_parts(request).await
    }
}
