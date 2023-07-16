use std::ops::{Deref, DerefMut};

use axol_http::{header::TypedHeader, request::RequestPartsRef, response::ResponsePartsRef, Extensions};

use crate::{Error, FromRequestParts, IntoResponseParts, Result};

#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Typed<H: TypedHeader>(pub H);

impl<H: TypedHeader> Deref for Typed<H> {
    type Target = H;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<H: TypedHeader> DerefMut for Typed<H> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<H: TypedHeader> IntoResponseParts for Typed<H> {
    fn into_response_parts(self, response: &mut ResponsePartsRef<'_>) -> Result<()> {
        self.0.encode(response.headers);
        Ok(())
    }
}

#[async_trait::async_trait]
impl<'a, H: TypedHeader + Send + Sync + 'a> FromRequestParts<'a> for Typed<H> {
    async fn from_request_parts(request: RequestPartsRef<'a>, _: &mut Extensions) -> Result<Self> {
        request
            .headers
            .get_typed::<H>()
            .ok_or_else(|| Error::bad_request(format!("missing header: {}", H::name())))
            .map(Typed)
    }
}
