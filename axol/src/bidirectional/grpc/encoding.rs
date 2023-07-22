use axol_http::typed_headers::{Error as HeaderError, Header, HeaderName, HeaderValue};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Encoding {
    Identity,
    Gzip,
    Deflate,
    Snappy,
    Other(String),
}

impl Default for Encoding {
    fn default() -> Self {
        Encoding::Identity
    }
}

impl Encoding {
    fn from_str(value: &str) -> Self {
        match value {
            "identity" => Encoding::Identity,
            "gzip" => Encoding::Gzip,
            "deflate" => Encoding::Deflate,
            "snappy" => Encoding::Snappy,
            x => Encoding::Other(x.to_string()),
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Encoding::Identity => "identity",
            Encoding::Gzip => "gzip",
            Encoding::Deflate => "deflate",
            Encoding::Snappy => "snappy",
            Encoding::Other(x) => x,
        }
    }
}

static GRPC_ENCODING: HeaderName = HeaderName::from_static("grpc-encoding");
static GRPC_ACCEPT_ENCODING: HeaderName = HeaderName::from_static("grpc-accept-encoding");

impl Header for Encoding {
    fn name() -> &'static HeaderName {
        &GRPC_ENCODING
    }

    fn decode<'i, I: Iterator<Item = &'i HeaderValue>>(values: &mut I) -> Result<Self, HeaderError>
    where
        Self: Sized,
    {
        let value = values.next().ok_or_else(|| HeaderError::invalid())?;
        let value = value.to_str().map_err(|_| HeaderError::invalid())?;
        Ok(Encoding::from_str(value))
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        values.extend(std::iter::once(
            HeaderValue::from_str(self.as_str()).unwrap(),
        ));
    }
}

#[derive(Clone, Debug)]
pub struct AcceptEncoding(pub Vec<Encoding>);

impl Header for AcceptEncoding {
    fn name() -> &'static HeaderName {
        &GRPC_ACCEPT_ENCODING
    }

    fn decode<'i, I: Iterator<Item = &'i HeaderValue>>(values: &mut I) -> Result<Self, HeaderError>
    where
        Self: Sized,
    {
        let mut out = AcceptEncoding(vec![]);
        for value in values {
            let value = value.to_str().map_err(|_| HeaderError::invalid())?;
            for value in value.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()) {
                out.0.push(Encoding::from_str(value));
            }
        }
        Ok(out)
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        let raw = self
            .0
            .iter()
            .map(|x| x.as_str())
            .collect::<Vec<_>>()
            .join(",");
        values.extend(std::iter::once(HeaderValue::from_str(&raw).unwrap()));
    }
}
