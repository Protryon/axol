use crate::{IntoResponseParts, Result};
use axol_http::{header::HeaderMap, response::Response, StatusCode};

pub trait IntoResponse {
    fn into_response(self) -> Result<Response>;
}

impl IntoResponse for () {
    fn into_response(self) -> Result<Response> {
        Ok(Response {
            body: axol_http::Body::Bytes(vec![]),
            ..Default::default()
        })
    }
}

//TODO fill out more
fn x() {}
impl IntoResponse for &str {
    fn into_response(self) -> Result<Response> {
        Ok(Response {
            body: axol_http::Body::Bytes(self.as_bytes().to_vec()),
            ..Default::default()
        })
    }
}

impl IntoResponse for &[u8] {
    fn into_response(self) -> Result<Response> {
        Ok(Response {
            body: axol_http::Body::Bytes(self.to_vec()),
            ..Default::default()
        })
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Result<Response> {
        Ok(Response {
            body: axol_http::Body::Bytes(self.into_bytes()),
            ..Default::default()
        })
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Result<Response> {
        Ok(Response {
            body: axol_http::Body::Bytes(self),
            ..Default::default()
        })
    }
}

impl IntoResponse for StatusCode {
    fn into_response(self) -> Result<Response> {
        Ok(Response {
            status: self,
            ..Default::default()
        })
    }
}

impl IntoResponse for HeaderMap {
    fn into_response(self) -> Result<Response> {
        Ok(Response {
            headers: self,
            ..Default::default()
        })
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Result<Response> {
        Ok(self)
    }
}

impl<T: IntoResponse> IntoResponse for Result<T> {
    fn into_response(self) -> Result<Response> {
        self.and_then(|x| x.into_response())
    }
}

impl<K: AsRef<str>, V: Into<String>, const N: usize> IntoResponse for [(K, V); N] {
    fn into_response(self) -> Result<Response> {
        (self, ()).into_response()
    }
}

macro_rules! impl_into_response {
    ( $($ty:ident),* $(,)? ) => {
        #[allow(non_snake_case)]
        impl<R, $($ty,)*> IntoResponse for ($($ty),*, R)
        where
            $( $ty: IntoResponseParts, )*
            R: IntoResponse,
        {
            fn into_response(self) -> Result<Response> {
                let ($($ty),*, res) = self;

                let mut res = res.into_response()?;
                let mut parts = res.parts_mut();

                $(
                    $ty.into_response_parts(&mut parts)?;
                )*

                Ok(res)
            }
        }
    }
}

all_the_tuples_no_empty!(impl_into_response);
