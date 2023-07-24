use std::ops::{Deref, DerefMut};

use axol_http::{
    mime::Mime, request::RequestPartsRef, response::Response, typed_headers::ContentType, Body,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::{Error, FromRequest, FromRequestParts, IntoResponse, Result, Typed};

#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Form<T>(pub T);

impl<T> Deref for Form<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Form<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait::async_trait]
impl<'a, T: DeserializeOwned + Send + Sync + 'a> FromRequest<'a> for Form<T> {
    async fn from_request(request: RequestPartsRef<'a>, body: Body) -> Result<Self> {
        let content_type: Mime = Typed::<ContentType>::from_request_parts(request)
            .await?
            .0
            .into();
        if content_type.essence_str()
            != axol_http::mime::APPLICATION_WWW_FORM_URLENCODED.essence_str()
        {
            return Err(Error::unsupported_media_type(
                "Expected request with `Content-Type: application/x-www-form-urlencoded`",
            ));
        }
        let bytes = body.collect().await?;
        let deserializer = serde_urlencoded::Deserializer::new(form_urlencoded::parse(&bytes));

        let value = match serde_path_to_error::deserialize(deserializer) {
            Ok(value) => value,
            Err(err) => {
                let rejection = Error::bad_request(format!(
                    "Failed to parse the request body as a form: {err}"
                ));
                return Err(rejection);
            }
        };

        Ok(Form(value))
    }
}

impl<T: Serialize> IntoResponse for Form<T> {
    fn into_response(self) -> Result<Response> {
        let mut out = Response::default();
        out.headers.append_typed(&ContentType::form_url_encoded());
        out.body = Body::Bytes(
            serde_urlencoded::to_string(&self.0)
                .map_err(Error::internal)?
                .into_bytes(),
        );
        Ok(out)
    }
}
