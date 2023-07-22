use axol_http::typed_headers::{Error as HeaderError, Header, HeaderName, HeaderValue};

#[derive(Clone, Debug)]
pub struct MessageType(pub String);

static GRPC_MESSAGE_TYPE: HeaderName = HeaderName::from_static("grpc-message-type");

impl Header for MessageType {
    fn name() -> &'static HeaderName {
        &GRPC_MESSAGE_TYPE
    }

    fn decode<'i, I: Iterator<Item = &'i HeaderValue>>(values: &mut I) -> Result<Self, HeaderError>
    where
        Self: Sized,
    {
        let value = values.next().ok_or_else(|| HeaderError::invalid())?;
        let value = value.to_str().map_err(|_| HeaderError::invalid())?;
        Ok(MessageType(value.to_string()))
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        values.extend(std::iter::once(HeaderValue::from_str(&self.0).unwrap()));
    }
}
