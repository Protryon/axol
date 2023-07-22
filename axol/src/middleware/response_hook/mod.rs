use crate::{FromRequestParts, Result};
use axol_http::{request::RequestPartsRef, response::Response};
use futures::Future;

#[async_trait::async_trait]
pub trait EarlyResponseHook: Send + Sync + 'static {
    /// Called when a handler returns an Ok(response)
    /// Any errors returned here will go through ErrorHooks.
    async fn handle_response<'a>(
        &self,
        request: RequestPartsRef<'a>,
        response: &mut Response,
    ) -> Result<()>;
}

#[async_trait::async_trait]
pub trait LateResponseHook: Send + Sync + 'static {
    /// Called before a response is written over the wire. Error responses also go through this stage.
    /// Notably, errors cannot be thrown here.
    /// If any request extractors fail, an error is logged and the hook is skipped. Keep that in mind!
    /// It's best to use Option extractors if any.
    async fn handle_response<'a>(&self, request: RequestPartsRef<'a>, response: &mut Response);
}

#[async_trait::async_trait]
impl<F, Fut> EarlyResponseHookExpansion<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<()>> + Send,
{
    async fn handle_response<'a>(
        &self,
        _request: RequestPartsRef<'a>,
        _response: &mut Response,
    ) -> Result<()> {
        self().await
    }
}

#[async_trait::async_trait]
pub trait EarlyResponseHookExpansion<G>: Send + Sync + 'static {
    async fn handle_response<'a>(
        &self,
        request: RequestPartsRef<'a>,
        response: &mut Response,
    ) -> Result<()>;
}

#[async_trait::async_trait]
impl<G: 'static> EarlyResponseHook for Box<dyn EarlyResponseHookExpansion<G>> {
    async fn handle_response<'a>(
        &self,
        request: RequestPartsRef<'a>,
        response: &mut Response,
    ) -> Result<()> {
        (&**self).handle_response(request, response).await
    }
}
//

#[async_trait::async_trait]
impl<F, Fut> LateResponseHookExpansion<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send,
{
    async fn handle_response<'a>(&self, _request: RequestPartsRef<'a>, _response: &mut Response) {
        self().await
    }
}

#[async_trait::async_trait]
pub trait LateResponseHookExpansion<G>: Send + Sync + 'static {
    async fn handle_response<'a>(&self, request: RequestPartsRef<'a>, response: &mut Response);
}

#[async_trait::async_trait]
impl<G: 'static> LateResponseHook for Box<dyn LateResponseHookExpansion<G>> {
    async fn handle_response<'a>(&self, request: RequestPartsRef<'a>, response: &mut Response) {
        (&**self).handle_response(request, response).await
    }
}

//
macro_rules! impl_handler {
    ( $($ty:ident),* $(,)? ) => {
        #[allow(non_snake_case)]
        #[allow(unused_variables)]
        #[async_trait::async_trait]
        impl<F, Fut, $($ty,)*> EarlyResponseHookExpansion<(($($ty,)*), Fut)> for F
        where for<'a> F: Fn($($ty,)* &mut Response) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Result<()>> + Send + 'static,
            $( for<'a> $ty: FromRequestParts<'a> + Send, )*
        {
            async fn handle_response<'a>(&self, request: RequestPartsRef<'a>, response: &mut Response) -> Result<()> {
                $(
                    let $ty = $ty::from_request_parts(request).await?;
                )*

                self($($ty,)* response).await
            }
        }

        #[allow(non_snake_case)]
        #[allow(unused_variables)]
        #[async_trait::async_trait]
        impl<F, Fut, $($ty,)*> LateResponseHookExpansion<(($($ty,)*), Fut)> for F
        where for<'a> F: Fn($($ty,)* Response) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Response> + Send + 'static,
            $( for<'a> $ty: FromRequestParts<'a> + Send, )*
        {
            async fn handle_response<'a>(&self, request: RequestPartsRef<'a>, response: &mut Response) {
                $(
                    let $ty = match $ty::from_request_parts(request).await {
                        Ok(x) => x,
                        Err(e) => {
                            log::warn!("late response hook extractor error, skipped hook: {e}");
                            return;
                        }
                    };
                )*

                let owned_response = std::mem::take(response);
                *response = self($($ty,)* owned_response).await;
            }
        }
    };
}

all_the_tuples_no_last_special_case!(impl_handler);
