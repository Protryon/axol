use std::{net::SocketAddr, ops::Deref};

use anyhow::anyhow;
use axol_http::{request::RequestPartsRef, Extensions};

use crate::{Error, FromRequestParts, Result};

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
    async fn from_request_parts(_: RequestPartsRef<'a>, extensions: &mut Extensions) -> Result<Self> {
        let info = extensions
            .get::<ConnectInfo>()
            .ok_or_else(|| Error::internal(anyhow!("missing ConnectInfo extension")))?;
        Ok(*info)
    }
}
