use std::borrow::Cow;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use crate::Result;
use crate::{ConnectInfo, DefaultErrorHook, Error, ErrorHook, ObservedRoute, RawPathExt};
use axol_http::body::{BodyComponent, BodyWrapper};
use axol_http::header::HeaderMapConvertError;
use axol_http::{request::Request, response::Response};
use axol_http::{Body, Method};
use derive_builder::Builder;
use futures::Stream;
use hyper::body::HttpBody;
use hyper::server::accept::Accept;
use hyper::server::conn::AddrIncoming;
use hyper::server::Builder;
pub use hyper::Error as HyperError;
use hyper::{
    server::conn::AddrStream, Body as HyperBody, Request as HyperRequest, Response as HyperResponse,
};
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite};

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

impl<I: Accept + 'static> Server<I>
where
    I::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    I::Conn: AsyncRead + AsyncWrite + RemoteSocket + Unpin + Send + 'static,
{
    async fn request_phase(
        observed: &ObservedRoute<'_>,
        request: &mut Request,
    ) -> Result<Response> {
        for middleware in &observed.request_hooks {
            if let Some(response) = middleware.handle_request(request).await? {
                return Ok(response);
            }
        }
        let body = std::mem::take(&mut request.body);

        observed.route.call(request.parts(), body).await
    }

    async fn handle_error(
        observed: &ObservedRoute<'_>,
        request: &mut Request,
        mut error: Error,
    ) -> Response {
        for middleware in &observed.error_hooks {
            match middleware.handle_error(request.parts(), &mut error).await {
                Ok(Some(x)) => return x,
                Ok(None) => (),
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
                Ok(()) => (),
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
        if request.method != Method::Connect
            && (request.uri.scheme().is_some() || request.uri.host().is_some())
        {
            return Err(Error::UnprocessableEntity);
        }

        let mut observed = router.resolve_path(request.method, request.uri.path());
        for (_, value) in observed.variables.0.iter_mut() {
            let decoded = percent_encoding::percent_decode_str(value)
                .decode_utf8()
                .map_err(|_| Error::BadUtf8)?;
            if let Cow::Owned(decoded) = decoded {
                *value = decoded;
            }
        }
        request
            .extensions
            .insert(RawPathExt(std::mem::take(&mut observed.variables.0)));
        request.extensions.insert(ConnectInfo(address));

        let mut late_response = match Self::request_phase(&observed, &mut request).await {
            Ok(x) => Self::handle_early_response(&observed, &mut request, x).await,
            Err(error) => Self::handle_error(&observed, &mut request, error).await,
        };
        Self::handle_late_response(&observed, &mut request, &mut late_response).await;

        if request.method == Method::Head {
            late_response.body = Body::default();
        }

        Ok(late_response)
    }

    async fn do_handle(
        router: Arc<Router>,
        address: SocketAddr,
        request: HyperRequest<HyperBody>,
    ) -> Result<HyperResponse<BodyWrapper>, Infallible> {
        let response = match Self::do_handle_axol_response(router, address, request).await {
            Ok(x) => x,
            Err(e) => e.into_response(),
        };

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
        self,
        customize: impl FnOnce(Builder<I>) -> Builder<I>,
    ) -> Result<(), hyper::Error> {
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
