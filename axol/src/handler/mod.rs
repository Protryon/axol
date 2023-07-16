use crate::{FromRequest, FromRequestParts, IntoResponse, Result};
use axol_http::{request::RequestPartsRef, response::Response, Body};
use futures::Future;

#[async_trait::async_trait]
pub trait Handler: Send + Sync + 'static {
    async fn call<'a>(&self, request_parts: RequestPartsRef<'a>, body: Body) -> Result<Response>;
}

#[async_trait::async_trait]
impl<F, Fut, Res> Handler for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send,
    Res: IntoResponse,
{
    async fn call<'a>(&self, _request_parts: RequestPartsRef<'a>, _body: Body) -> Result<Response> {
        self().await.into_response()
    }
}

#[async_trait::async_trait]
pub trait HandlerExpansion<G>: Send + Sync + 'static {
    async fn call<'a>(&self, request_parts: RequestPartsRef<'a>, body: Body) -> Result<Response>;
}

#[async_trait::async_trait]
impl<G: 'static> Handler for Box<dyn HandlerExpansion<G>> {
    async fn call<'a>(&self, request_parts: RequestPartsRef<'a>, body: Body) -> Result<Response> {
        (&**self).call(request_parts, body).await
    }
}

macro_rules! impl_handler {
    (
        [$($ty:ident),*], $last:ident
    ) => {
        #[allow(non_snake_case)]
        #[async_trait::async_trait]
        impl<F, Fut, Res, $($ty,)* $last> HandlerExpansion<(($($ty,)* $last,), Fut, Res)> for F
        where for<'a> F: Fn($($ty,)* $last,) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
            Res: IntoResponse,
            $( for<'a> $ty: FromRequestParts<'a> + Send, )*
            for<'a> $last: FromRequest<'a> + Send,
        {
            async fn call<'a>(&self, request_parts: RequestPartsRef<'a>, body: Body) -> Result<Response> {
                $(
                    let $ty = $ty::from_request_parts(request_parts).await?;
                )*

                let $last = $last::from_request(request_parts, body).await?;

                let res = self($($ty,)* $last,).await;

                res.into_response()
            }
        }
    };
}

all_the_tuples!(impl_handler);
