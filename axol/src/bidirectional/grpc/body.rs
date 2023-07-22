use axol_http::{request::RequestPartsRef, response::Response, Body};
use prost::Message;

use crate::{grpc::Status, Error, FromRequest, FromRequestParts, IntoResponse, Result, Typed};

use super::{Encoding, GrpcContentType};

pub struct Grpc<T: Message> {
    pub encoding: Encoding,
    pub content_type: GrpcContentType,
    pub body: T,
}

#[async_trait::async_trait]
impl<'a, T: Default + Message + 'a> FromRequest<'a> for Grpc<T> {
    async fn from_request(request: RequestPartsRef<'a>, body: Body) -> Result<Self> {
        let content_type = Typed::<GrpcContentType>::from_request_parts(request)
            .await?
            .0;
        let encoding = Option::<Typed<Encoding>>::from_request_parts(request)
            .await?
            .map(|x| x.0)
            .unwrap_or(Encoding::Identity);

        if content_type == GrpcContentType::Invalid {
            return Err(Error::UnsupportedMediaType);
        } else if content_type != GrpcContentType::Proto {
            return Err(Error::GrpcMessage(
                Status::Internal,
                "only proto subtype is supported".to_string(),
            ));
        }

        if encoding != Encoding::Identity {
            //TODO: support more encodings
            fn x() {}
            return Err(Error::GrpcMessage(
                Status::Internal,
                "only identity encoding is supported".to_string(),
            ));
        }

        let body = body.collect().await?;

        if body.len() < 5 {
            return Err(Error::GrpcMessage(
                Status::Internal,
                "truncated body".to_string(),
            ));
        }
        let is_compressed = match body[0] {
            0 => false,
            1 => true,
            _ => {
                return Err(Error::GrpcMessage(
                    Status::Internal,
                    "invalid compression flag".to_string(),
                ));
            }
        };
        if is_compressed != (encoding != Encoding::Identity) {
            return Err(Error::GrpcMessage(
                Status::Internal,
                "compression flag mismatch with headers".to_string(),
            ));
        }
        let length = u32::from_be_bytes((&body[1..5]).try_into().unwrap());
        //TODO: should we assert length == body.len() - 5?
        let body: T = T::decode(&body[5..5 + length as usize])
            .map_err(|e| Error::GrpcMessage(Status::Internal, format!("decode failure: {e}")))?;
        Ok(Self {
            encoding,
            content_type,
            body,
        })
    }
}

impl<T: Message + Default> IntoResponse for Grpc<T> {
    fn into_response(self) -> Result<Response> {
        let is_compressed = self.encoding != Encoding::Identity;

        let mut out = vec![is_compressed as u8, 0, 0, 0, 0];
        self.body
            .encode(&mut out)
            .map_err(|e| Error::GrpcMessage(Status::Internal, format!("encode failure: {e}")))?;
        let out_len = (out.len() - 5) as u32;
        out[1..5].copy_from_slice(&out_len.to_be_bytes()[..]);
        // must return a Body::Bytes or we will break downstream stuff
        let mut response = Response::new(Body::Bytes(out));
        response.headers.insert_typed(&self.content_type);
        response.headers.insert_typed(&self.encoding);
        Ok(response)
    }
}
