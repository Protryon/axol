use std::borrow::Cow;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use crate::{
    ConnectInfo, DefaultErrorHook, Error, ErrorHook, Handler, ObservedRoute, OuterWrapState,
    RawPathExt, RequestHook, Wrap, WrapTarget,
};
use crate::{IntoResponse, Result, WrapState};
use axol_http::body::{BodyComponent, BodyWrapper};
use axol_http::header::HeaderMapConvertError;
use axol_http::{request::Request, response::Response};
use axol_http::{Body, StatusCode};
use derive_builder::Builder;
use futures::{FutureExt, Stream};
use hyper::body::HttpBody;
use hyper::server::accept::Accept;
use hyper::server::conn::AddrIncoming;
use hyper::server::Builder;
pub use hyper::Error as HyperError;
use hyper::{
    server::conn::AddrStream, Body as HyperBody, Request as HyperRequest, Response as HyperResponse,
};
use log::error;
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite};
#[cfg(feature = "trace")]
use tracing::Instrument;

use crate::Router;

#[cfg(feature = "tls")]
mod tls_acceptor;
#[cfg(feature = "tls")]
pub use tls_acceptor::*;

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct Server<I> {
    incoming: I,
    router: Router,
}

impl ServerBuilder<AddrIncoming> {
    pub fn bind(mut self, addr: SocketAddr) -> Result<Self, HyperError> {
        self.incoming = Some(AddrIncoming::bind(&addr)?);
        Ok(self)
    }
}

#[cfg(feature = "tls")]
impl ServerBuilder<TlsIncoming> {
    pub fn bind_with_tls(
        self,
        addr: SocketAddr,
        tls_config: rustls::ServerConfig,
    ) -> Result<
        ServerBuilder<
            AcceptWrapper<
                impl Stream<Item = Result<tokio_rustls::server::TlsStream<AddrStream>, std::io::Error>>,
            >,
        >,
        HyperError,
    > {
        Ok(ServerBuilder {
            incoming: Some(
                TlsIncoming::new_static(
                    addr,
                    false,
                    Some(std::time::Duration::from_secs(10)),
                    tls_config,
                )?
                .start(),
            ),
            router: self.router,
        })
    }
}

impl<I: Accept + 'static> ServerBuilder<I>
where
    I::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    I::Conn: AsyncRead + AsyncWrite + RemoteSocket + Unpin + Send + 'static,
{
    pub async fn serve(self) -> Result<(), hyper::Error> {
        self.serve_custom(|x| x).await
    }

    pub async fn serve_custom(
        self,
        customize: impl FnOnce(Builder<I>) -> Builder<I>,
    ) -> Result<(), hyper::Error> {
        self.build().unwrap().serve_custom(customize).await
    }
}

impl Server<AddrIncoming> {
    pub fn bind(addr: SocketAddr) -> Result<ServerBuilder<AddrIncoming>, HyperError> {
        ServerBuilder::default().bind(addr)
    }
}

impl Server<TlsIncoming> {
    pub fn bind_with_tls(
        self,
        addr: SocketAddr,
        tls_config: rustls::ServerConfig,
    ) -> Result<
        ServerBuilder<
            AcceptWrapper<
                impl Stream<Item = Result<tokio_rustls::server::TlsStream<AddrStream>, std::io::Error>>,
            >,
        >,
        HyperError,
    > {
        ServerBuilder::default().bind_with_tls(addr, tls_config)
    }
}

impl<I: Accept> Server<I> {
    pub fn builder() -> ServerBuilder<I> {
        ServerBuilder::default()
    }
}

pin_project! {
    struct BodyInputStream {
        #[pin]
        body: HyperBody,
        data_ended: bool,
        trailers_ended: bool,
    }
}

impl Stream for BodyInputStream {
    type Item = Result<BodyComponent, anyhow::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.data_ended && self.trailers_ended {
            return Poll::Ready(None);
        }
        let mut this = self.project();
        if !*this.data_ended {
            if let Poll::Ready(ready) = HttpBody::poll_data(this.body.as_mut(), cx)
                .map_ok(|x| BodyComponent::Data(x))
                .map_err(|e| e.into())
            {
                if let Some(ready) = ready {
                    return Poll::Ready(Some(ready));
                } else {
                    *this.data_ended = true;
                }
            } else {
                return Poll::Pending;
            }
        }
        if let Poll::Ready(ready) = HttpBody::poll_trailers(this.body, cx).map_err(|e| e.into()) {
            *this.trailers_ended = true;
            return match ready {
                Ok(Some(trailers)) => Poll::Ready(Some(
                    trailers
                        .try_into()
                        .map_err(Into::into)
                        .map(BodyComponent::Trailers),
                )),
                Ok(None) => Poll::Ready(None),
                Err(e) => Poll::Ready(Some(Err(e))),
            };
        } else {
            return Poll::Pending;
        }
    }
}

