use axol_http::{header::HeaderMap, request::RequestPartsRef, Method, Uri, Version, Extensions};

use crate::Result;

mod query;
pub use query::*;

mod path;
mod path_de;
pub use path::*;

mod connect_info;
pub use connect_info::*;

#[async_trait::async_trait]
pub trait FromRequestParts<'a>: Sized + Send + Sync + 'a {
    async fn from_request_parts(request: RequestPartsRef<'a>, extensions: &mut Extensions) -> Result<Self>;
}

#[async_trait::async_trait]
impl<'a, T: FromRequestParts<'a>> FromRequestParts<'a> for Option<T> {
    async fn from_request_parts(request: RequestPartsRef<'a>, extensions: &mut Extensions) -> Result<Self> {
        Ok(T::from_request_parts(request, extensions).await.ok())
    }
}

#[async_trait::async_trait]
impl<'a, T: FromRequestParts<'a>> FromRequestParts<'a> for Result<T> {
    async fn from_request_parts(request: RequestPartsRef<'a>, extensions: &mut Extensions) -> Result<Self> {
        Ok(T::from_request_parts(request, extensions).await)
    }
}

#[async_trait::async_trait]
impl<'a> FromRequestParts<'a> for Method {
    async fn from_request_parts(request: RequestPartsRef<'a>, _: &mut Extensions) -> Result<Self> {
        Ok(request.method)
    }
}

#[async_trait::async_trait]
impl<'a> FromRequestParts<'a> for &'a HeaderMap {
    async fn from_request_parts(request: RequestPartsRef<'a>, _: &mut Extensions) -> Result<Self> {
        Ok(&request.headers)
    }
}

#[async_trait::async_trait]
impl<'a> FromRequestParts<'a> for &'a Uri {
    async fn from_request_parts(request: RequestPartsRef<'a>, _: &mut Extensions) -> Result<Self> {
        Ok(&request.uri)
    }
}

#[async_trait::async_trait]
impl<'a> FromRequestParts<'a> for Version {
    async fn from_request_parts(request: RequestPartsRef<'a>, _: &mut Extensions) -> Result<Self> {
        Ok(request.version)
    }
}

macro_rules! impl_from_request {
    (
        [$($ty:ident),*], $last:ident
    ) => {
        #[async_trait::async_trait]
        #[allow(non_snake_case, unused_mut, unused_variables)]
        impl<'a, $($ty,)* $last> FromRequestParts<'a> for ($($ty,)* $last,)
        where
            $( $ty: FromRequestParts<'a>, )*
            $last: FromRequestParts<'a>,
        {
            async fn from_request_parts(request: RequestPartsRef<'a>, extensions: &mut Extensions) -> Result<Self> {
                $(
                    let $ty = $ty::from_request_parts(request, extensions)
                        .await?;
                )*
                let $last = $last::from_request_parts(request, extensions)
                    .await?;

                Ok(($($ty,)* $last,))
            }
        }
    };
}

all_the_tuples!(impl_from_request);
