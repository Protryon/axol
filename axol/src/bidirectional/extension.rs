use std::ops::Deref;

use anyhow::anyhow;
use axol_http::{request::RequestPartsRef, response::ResponsePartsRef, Body};

use crate::{Error, FromRequest, FromRequestParts, IntoResponseParts, Result};

#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct Extension<T>(pub T);

#[async_trait::async_trait]
impl<'a, T: Send + Sync + Clone + 'static> FromRequestParts<'a> for Extension<T> {
    async fn from_request_parts(request: RequestPartsRef<'a>) -> Result<Self> {
        Ok(Self(
            request
                .extensions
                .get::<T>()
                .ok_or_else(|| Error::internal(anyhow!("missing request extension")))?
                .clone(),
        ))
    }
}

#[async_trait::async_trait]
impl<'a, T: Send + Sync + Clone + 'static> FromRequest<'a> for Extension<T> {
    async fn from_request(request: RequestPartsRef<'a>, _: Body) -> Result<Self> {
        Self::from_request_parts(request).await
    }
}

impl<T> Deref for Extension<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct AddExtension<T>(pub T);

impl<T: Send + Sync + 'static> IntoResponseParts for AddExtension<T> {
    fn into_response_parts(self, response: &mut ResponsePartsRef<'_>) -> Result<()> {
        response.extensions.insert(self.0);
        Ok(())
    }
}
