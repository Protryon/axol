use axol_http::request::Request;
use axol_http::response::Response;
use futures::Future;

use crate::Result;
use crate::{FromRequestParts, IntoResponse};

#[async_trait::async_trait]
pub trait RequestHook: Send + Sync + 'static {
    /// Called on an inbound request
    async fn handle_request(&self, request: &mut Request) -> Result<Option<Response>>;
}

#[async_trait::async_trait]
impl<'a, F, Fut, Res> RequestHook for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Option<Res>>> + Send,
    Res: IntoResponse,
{
    async fn handle_request(&self, _request: &mut Request) -> Result<Option<Response>> {
        Ok(self().await?.map(IntoResponse::into_response).transpose()?)
    }
}

#[async_trait::async_trait]
pub trait RequestHookExpansion<G>: Send + Sync + 'static {
    async fn handle_request(&self, request: &mut Request) -> Result<Option<Response>>;
}

#[async_trait::async_trait]
impl<G: 'static> RequestHook for Box<dyn RequestHookExpansion<G>> {
    async fn handle_request(&self, request: &mut Request) -> Result<Option<Response>> {
        (&**self).handle_request(request).await
    }
}

macro_rules! impl_handler {
    ( $($ty:ident),* $(,)? ) => {
        #[allow(non_snake_case)]
        #[async_trait::async_trait]
        impl<F, Fut, Res, $($ty,)*> RequestHookExpansion<(($($ty,)*), Fut, Res)> for F
        where for<'a> F: Fn($($ty,)* &mut Request) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Result<Option<Res>>> + Send + 'static,
            Res: IntoResponse,
            $( for<'a> $ty: FromRequestParts<'a> + Send, )*
        {
            async fn handle_request(&self, request: &mut Request) -> Result<Option<Response>> {
                let (parts, extensions) = request.parts();
                $(
                    let $ty = $ty::from_request_parts(parts, extensions).await?;
                )*

                let res = self($($ty,)* request).await?;

                res.map(IntoResponse::into_response).transpose()
            }
        }
    };
}

all_the_tuples_no_last_special_case!(impl_handler);
