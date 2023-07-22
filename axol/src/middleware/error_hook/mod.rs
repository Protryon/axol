use axol_http::{request::RequestPartsRef, response::Response};
use futures::Future;

use crate::{Error, FromRequestParts, IntoResponse, Result};

mod default;
pub use default::DefaultErrorHook;

#[async_trait::async_trait]
pub trait ErrorHook: Send + Sync + 'static {
    /// If returns Some(response), no further ErrorHooks will be invoked and that response will have LateResponseHooks called on it.
    /// If no ErrorHook returns Some, then the default ErrorHook is invoked
    /// Note that any Error returned will result in a warning log and the ErrorHook being skipped.
    async fn handle_error<'a>(
        &self,
        request: RequestPartsRef<'a>,
        error: &mut Error,
    ) -> Result<Option<Response>>;
}

#[async_trait::async_trait]
impl<F, Fut, Res> ErrorHookExpansion<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Option<Res>>> + Send,
    Res: IntoResponse,
{
    async fn handle_error<'a>(
        &self,
        _request: RequestPartsRef<'a>,
        _error: &mut Error,
    ) -> Result<Option<Response>> {
        self().await?.map(IntoResponse::into_response).transpose()
    }
}

#[async_trait::async_trait]
pub trait ErrorHookExpansion<G>: Send + Sync + 'static {
    async fn handle_error<'a>(
        &self,
        request: RequestPartsRef<'a>,
        error: &mut Error,
    ) -> Result<Option<Response>>;
}

#[async_trait::async_trait]
impl<G: 'static> ErrorHook for Box<dyn ErrorHookExpansion<G>> {
    async fn handle_error<'a>(
        &self,
        request: RequestPartsRef<'a>,
        error: &mut Error,
    ) -> Result<Option<Response>> {
        (&**self).handle_error(request, error).await
    }
}

macro_rules! impl_handler {
    ( $($ty:ident),* $(,)? ) => {
        #[allow(non_snake_case)]
        #[async_trait::async_trait]
        impl<F, Fut, Res, $($ty,)*> ErrorHookExpansion<(($($ty,)*), Fut, Res)> for F
        where for<'a> F: Fn($($ty,)* &mut Error) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Result<Option<Res>>> + Send + 'static,
            Res: IntoResponse,
            $( for<'a> $ty: FromRequestParts<'a> + Send, )*
        {
            async fn handle_error<'a>(&self, request: RequestPartsRef<'a>, error: &mut Error) -> Result<Option<Response>> {
                $(
                    let $ty = $ty::from_request_parts(request).await?;
                )*

                let res = self($($ty,)* error).await?;

                res.map(IntoResponse::into_response).transpose()
            }
        }
    };
}

all_the_tuples_no_last_special_case!(impl_handler);
