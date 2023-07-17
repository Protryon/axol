use std::{net::SocketAddr, ops::Deref};

use anyhow::anyhow;
use axol_http::{request::RequestPartsRef, Body};

use crate::{Error, FromRequest, FromRequestParts, Result};

#[derive(Debug, Clone, Copy)]
pub struct ConnectInfo(pub SocketAddr);

impl Deref for ConnectInfo {
    type Target = SocketAddr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait::async_trait]
impl<'a> FromRequestParts<'a> for ConnectInfo {
    async fn from_request_parts(request: RequestPartsRef<'a>) -> Result<Self> {
        let info = request
            .extensions
            .get::<ConnectInfo>()
            .ok_or_else(|| Error::internal(anyhow!("missing ConnectInfo extension")))?;
        Ok(*info)
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for ConnectInfo {
    async fn from_request(request: RequestPartsRef<'a>, _body: Body) -> Result<Self> {
        <Self as FromRequestParts<'a>>::from_request_parts(request).await
    }
}
