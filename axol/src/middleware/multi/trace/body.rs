use crate::Result;

use axol_http::{
    body::{BodyComponent, BodyStream},
    Body,
};
use futures::{ready, Stream};
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};
use tracing::{Level, Span};
use tracing_futures::Instrument;

pub struct TraceBody {
    inner: BodyStream,
    body_start: Instant,
    body_size: usize,
    is_response: bool,
}

impl TraceBody {
    pub fn wrap(span: Span, body: Body, is_response: bool) -> Body {
        let size_hint = match &body {
            Body::Bytes(x) => Some(x.len()),
            Body::Stream {
                size_hint,
                stream: _,
            } => *size_hint,
        };
        let stream = Self {
            inner: body.into_stream(),
            body_start: Instant::now(),
            body_size: 0,
            is_response,
        };
        Body::Stream {
            size_hint,
            stream: Box::pin(stream.instrument(span)),
        }
    }
}

impl Stream for TraceBody {
    type Item = Result<BodyComponent, anyhow::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next = self.inner.as_mut().poll_next(cx);

        let Some(next) = ready!(next) else {
            let body_elapsed_ms = self.body_start.elapsed().as_secs_f64() * 1000.0;

            if self.is_response {
                Span::current().record("http.response.body.size", self.body_size);
                Span::current().record("http.response.body.elapsed_ms", body_elapsed_ms);
            } else {
                Span::current().record("http.request.body.size", self.body_size);
                Span::current().record("http.request.body.elapsed_ms", body_elapsed_ms);
            }

            return Poll::Ready(None);
        };

        match &next {
            Ok(BodyComponent::Data(data)) => {
                self.body_size += data.len();
            }
            Ok(BodyComponent::Trailers(trailers)) => {
                tracing::event!(Level::DEBUG, ?trailers, "body trailers");
            }
            Err(err) => {
                tracing::event!(Level::ERROR, ?err, "body error");
            }
        }

        Poll::Ready(Some(next))
    }
}
