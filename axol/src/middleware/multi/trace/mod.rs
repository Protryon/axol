mod body;

use std::{borrow::Cow, sync::Arc};

use anyhow::anyhow;
use axol_http::{
    request::{Request, RequestPartsRef},
    response::Response,
    Body, Version,
};
use opentelemetry::{Key, StringValue, Value};
use tracing::{field::Empty, Instrument, Level, Span};
use tracing_subscriber::registry::{Data, LookupSpan, SpanData};

use crate::{
    trace::body::TraceBody, ConnectInfo, Error, LateResponseHook, MatchedPath, Plugin, RequestHook,
    Result, Router, Wrap, WrapState,
};
use tracing_opentelemetry::{OpenTelemetrySpanExt, OtelData};

struct TraceInfo {
    span: Span,
}

#[derive(Clone)]
pub struct Trace {
    pub request_header_filter:
        Arc<dyn for<'a> Fn(&str, &'a str) -> Option<Cow<'a, str>> + Send + Sync + 'static>,
    pub response_header_filter:
        Arc<dyn for<'a> Fn(&str, &'a str) -> Option<Cow<'a, str>> + Send + Sync + 'static>,
}

pub fn default_request_header_filter<'a>(name: &str, value: &'a str) -> Option<Cow<'a, str>> {
    match name {
        "authorization" => Some(Cow::Borrowed("present")),
        "cookie" => Some(Cow::Borrowed("present")),
        "user-agent" => None,
        _ => Some(Cow::Borrowed(value)),
    }
}

pub fn default_response_header_filter<'a>(name: &str, value: &'a str) -> Option<Cow<'a, str>> {
    match name {
        "set-cookie" => Some(Cow::Borrowed("present")),
        _ => Some(Cow::Borrowed(value)),
    }
}

pub fn allow_all_header_filter<'a>(_name: &str, value: &'a str) -> Option<Cow<'a, str>> {
    Some(Cow::Borrowed(value))
}

pub fn deny_all_header_filter<'a>(_name: &str, _value: &'a str) -> Option<Cow<'a, str>> {
    None
}

impl Default for Trace {
    fn default() -> Self {
        Self {
            request_header_filter: Arc::new(default_request_header_filter),
            response_header_filter: Arc::new(default_response_header_filter),
        }
    }
}

impl Trace {
    pub fn response_header_filter<F>(mut self, func: F) -> Self
    where
        for<'a> F: Fn(&str, &'a str) -> Option<Cow<'a, str>> + Send + Sync + 'static,
    {
        self.response_header_filter = Arc::new(func);
        self
    }

    pub fn request_header_filter<F>(mut self, func: F) -> Self
    where
        for<'a> F: Fn(&str, &'a str) -> Option<Cow<'a, str>> + Send + Sync + 'static,
    {
        self.request_header_filter = Arc::new(func);
        self
    }
}

pub fn http_flavor(version: Version) -> Cow<'static, str> {
    match version {
        Version::HTTP_09 => "0.9".into(),
        Version::HTTP_10 => "1.0".into(),
        Version::HTTP_11 => "1.1".into(),
        Version::HTTP_2 => "2.0".into(),
        Version::HTTP_3 => "3.0".into(),
        other => format!("{other:?}").into(),
    }
}

impl Trace {
    fn make_span(&self, request: RequestPartsRef<'_>) -> Span {
        let host = request
            .headers
            .get("host")
            .or(request.uri.host())
            .unwrap_or_default();
        let port = request.uri.port().map(|x| x.as_u16());
        let connect_info = request
            .extensions
            .get::<ConnectInfo>()
            .map(|x| x.0.ip().to_string());
        let user_agent = request.headers.get("user-agent");
        let scheme = request.uri.scheme().map(|x| x.as_str());
        let route = request.extensions.get::<MatchedPath>().map(|x| &**x.0);
        let name = format!("{} {}", request.method, route.unwrap_or_default());
        let span = tracing::info_span!(
            target: "otel::tracing",
            "HTTP request",
            http.request.method = %request.method,
            http.route = route,
            network.protocol.version = %http_flavor(request.version),
            server.address = host,
            server.port = port,
            http.client.address = connect_info,
            user_agent.original = user_agent,
            url.path = request.uri.path(),
            url.query = request.uri.query(),
            url.scheme = scheme,
            otel.name = name,
            otel.kind = ?opentelemetry_api::trace::SpanKind::Server,
            http.response.status_code = Empty, // to set on response
            otel.status_code = Empty, // to set on response
            trace_id = Empty, // to set on response
            request_id = Empty, // to set
            exception.message = Empty, // to set on response
            rpc.system = Empty,
            rpc.service = Empty,
            rpc.method = Empty,
            http.grpc_status = Empty,
            http.request.body.size = Empty,
            http.response.body.size = Empty,
            http.request.body.elapsed_ms = Empty,
            http.response.body.elapsed_ms = Empty,
        );
        if let Some(span_id) = span.id() {
            tracing::dispatcher::get_default(|dispatch| {
                println!("got dispatch {span_id:?}");
                if let Some(lookup) = dispatch.downcast_ref::<&dyn LookupSpan<Data = Data<'_>>>() {
                    println!("got lookup");
                    let span = lookup.span_data(&span_id).expect("missing span");
                    let mut extensions = span.extensions_mut();
                    if let Some(data) = extensions.get_mut::<OtelData>() {
                        println!("got otel data");
                        let target = data.builder.attributes.as_mut().unwrap();
                        for (name, values) in request.headers.grouped() {
                            let values: Vec<StringValue> = values
                                .into_iter()
                                .filter_map(|value| (self.request_header_filter)(name, value))
                                .map(|x| StringValue::from(x.to_string()))
                                .collect::<Vec<_>>();
                            if values.is_empty() {
                                continue;
                            }
                            println!("push header {name}");
                            //todo: use static header values?
                            target.insert(
                                Key::new(format!("http.request.header.{}", name.replace('-', "_"))),
                                Value::Array(values.into()),
                            );
                        }
                    }
                }
            });
        }
        span.set_parent(opentelemetry_api::global::get_text_map_propagator(
            |propagator| propagator.extract(&request.headers),
        ));
        span
    }
}

