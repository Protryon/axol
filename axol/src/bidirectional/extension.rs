use std::{ops::Deref, sync::Arc};

use anyhow::anyhow;
use axol_http::{extensions::Removed, request::RequestPartsRef, response::ResponsePartsRef};

use crate::{Error, FromRequestParts, IntoResponseParts, Result};

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

impl<T> Deref for Extension<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
#[must_use]
pub struct ExtensionArc<T>(pub Arc<T>);

#[async_trait::async_trait]
impl<'a, T: Send + Sync + Clone + 'static> FromRequestParts<'a> for ExtensionArc<T> {
    async fn from_request_parts(request: RequestPartsRef<'a>) -> Result<Self> {
        Ok(Self(
            request
                .extensions
                .get_arc::<T>()
                .ok_or_else(|| Error::internal(anyhow!("missing request extension")))?
                .clone(),
        ))
    }
}

impl<T> Deref for ExtensionArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
#[must_use]
pub struct ExtensionRemove<T>(pub Removed<T>);

#[async_trait::async_trait]
impl<'a, T: Send + Sync + Clone + 'static> FromRequestParts<'a> for ExtensionRemove<T> {
    async fn from_request_parts(request: RequestPartsRef<'a>) -> Result<Self> {
        Ok(Self(
            request
                .extensions
                .remove::<T>()
                .ok_or_else(|| Error::internal(anyhow!("missing request extension")))?
                .clone(),
        ))
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
