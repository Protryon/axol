use std::ops::Deref;

use axol_http::{request::RequestPartsRef, Extensions};
use serde::Deserialize;

use crate::{Error, FromRequestParts, Result};

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
    async fn from_request_parts(request: RequestPartsRef<'a>, _: &mut Extensions) -> Result<Self> {
        request
            .uri
            .query()
            .map(RawQuery)
            .ok_or_else(|| Error::bad_request("missing query string"))
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
    async fn from_request_parts(request: RequestPartsRef<'a>, _: &mut Extensions) -> Result<Self> {
        let query = request.uri.query().unwrap_or_default();
        Ok(Query(serde_urlencoded::from_str(query).map_err(|e| {
            Error::bad_request(format!("Failed to deserialize query string: {e}"))
        })?))
    }
}
