use std::ops::{Deref, DerefMut};

use crate::{IntoResponse, Result};
use axol_http::{response::Response, typed_headers::ContentType, Body};

#[derive(Debug, Clone)]
#[must_use]
pub struct TypedBody<T: Into<Vec<u8>>>(pub ContentType, pub T);

impl<T: Into<Vec<u8>>> Deref for TypedBody<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl<T: Into<Vec<u8>>> DerefMut for TypedBody<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.1
    }
}

impl<T: Into<Vec<u8>>> IntoResponse for TypedBody<T> {
    fn into_response(self) -> Result<Response> {
        let mut out = Response::default();
        out.headers.append_typed(&self.0);
        out.body = Body::Bytes(self.1.into());
        Ok(out)
    }
}

#[derive(Debug, Clone)]
#[must_use]
pub struct Html<T: Into<Vec<u8>>>(pub T);

impl<T: Into<Vec<u8>>> IntoResponse for Html<T> {
    fn into_response(self) -> Result<Response> {
        TypedBody(ContentType::html(), self.0.into()).into_response()
    }
}
