use std::{fmt, pin::Pin};

use bytes::Bytes;
use futures::{Stream, StreamExt, TryStreamExt};

use crate::header::HeaderMap;

#[derive(Debug)]
pub enum BodyComponent {
    Data(Bytes),
    Trailers(HeaderMap),
}

//TODO: docs
pub enum Body {
    Bytes(Vec<u8>),
    //TODO: do we really want to use std::io::Error here?
    Stream {
        size_hint: Option<usize>,
        stream: Pin<
            Box<dyn Stream<Item = Result<BodyComponent, anyhow::Error>> + Send + Sync + 'static>,
        >,
    },
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
