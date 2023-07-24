use std::sync::Arc;

use axol_http::response::Response;
use axol_http::Body;
use axol_http::{request::RequestPartsRef, Request};

use crate::{inner_handler, Handler, RequestHook, Result};

pub struct WrapState<'a> {
    pub(crate) wraps: Vec<Arc<dyn Wrap>>,
    pub(crate) target: WrapTarget<'a>,
    pub(crate) request: &'a mut Request,
}

pub(crate) struct OuterWrapState {
    pub(crate) request_hooks: Vec<Arc<dyn RequestHook>>,
    pub(crate) wraps: Vec<Arc<dyn Wrap>>,
    pub(crate) handler: Arc<dyn Handler>,
}

pub(crate) enum WrapTarget<'a> {
    Handler(&'a dyn Handler),
    Phase(OuterWrapState),
}

impl<'a> WrapState<'a> {
    pub async fn next(mut self) -> Result<Response> {
        if let Some(wrap) = self.wraps.pop() {
            return wrap.wrap(self).await;
        }
        match self.target {
            WrapTarget::Handler(h) => {
                let body = std::mem::take(&mut self.request.body);
                h.call(self.request.parts(), body).await
            }
            WrapTarget::Phase(phase) => {
                inner_handler(
                    phase.request_hooks,
                    phase.wraps,
                    phase.handler,
                    self.request,
                )
                .await
            }
        }
    }

    pub fn remove_body(&mut self) -> Body {
        std::mem::take(&mut self.request.body)
    }

    pub fn body(&self) -> &Body {
        &self.request.body
    }

    pub fn set_body(&mut self, body: Body) {
        self.request.body = body;
    }

    pub fn request(&self) -> RequestPartsRef<'_> {
        (&*self.request).parts()
    }
}

#[async_trait::async_trait]
pub trait Wrap: Send + Sync + 'static {
    async fn wrap(&self, state: WrapState<'_>) -> Result<Response>;
}
