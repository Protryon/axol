use std::ops::{Deref, DerefMut};

use axol_http::{
    mime::Mime, request::RequestPartsRef, response::Response, typed_headers::ContentType, Body,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::{Error, FromRequest, FromRequestParts, IntoResponse, Result, Typed};

#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Json<T>(pub T);

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait::async_trait]
impl<'a, T: DeserializeOwned + Send + Sync + 'a> FromRequest<'a> for Json<T> {
    async fn from_request(request: RequestPartsRef<'a>, body: Body) -> Result<Self> {
        let content_type: Mime = Typed::<ContentType>::from_request_parts(request)
            .await?
            .0
            .into();
        if content_type.essence_str() != axol_http::mime::APPLICATION_JSON.essence_str() {
            return Err(Error::unsupported_media_type(
                "Expected request with `Content-Type: application/json`",
            ));
        }
        let bytes = body.collect().await?;
        let deserializer = &mut serde_json::Deserializer::from_slice(&bytes);

        let value = match serde_path_to_error::deserialize(deserializer) {
            Ok(value) => value,
            Err(err) => {
                let rejection = match err.inner().classify() {
                    serde_json::error::Category::Data => Error::unprocessable_entity(format!(
                        "Failed to deserialize the JSON body into the target type: {err}"
                    )),
                    serde_json::error::Category::Syntax | serde_json::error::Category::Eof => {
                        Error::bad_request(format!(
                            "Failed to parse the request body as JSON: {err}"
                        ))
                    }
                    serde_json::error::Category::Io => {
                        if cfg!(debug_assertions) {
                            // we don't use `serde_json::from_reader` and instead always buffer
                            // bodies first, so we shouldn't encounter any IO errors
                            unreachable!()
                        } else {
                            Error::bad_request(format!(
                                "Failed to parse the request body as JSON: {err}"
                            ))
                        }
                    }
                };
                return Err(rejection);
            }
        };

        Ok(Json(value))
    }
}

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Result<Response> {
        let mut out = Response::default();
        out.headers.append_typed(&ContentType::json());
        out.body = Body::Bytes(serde_json::to_vec(&self.0).map_err(Error::internal)?);
        Ok(out)
    }
}
