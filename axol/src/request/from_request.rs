use axol_http::{request::RequestPartsRef, Body};

use crate::{Error, FromRequestParts, Result};

#[async_trait::async_trait]
pub trait FromRequest<'a>: Sized + Send + Sync + 'a {
    async fn from_request(request: RequestPartsRef<'a>, body: Body) -> Result<Self>;
}

#[async_trait::async_trait]
impl<'a, T: FromRequest<'a>> FromRequest<'a> for Option<T> {
    async fn from_request(request: RequestPartsRef<'a>, body: Body) -> Result<Self> {
        Ok(T::from_request(request, body).await.ok())
    }
}

#[async_trait::async_trait]
impl<'a, T: FromRequest<'a>> FromRequest<'a> for Result<T> {
    async fn from_request(request: RequestPartsRef<'a>, body: Body) -> Result<Self> {
        Ok(T::from_request(request, body).await)
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Body {
    async fn from_request(_: RequestPartsRef<'a>, body: Body) -> Result<Self> {
        Ok(body)
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Vec<u8> {
    async fn from_request(_: RequestPartsRef<'a>, body: Body) -> Result<Self> {
        Ok(body.collect().await?)
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for String {
    async fn from_request(_: RequestPartsRef<'a>, body: Body) -> Result<Self> {
        String::from_utf8(body.collect().await?).map_err(|_| Error::BadUtf8)
    }
}

macro_rules! impl_from_request {
    (
        [$($ty:ident),*], $last:ident
    ) => {
        #[async_trait::async_trait]
        #[allow(non_snake_case, unused_mut, unused_variables)]
        impl<'a, $($ty,)* $last> FromRequest<'a> for ($($ty,)* $last,)
        where
            $( $ty: FromRequestParts<'a>, )*
            $last: FromRequest<'a>,
        {
            async fn from_request(request: RequestPartsRef<'a>, body: Body) -> Result<Self> {
                $(
                    let $ty = $ty::from_request_parts(request).await?;
                )*

                let $last = $last::from_request(request, body).await?;

                Ok(($($ty,)* $last,))
            }
        }
    };
}

all_the_tuples!(impl_from_request);
