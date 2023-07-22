use axol_http::{
    header::HeaderMap,
    response::Response,
    typed_headers::{Error as HeaderError, Header, HeaderName, HeaderValue},
    Body,
};
use percent_encoding::NON_ALPHANUMERIC;

use crate::{IntoResponse, Result};

use super::GrpcContentType;

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    strum::FromRepr,
    Default,
    strum::Display,
    strum::IntoStaticStr,
)]
#[repr(u8)]
pub enum Status {
    /// Not an error; returned on success.
    #[default]
    Ok = 0,

    /// The operation was cancelled, typically by the caller.
    Cancelled = 1,

    /// Unknown error. For example, this error may be returned when a Status value received from another address space belongs to an error space that is not known in this address space. Also errors raised by APIs that do not return enough error information may be converted to this error.
    Unknown = 2,

    /// The client specified an invalid argument. Note that this differs from FAILED_PRECONDITION. INVALID_ARGUMENT indicates arguments that are problematic regardless of the state of the system (e.g., a malformed file name).
    InvalidArgument = 3,

    /// The deadline expired before the operation could complete. For operations that change the state of the system, this error may be returned even if the operation has completed successfully. For example, a successful response from a server could have been delayed long
    DeadlineExceeded = 4,

    /// Some requested entity (e.g., file or directory) was not found. Note to server developers: if a request is denied for an entire class of users, such as gradual feature rollout or undocumented allowlist, NOT_FOUND may be used. If a request is denied for some users within a class of users, such as user-based access control, PERMISSION_DENIED must be used.
    NotFound = 5,

    /// The entity that a client attempted to create (e.g., file or directory) already exists.
    AlreadyExists = 6,

    /// The caller does not have permission to execute the specified operation. PERMISSION_DENIED must not be used for rejections caused by exhausting some resource (use RESOURCE_EXHAUSTED instead for those errors). PERMISSION_DENIED must not be used if the caller can not be identified (use UNAUTHENTICATED instead for those errors). This error code does not imply the request is valid or the requested entity exists or satisfies other pre-conditions.
    PermissionDenied = 7,

    /// Some resource has been exhausted, perhaps a per-user quota, or perhaps the entire file system is out of space.
    ResourceExhausted = 8,

    /// The operation was rejected because the system is not in a state required for the operation's execution. For example, the directory to be deleted is non-empty, an rmdir operation is applied to a non-directory, etc. Service implementors can use the following guidelines to decide between FAILED_PRECONDITION, ABORTED, and UNAVAILABLE: (a) Use UNAVAILABLE if the client can retry just the failing call. (b) Use ABORTED if the client should retry at a higher level (e.g., when a client-specified test-and-set fails, indicating the client should restart a read-modify-write sequence). (c) Use FAILED_PRECONDITION if the client should not retry until the system state has been explicitly fixed. E.g., if an "rmdir" fails because the directory is non-empty, FAILED_PRECONDITION should be returned since the client should not retry unless the files are deleted from the directory.
    FailedPrecondition = 9,

    /// The operation was aborted, typically due to a concurrency issue such as a sequencer check failure or transaction abort. See the guidelines above for deciding between FAILED_PRECONDITION, ABORTED, and UNAVAILABLE.
    Aborted = 10,

    /// The operation was attempted past the valid range. E.g., seeking or reading past end-of-file. Unlike INVALID_ARGUMENT, this error indicates a problem that may be fixed if the system state changes. For example, a 32-bit file system will generate INVALID_ARGUMENT if asked to read at an offset that is not in the range [0,2^32-1], but it will generate OUT_OF_RANGE if asked to read from an offset past the current file size. There is a fair bit of overlap between FAILED_PRECONDITION and OUT_OF_RANGE. We recommend using OUT_OF_RANGE (the more specific error) when it applies so that callers who are iterating through a space can easily look for an OUT_OF_RANGE error to detect when they are done.
    OutOfRange = 11,

    /// The operation is not implemented or is not supported/enabled in this service.
    Unimplemented = 12,

    /// Internal errors. This means that some invariants expected by the underlying system have been broken. This error code is reserved for serious errors.
    Internal = 13,

    /// The service is currently unavailable. This is most likely a transient condition, which can be corrected by retrying with a backoff. Note that it is not always safe to retry non-idempotent operations.
    Unavailable = 14,

    /// Unrecoverable data loss or corruption.
    DataLoss = 15,

    /// The request does not have valid authentication credentials for the operation.
    Unauthenticated = 16,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        self.into()
    }
}

impl IntoResponse for Status {
    fn into_response(self) -> Result<Response> {
        let mut trailers = HeaderMap::new();
        trailers.insert_typed(&self);

        let mut out = Response::new(Body::trailers(trailers));
        out.headers.insert_typed(&GrpcContentType::Proto);
        out.extensions.insert(self);
        Ok(out)
    }
}

static GRPC_STATUS: HeaderName = HeaderName::from_static("grpc-status");

impl Header for Status {
    fn name() -> &'static HeaderName {
        &GRPC_STATUS
    }

    fn decode<'i, I: Iterator<Item = &'i HeaderValue>>(values: &mut I) -> Result<Self, HeaderError>
    where
        Self: Sized,
    {
        let value = values.next().ok_or_else(|| HeaderError::invalid())?;
        let value = value.to_str().map_err(|_| HeaderError::invalid())?;
        let value: u8 = value.parse().map_err(|_| HeaderError::invalid())?;
        let value = Status::from_repr(value).unwrap_or(Status::Unknown);
        Ok(value)
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        values.extend(std::iter::once(
            HeaderValue::from_str(&(*self as u8).to_string()).unwrap(),
        ));
    }
}

#[derive(Clone, Debug)]
pub struct StatusMessage(pub String);

static GRPC_MESSAGE: HeaderName = HeaderName::from_static("grpc-message");

impl Header for StatusMessage {
    fn name() -> &'static HeaderName {
        &GRPC_MESSAGE
    }

    fn decode<'i, I: Iterator<Item = &'i HeaderValue>>(values: &mut I) -> Result<Self, HeaderError>
    where
        Self: Sized,
    {
        let value = values.next().ok_or_else(|| HeaderError::invalid())?;
        let value = value.to_str().map_err(|_| HeaderError::invalid())?;
        let value = percent_encoding::percent_decode_str(value)
            .decode_utf8_lossy()
            .into_owned();
        Ok(Self(value))
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        values.extend(std::iter::once(
            HeaderValue::from_str(
                &percent_encoding::utf8_percent_encode(&self.0, NON_ALPHANUMERIC).to_string(),
            )
            .unwrap(),
        ));
    }
}

impl IntoResponse for (Status, StatusMessage) {
    fn into_response(self) -> Result<Response> {
        let mut trailers = HeaderMap::new();
        trailers.insert_typed(&self.0);
        trailers.insert_typed(&self.1);

        let mut out = Response::new(Body::trailers(trailers));
        out.headers.insert_typed(&GrpcContentType::Proto);
        out.extensions.insert(self.0);
        out.extensions.insert(self.1);
        Ok(out)
    }
}
