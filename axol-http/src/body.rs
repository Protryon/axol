use std::{fmt, pin::Pin, task::{Context, Poll}};

use bytes::Bytes;
use futures::{Stream, StreamExt, TryStreamExt};
use http_body::SizeHint;

use crate::header::HeaderMap;

#[derive(Debug)]
pub enum BodyComponent {
    Data(Bytes),
    Trailers(HeaderMap),
}

//TODO: docs
pub enum Body {
    Bytes(Vec<u8>),
    Stream {
        size_hint: Option<usize>,
        stream: Pin<
            Box<dyn Stream<Item = Result<BodyComponent, anyhow::Error>> + Send + Sync + 'static>,
        >,
    },
}

/// Wrapper over `Body` that implements `http_body::Body`
pub struct BodyWrapper {
    body: Body,
    has_written_blob: bool,
    bytes_streamed: usize,
    pending_trailers: Option<HeaderMap>,
    eof: bool,
}

impl From<Body> for BodyWrapper {
    fn from(body: Body) -> Self {
        Self {
            body,
            has_written_blob: false,
            bytes_streamed: 0,
            pending_trailers: None,
            eof: false,
        }
    }
}

impl Into<Body> for BodyWrapper {
    fn into(self) -> Body {
        self.body
    }
}

impl Body {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub async fn collect(self) -> Result<Vec<u8>, anyhow::Error> {
        match self {
            Body::Bytes(x) => Ok(x),
            Body::Stream {
                size_hint,
                mut stream,
            } => {
                let mut out = Vec::with_capacity(size_hint.unwrap_or_default());
                while let Some(component) = stream.next().await.transpose()? {
                    match component {
                        BodyComponent::Data(data) => {
                            out.extend_from_slice(&data[..]);
                        }
                        BodyComponent::Trailers(_) => (),
                    }
                }
                Ok(out)
            }
        }
    }

    pub fn into_stream(self) -> Pin<Box<dyn Stream<Item = Result<BodyComponent, anyhow::Error>> + Send + Sync + 'static>> {
        match self {
            Body::Bytes(bytes) => Box::pin(futures::stream::once(async move { Ok(BodyComponent::Data(bytes.into())) })),
            Body::Stream { size_hint: _, stream } => stream,
        }
    }
}

impl Into<Body> for Vec<u8> {
    fn into(self) -> Body {
        Body::Bytes(self)
    }
}

impl Into<Body> for String {
    fn into(self) -> Body {
        Body::Bytes(self.into_bytes())
    }
}

impl Into<Body> for &str {
    fn into(self) -> Body {
        Body::Bytes(self.as_bytes().to_vec())
    }
}

impl Into<Body> for () {
    fn into(self) -> Body {
        Body::Bytes(vec![])
    }
}

impl From<Pin<Box<dyn Stream<Item = Result<BodyComponent, anyhow::Error>> + Send + Sync + 'static>>>
    for Body
{
    fn from(
        stream: Pin<
            Box<dyn Stream<Item = Result<BodyComponent, anyhow::Error>> + Send + Sync + 'static>,
        >,
    ) -> Self {
        Self::Stream {
            size_hint: None,
            stream,
        }
    }
}

impl From<Pin<Box<dyn Stream<Item = Result<Bytes, anyhow::Error>> + Send + Sync + 'static>>>
    for Body
{
    fn from(
        value: Pin<Box<dyn Stream<Item = Result<Bytes, anyhow::Error>> + Send + Sync + 'static>>,
    ) -> Self {
        Self::Stream {
            size_hint: None,
            stream: Box::pin(value.map_ok(|x| BodyComponent::Data(x))),
        }
    }
}

impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bytes(arg0) => f.debug_tuple("Bytes").field(arg0).finish(),
            Self::Stream { size_hint, .. } => f.debug_tuple("Stream").field(size_hint).finish(),
        }
    }
}

impl Default for Body {
    fn default() -> Self {
        Body::Bytes(vec![])
    }
}

impl http_body::Body for BodyWrapper {
    type Data = Bytes;

    type Error = anyhow::Error;

    fn poll_data(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        match &mut self.body {
            Body::Bytes(bytes) => {
                let bytes = std::mem::take(bytes);
                if self.has_written_blob {
                    return Poll::Ready(None);
                }
                self.eof = true;
                self.has_written_blob = true;
                Poll::Ready(Some(Ok(bytes.into())))
            },
            Body::Stream { size_hint: _, stream } => {
                match stream.poll_next_unpin(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(None) => {
                        self.eof = true;
                        Poll::Ready(None)
                    },
                    Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
                    Poll::Ready(Some(Ok(BodyComponent::Data(data)))) => {
                        self.bytes_streamed += data.len();
                        Poll::Ready(Some(Ok(data)))
                    },
                    Poll::Ready(Some(Ok(BodyComponent::Trailers(trailers)))) => {
                        self.pending_trailers = Some(trailers);
                        Poll::Ready(None)
                    },
                }
            }
        }
    }

    fn poll_trailers(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<Option<headers::HeaderMap>, Self::Error>> {
        self.eof = true;
        if let Some(pending_trailers) = self.pending_trailers.take() {
            Poll::Ready(Ok(Some(pending_trailers.into())))
        } else {
            Poll::Ready(Ok(None))
        }
    }

    fn is_end_stream(&self) -> bool {
        self.eof
    }

    fn size_hint(&self) -> SizeHint {
        if self.eof {
            return SizeHint::with_exact(0);
        }
        match &self.body {
            Body::Bytes(bytes) => {
                SizeHint::with_exact(bytes.len() as u64)
            },
            Body::Stream { size_hint, stream: _ } => {
                size_hint.map(|x| SizeHint::with_exact(x as u64)).unwrap_or_default()
            }
        }
    }
}