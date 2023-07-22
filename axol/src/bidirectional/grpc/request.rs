use crate::{Error, FromRequest, FromRequestParts, Result, Typed};
use axol_http::{request::RequestPartsRef, Body};
use prost::Message;

use super::{
    AcceptEncoding, Encoding, Grpc, GrpcContentType, GrpcTimeout, MessageType, Metadata, Status,
};

#[derive(Debug)]
pub struct GrpcRequest<T: Message + Default> {
    pub timeout: Option<GrpcTimeout>,
    pub service_name: String,
    pub method_name: String,
    pub content_type: GrpcContentType,
    pub message_type: Option<MessageType>,
    pub message_encoding: Encoding,
    pub message_accept_encoding: Option<AcceptEncoding>,
    pub metadata: Metadata,
    pub body: T,
}

#[async_trait::async_trait]
impl<'a, T: Message + Default + 'a> FromRequest<'a> for GrpcRequest<T> {
    async fn from_request(request: RequestPartsRef<'a>, body: Body) -> Result<Self> {
        let path = request.uri.path();
        let path = path.strip_prefix("/").unwrap_or(path);
        let Some((service_name, method_name)) = path.split_once('/') else {
            //todo: use grpc status instead
            return Err(Error::Grpc(Status::Unimplemented));
        };
        let Grpc {
            encoding,
            content_type,
            body,
        } = Grpc::<T>::from_request(request, body).await?;
        Ok(GrpcRequest {
            timeout: Option::<Typed<GrpcTimeout>>::from_request_parts(request)
                .await?
                .map(|x| x.0),
            service_name: service_name.to_string(),
            method_name: method_name.to_string(),
            content_type,
            message_type: Option::<Typed<MessageType>>::from_request_parts(request)
                .await?
                .map(|x| x.0),
            message_encoding: encoding,
            message_accept_encoding: Option::<Typed<AcceptEncoding>>::from_request_parts(request)
                .await?
                .map(|x| x.0),
            metadata: Metadata::from_request_parts(request).await?,
            body,
        })
    }
}
