use std::sync::Arc;

use axol_http::request::RequestPartsRef;
use axol_http::response::Response;
use axol_http::Body;

use crate::{Handler, Result};

pub struct WrapState<'a> {
    pub(crate) wraps: Vec<Arc<dyn Wrap>>,
    pub(crate) handler: &'a dyn Handler,
    pub(crate) request: RequestPartsRef<'a>,
}

impl<'a> WrapState<'a> {
    pub async fn next(mut self, body: Body) -> Result<Response> {
        if let Some(wrap) = self.wraps.pop() {
            return wrap.wrap(self.request, body, self).await;
        }
        self.handler.call(self.request, body).await
    }
}

#[async_trait::async_trait]
pub trait Wrap: Send + Sync + 'static {
    async fn wrap(
        &self,
        request: RequestPartsRef<'_>,
        body: Body,
        state: WrapState<'_>,
    ) -> Result<Response>;
}