#[async_trait::async_trait]
impl RequestHook for Trace {
    async fn handle_request(&self, request: &mut Request) -> Result<Option<Response>> {
        let span = self.make_span(request.parts());
        request.extensions.insert(TraceInfo { span });
        Ok(None)
    }
}

#[async_trait::async_trait]
impl Wrap for Trace {
    async fn wrap(
        &self,
        request: RequestPartsRef<'_>,
        body: Body,
        state: WrapState<'_>,
    ) -> Result<Response> {
        let Some(info) = request.extensions.get::<TraceInfo>() else {
            return Err(Error::internal(anyhow!("missing trace extension")));
        };
        let out = {
            let body = TraceBody::wrap(info.span.clone(), body, false);
            info.span.in_scope(|| {
                tracing::event!(Level::DEBUG, "started processing request");
            });
            let out = state.next(body).instrument(info.span.clone()).await;
            out
        };
        out
    }
}

#[async_trait::async_trait]
impl LateResponseHook for Trace {
    async fn handle_response<'a>(&self, request: RequestPartsRef<'a>, response: &mut Response) {
        let Some(info) = request.extensions.get::<TraceInfo>() else {
            return;
        };

        let mut is_grpc = false;

        #[cfg(feature = "grpc")]
        {
            if let Some(status) = response.extensions.get::<crate::grpc::Status>().copied() {
                is_grpc = true;
                info.span.record("http.grpc_status", status.as_str());
                info.span.record("rpc.system", "grpc");
                let mut path_segments = request.uri.path().split('/').filter(|x| !x.is_empty());
                info.span
                    .record("rpc.service", path_segments.next().unwrap_or_default());
                info.span
                    .record("rpc.method", path_segments.next().unwrap_or_default());
            }
        }
        #[cfg(feature = "grpc")]
        {
            if let Some(status) = response.extensions.get::<crate::grpc::StatusMessage>() {
                info.span.record("exception.message", &status.0);
            }
        }

        info.span.record(
            "http.response.status_code",
            &tracing::field::display(response.status.as_u16()),
        );
        if response.status.is_server_error() {
            info.span.record("otel.status_code", "ERROR");
        } else if is_grpc {
            info.span.record("otel.status_code", "OK");
        }

        if let Some(span_id) = info.span.id() {
            tracing::dispatcher::get_default(|dispatch| {
                if let Some(lookup) = dispatch.downcast_ref::<&dyn LookupSpan<Data = Data<'_>>>() {
                    let span = lookup.span_data(&span_id).expect("missing span");
                    let mut extensions = span.extensions_mut();
                    if let Some(data) = extensions.get_mut::<OtelData>() {
                        let target = data.builder.attributes.as_mut().unwrap();
                        for (name, values) in response.headers.grouped() {
                            let values: Vec<StringValue> = values
                                .into_iter()
                                .filter_map(|value| (self.response_header_filter)(name, value))
                                .map(|x| StringValue::from(x.to_string()))
                                .collect::<Vec<_>>();
                            if values.is_empty() {
                                continue;
                            }

                            //todo: use static header values?
                            target.insert(
                                Key::new(format!(
                                    "http.response.header.{}",
                                    name.replace('-', "_")
                                )),
                                Value::Array(values.into()),
                            );
                        }
                    }
                }
            });
        }

        response.body =
            TraceBody::wrap(info.span.clone(), std::mem::take(&mut response.body), true);
    }
}

impl Plugin for Trace {
    fn apply(self, router: Router, path: &str) -> Router {
        router
            .request_hook_direct(path, self.clone())
            .late_response_hook_direct(path, self.clone())
            .wrap(path, self.clone())
    }
}