pub trait RemoteSocket {
    fn remote_addr(&self) -> SocketAddr;
}

impl RemoteSocket for AddrStream {
    fn remote_addr(&self) -> SocketAddr {
        AddrStream::remote_addr(self)
    }
}

#[async_recursion::async_recursion]
pub(crate) async fn inner_handler(
    request_hooks: Vec<Arc<dyn RequestHook>>,
    wraps: Vec<Arc<dyn Wrap>>,
    handler: Arc<dyn Handler>,
    request: &mut Request,
) -> Result<Response> {
    for middleware in request_hooks {
        match middleware.handle_request(&mut *request).await {
            Ok(Some(x)) => return Ok(x),
            Err(Error::SkipMiddleware) | Ok(None) => (),
            Err(e) => return Err(e),
        }
    }
    let state = WrapState {
        wraps,
        target: WrapTarget::Handler(&*handler),
        request,
    };
    state.next().await
}

impl<I: Accept + 'static> Server<I>
where
    I::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    I::Conn: AsyncRead + AsyncWrite + RemoteSocket + Unpin + Send + 'static,
{
    async fn request_phase(
        request_hooks: Vec<Arc<dyn RequestHook>>,
        wraps: Vec<Arc<dyn Wrap>>,
        outer_wraps: Vec<Arc<dyn Wrap>>,
        handler: Arc<dyn Handler>,
        request: &mut Request,
    ) -> Result<Response> {
        let outer_wrap_state = OuterWrapState {
            request_hooks,
            wraps,
            handler,
        };

        let state = WrapState {
            wraps: outer_wraps,
            target: WrapTarget::Phase(outer_wrap_state),
            request,
        };
        state.next().await
    }

    async fn handle_error(
        observed: &ObservedRoute<'_>,
        request: &mut Request,
        mut error: Error,
    ) -> Response {
        for middleware in &observed.error_hooks {
            match middleware.handle_error(request.parts(), &mut error).await {
                Ok(Some(x)) => return x,
                Err(Error::SkipMiddleware) | Ok(None) => (),
                Err(e) => {
                    log::error!("error hook middleware failure: {e}");
                }
            }
        }
        DefaultErrorHook
            .handle_error(request.parts(), &mut error)
            .await
            .unwrap()
            .unwrap()
    }

    async fn handle_early_response(
        observed: &ObservedRoute<'_>,
        request: &mut Request,
        mut response: Response,
    ) -> Response {
        for middleware in &observed.early_response_hooks {
            match middleware
                .handle_response(request.parts(), &mut response)
                .await
            {
                Err(Error::SkipMiddleware) | Ok(()) => (),
                Err(error) => {
                    return Self::handle_error(observed, request, error).await;
                }
            }
        }
        response
    }

    async fn handle_late_response(
        observed: &ObservedRoute<'_>,
        request: &mut Request,
        response: &mut Response,
    ) {
        for middleware in &observed.late_response_hooks {
            middleware.handle_response(request.parts(), response).await;
        }
    }

    async fn do_handle_axol_response(
        router: Arc<Router>,
        address: SocketAddr,
        request: HyperRequest<HyperBody>,
    ) -> Result<Response> {
        let (parts, body) = request.into_parts();
        let mut request = Request {
            method: parts
                .method
                .try_into()
                .map_err(|_| Error::Status(axol_http::StatusCode::MethodNotAllowed))?,
            uri: parts.uri,
            version: parts.version,
            headers: parts
                .headers
                .try_into()
                .map_err(|e: HeaderMapConvertError| Error::unprocessable_entity(e.to_string()))?,
            extensions: parts.extensions.into(),
            body: Body::Stream {
                size_hint: Some(<HyperBody as HttpBody>::size_hint(&body).lower() as usize),
                stream: Box::pin(BodyInputStream {
                    body,
                    data_ended: false,
                    trailers_ended: false,
                }),
            },
        };
        let mut observed = router.resolve_path(request.method, request.uri.path());
        for (_, value) in observed.variables.0.iter_mut() {
            let decoded = percent_encoding::percent_decode_str(value)
                .decode_utf8()
                .map_err(|_| Error::BadUtf8)?;
            if let Cow::Owned(decoded) = decoded {
                *value = decoded;
            }
        }
        //TODO: make this extension gathering more efficient
        request.extensions.extend(&observed.extensions);
        request
            .extensions
            .insert(RawPathExt(std::mem::take(&mut observed.variables.0)));
        request.extensions.insert(ConnectInfo(address));

        #[cfg(feature = "tracing")]
        let remote = address;
        #[cfg(feature = "tracing")]
        let span = tracing::trace_span!("axol_http", %remote, %request.uri);

        let wraps = std::mem::take(&mut observed.wraps);
        let outer_wraps = std::mem::take(&mut observed.outer_wraps);
        let request_hooks = std::mem::take(&mut observed.request_hooks);

        // we are not passing any interior mutability or mutability into the catch_unwind.
        // (that isn't dropped inside if a panic occurs)
        // TODO: this might not be a good idea, analyze how this could interact with application code
        let late_response = AssertUnwindSafe(async move {
            let mut late_response = match Self::request_phase(
                request_hooks,
                wraps,
                outer_wraps,
                observed.route.clone(),
                &mut request,
            )
            .await
            {
                Ok(x) => Self::handle_early_response(&observed, &mut request, x).await,
                Err(error) => Self::handle_error(&observed, &mut request, error).await,
            };
            Self::handle_late_response(&observed, &mut request, &mut late_response).await;
            late_response
        })
        .catch_unwind();

        #[cfg(feature = "tracing")]
        let late_response = late_response.instrument(span.clone()).await;
        #[cfg(not(feature = "tracing"))]
        let late_response = late_response.await;
        #[cfg(feature = "tracing")]
        let _span = span.enter();
        // no more awaiting because of in-span

        let late_response = match late_response {
            Ok(x) => x,
            Err(e) => {
                let display = e
                    .downcast::<String>()
                    .map(|x| *x)
                    .or_else(|e| e.downcast::<&'static str>().map(|x| x.to_string()))
                    .unwrap_or_else(|e| format!("{e:?}"));
                error!("panic during handler/middlware: {display}");
                StatusCode::InternalServerError.into_response().unwrap()
            }
        };

        Ok(late_response)
    }

    async fn do_handle(
        router: Arc<Router>,
        address: SocketAddr,
        request: HyperRequest<HyperBody>,
    ) -> Result<HyperResponse<BodyWrapper>, Infallible> {
        let is_head = request.method() == axol_http::http::Method::HEAD;
        let mut response = match Self::do_handle_axol_response(router, address, request).await {
            Ok(x) => x,
            Err(e) => e.into_response(),
        };

        if is_head {
            std::mem::take(&mut response.body);
        }

        let status: axol_http::http::StatusCode = response.status.into();
        let mut builder = HyperResponse::builder()
            .status(status)
            .version(response.version);
        *builder.headers_mut().unwrap() = response.headers.into();
        //TODO: due to limitations in converting Arc <-> Box for unsized types, we can't pass extensions back atm
        *builder.extensions_mut().unwrap() = Default::default();
        Ok(builder
            .body(response.body.into())
            .expect("body conversion failed"))
    }

    pub async fn serve(self) -> Result<(), hyper::Error> {
        self.serve_custom(|x| x).await
    }

    pub async fn serve_custom(
        mut self,
        customize: impl FnOnce(Builder<I>) -> Builder<I>,
    ) -> Result<(), hyper::Error> {
        self.router.set_paths("");
        let router = Arc::new(self.router);
        let service = hyper::service::make_service_fn(move |conn: &I::Conn| {
            let addr = conn.remote_addr();
            let router = router.clone();
            let service =
                hyper::service::service_fn(move |req| Self::do_handle(router.clone(), addr, req));
            async move { Ok::<_, Infallible>(service) }
        });
        let mut builder = hyper::Server::builder(self.incoming);
        builder = customize(builder);
        builder.serve(service).await
    }
}
