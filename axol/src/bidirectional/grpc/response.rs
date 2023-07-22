use crate::{grpc::StatusMessage, IntoResponse, IntoResponseParts, Result};
use axol_http::{header::HeaderMap, response::Response, Body};
use prost::Message;

use super::{AcceptEncoding, Encoding, Grpc, GrpcContentType, Metadata, Status};

#[derive(Default, Debug)]
pub struct GrpcResponse<T: Message + Default> {
    pub content_type: GrpcContentType,
    pub message_encoding: Encoding,
    pub message_accept_encoding: Option<AcceptEncoding>,
    pub header_metadata: Metadata,
    pub body: T,
    pub trailer_metadata: Metadata,
    pub status: Status,
    pub status_message: Option<String>,
}

impl<T: Message + Default> IntoResponse for GrpcResponse<T> {
    fn into_response(self) -> Result<Response> {
        let mut out = Grpc {
            content_type: self.content_type,
            encoding: self.message_encoding,
            body: self.body,
        }
        .into_response()?;
        let Body::Bytes(body) = std::mem::take(&mut out.body) else {
            panic!("Grpc encode did not return a Body::Bytes");
        };
        if let Some(message_accept_encoding) = &self.message_accept_encoding {
            out.headers.insert_typed(message_accept_encoding);
        }
        self.header_metadata
            .into_response_parts(&mut out.parts_mut())?;
        let mut trailers = HeaderMap::new();
        trailers.insert_typed(&self.status);
        out.extensions.insert(self.status);
        let status_message = self.status_message.map(StatusMessage);
        if let Some(status_message) = status_message {
            trailers.insert_typed(&status_message);
            out.extensions.insert(status_message);
        }
        self.trailer_metadata.append_to(&mut trailers)?;
        out.body = Body::bytes_and_trailers(body, trailers);
        Ok(out)
    }
}
