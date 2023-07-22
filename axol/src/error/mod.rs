use axol_http::{response::Response, StatusCode, Uri};
use url::Url;

use crate::{AppendHeader, IntoResponse};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, strum::Display)]
pub enum RedirectMode {
    MovedPermanently,
    Found,
    SeeOther,
    #[default]
    TemporaryRedirect,
    PermanentRedirect,
}

impl RedirectMode {
    pub fn status(self) -> StatusCode {
        match self {
            RedirectMode::MovedPermanently => StatusCode::MovedPermanently,
            RedirectMode::Found => StatusCode::Found,
            RedirectMode::SeeOther => StatusCode::SeeOther,
            RedirectMode::TemporaryRedirect => StatusCode::TemporaryRedirect,
            RedirectMode::PermanentRedirect => StatusCode::PermanentRedirect,
        }
    }
}

impl IntoResponse for (RedirectMode, Uri) {
    fn into_response(self) -> Result<Response> {
        (
            AppendHeader("location", self.1.to_string()),
            self.0.status(),
        )
            .into_response()
    }
}

impl IntoResponse for (RedirectMode, Url) {
    fn into_response(self) -> Result<Response> {
        (
            AppendHeader("location", self.1.to_string()),
            self.0.status(),
        )
            .into_response()
    }
}

impl IntoResponse for Url {
    fn into_response(self) -> Result<Response> {
        (RedirectMode::TemporaryRedirect, self).into_response()
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Placeholder default for when an error has been taken
    #[error("not an error (you shouldn't see this)")]
    NotAnError,

    /// Set redirect status code and set `location` header.
    #[error("redirect {0} {1}")]
    Redirect(RedirectMode, Uri),
    /// Set redirect status code and set `location` header. (with `url::Url` type)
    #[error("redirect {0} {1}")]
    RedirectUrl(RedirectMode, Url),
    // common status codes with no body

    // 4xx
    #[error("400 Bad Request")]
    BadRequest,
    #[error("401 Unauthorized")]
    Unauthorized,
    #[error("403 Forbidden")]
    Forbidden,
    #[error("404 Not Found")]
    NotFound,
    #[error("405 Method Not Allowed")]
    MethodNotAllowed,
    #[error("406 Not Acceptable")]
    NotAcceptable,
    #[error("408 Request Timeout")]
    RequestTimeout,
    #[error("409 Conflict")]
    Conflict,
    #[error("410 Gone")]
    Gone,
    #[error("412 Precondition Failed")]
    PreconditionFailed,
    #[error("413 Payload Too Large")]
    PayloadTooLarge,
    #[error("414 Uri Too Long")]
    UriTooLong,
    #[error("415 Unsupported Media Type")]
    UnsupportedMediaType,
    #[error("416 Range Not Satisfiable")]
    RangeNotSatisfiable,
    #[error("417 Expectation Failed")]
    ExpectationFailed,
    #[error("422 Unprocessable Entity")]
    UnprocessableEntity,
    #[error("429 Too Many Requests")]
    TooManyRequests,
    #[error("451 Unavailable For Legal Reasons")]
    UnavailableForLegalReasons,

    // 5xx
    #[error("500 Internal Server Error")]
    InternalServerError,
    #[error("501 Not Implemented")]
    NotImplemented,
    #[error("502 Bad Gateway")]
    BadGateway,
    #[error("503 Service Unavailable")]
    ServiceUnavailable,
    #[error("504 Gateway Timeout")]
    GatewayTimeout,

    /// Invalid UTF-8 in request
    #[error("Invalid UTF8")]
    BadUtf8,

    /// Return an empty response with the given status code
    #[error("{0}")]
    Status(StatusCode),
    /// Return the specified response with no transformation (except middleware)
    #[error("RAW {}", .0.status)]
    Response(Response),
    /// Returns a 500 Internal Service Error and logs the anyhow::Error to log::error (by default)
    #[error("{0:#}")]
    Internal(anyhow::Error),

    #[cfg(feature = "grpc")]
    #[error("{0:?}")]
    Grpc(crate::grpc::Status),
    #[cfg(feature = "grpc")]
    #[error("{0:?}")]
    GrpcMessage(crate::grpc::Status, String),

    // if returned from a middleware, it's skipped with no further action. used to return from extractors as an early filter.
    // there will be a panic if used elsewhere
    #[error("unreachable")]
    SkipMiddleware,
}

impl Default for Error {
    fn default() -> Self {
        Error::NotAnError
    }
}

impl Error {
    pub fn into_response(self) -> Response {
        match self {
            Error::NotAnError => unreachable!(),
            Error::BadRequest => StatusCode::BadRequest.into_response().unwrap(),
            Error::Unauthorized => StatusCode::Unauthorized.into_response().unwrap(),
            Error::Forbidden => StatusCode::Forbidden.into_response().unwrap(),
            Error::NotFound => StatusCode::NotFound.into_response().unwrap(),
            Error::MethodNotAllowed => StatusCode::MethodNotAllowed.into_response().unwrap(),
            Error::NotAcceptable => StatusCode::NotAcceptable.into_response().unwrap(),
            Error::RequestTimeout => StatusCode::RequestTimeout.into_response().unwrap(),
            Error::Conflict => StatusCode::Conflict.into_response().unwrap(),
            Error::Gone => StatusCode::Gone.into_response().unwrap(),
            Error::PreconditionFailed => StatusCode::PreconditionFailed.into_response().unwrap(),
            Error::PayloadTooLarge => StatusCode::PayloadTooLarge.into_response().unwrap(),
            Error::UriTooLong => StatusCode::UriTooLong.into_response().unwrap(),
            Error::UnsupportedMediaType => {
                StatusCode::UnsupportedMediaType.into_response().unwrap()
            }
            Error::RangeNotSatisfiable => StatusCode::RangeNotSatisfiable.into_response().unwrap(),
            Error::ExpectationFailed => StatusCode::ExpectationFailed.into_response().unwrap(),
            Error::UnprocessableEntity => StatusCode::UnprocessableEntity.into_response().unwrap(),
            Error::TooManyRequests => StatusCode::TooManyRequests.into_response().unwrap(),
            Error::UnavailableForLegalReasons => StatusCode::UnavailableForLegalReasons
                .into_response()
                .unwrap(),

            Error::InternalServerError => StatusCode::InternalServerError.into_response().unwrap(),
            Error::NotImplemented => StatusCode::NotImplemented.into_response().unwrap(),
            Error::BadGateway => StatusCode::BadGateway.into_response().unwrap(),
            Error::ServiceUnavailable => StatusCode::ServiceUnavailable.into_response().unwrap(),
            Error::GatewayTimeout => StatusCode::GatewayTimeout.into_response().unwrap(),

            Error::Redirect(mode, uri) => (mode, uri).into_response().unwrap(),
            Error::RedirectUrl(mode, uri) => (mode, uri).into_response().unwrap(),
            Error::BadUtf8 => (StatusCode::UnprocessableEntity, "invalid UTF-8 in request")
                .into_response()
                .unwrap(),
            Error::Status(s) => s.into_response().unwrap(),
            Error::Response(r) => r,
            Error::Internal(_) => StatusCode::InternalServerError.into_response().unwrap(),
            #[cfg(feature = "grpc")]
            Error::Grpc(status) => status.into_response().unwrap(),
            #[cfg(feature = "grpc")]
            Error::GrpcMessage(status, message) => (status, crate::grpc::StatusMessage(message))
                .into_response()
                .unwrap(),
            Error::SkipMiddleware => unreachable!(),
        }
    }
}

impl Error {
    pub fn redirect(mode: RedirectMode, uri: impl Into<Uri>) -> Self {
        Self::Redirect(mode, uri.into())
    }

    pub fn redirect_url(mode: RedirectMode, url: impl Into<Url>) -> Self {
        Self::RedirectUrl(mode, url.into())
    }

    pub fn moved_permanently(uri: impl Into<Uri>) -> Self {
        Self::redirect(RedirectMode::MovedPermanently, uri)
    }

    pub fn found(uri: impl Into<Uri>) -> Self {
        Self::redirect(RedirectMode::Found, uri)
    }

    pub fn see_other(uri: impl Into<Uri>) -> Self {
        Self::redirect(RedirectMode::SeeOther, uri)
    }

    pub fn temporary_redirect(uri: impl Into<Uri>) -> Self {
        Self::redirect(RedirectMode::TemporaryRedirect, uri)
    }

    pub fn permanent_redirect(uri: impl Into<Uri>) -> Self {
        Self::redirect(RedirectMode::PermanentRedirect, uri)
    }

    pub fn bad_request(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::BadRequest))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn unauthorized(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::Unauthorized))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn forbidden(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::Forbidden))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn not_found(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::NotFound))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn method_not_allowed(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::MethodNotAllowed))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn not_acceptable(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::NotAcceptable))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn request_timeout(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::RequestTimeout))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn conflict(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::Conflict))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn gone(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::Gone))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn precondition_failed(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::PreconditionFailed))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn payload_too_large(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::PayloadTooLarge))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn uri_too_long(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::UriTooLong))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn unsupported_media_type(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::UnsupportedMediaType))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn range_not_satisfiable(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::RangeNotSatisfiable))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn expectation_failed(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::ExpectationFailed))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn unprocessable_entity(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::UnprocessableEntity))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn too_many_requests(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::TooManyRequests))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn unavailable_for_legal_reasons(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::UnavailableForLegalReasons))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn internal_server_error(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::InternalServerError))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn not_implemented(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::NotImplemented))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn bad_gateway(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::BadGateway))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn service_unavailable(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::ServiceUnavailable))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn gateway_timeout(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(|x| x.with_status(StatusCode::GatewayTimeout))
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn response(body: impl IntoResponse) -> Self {
        body.into_response()
            .map(Error::Response)
            .unwrap_or_else(|x| x)
    }

    pub fn internal(error: impl Into<anyhow::Error>) -> Self {
        Self::Internal(error.into())
    }
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Self::Internal(value)
    }
}

impl From<Response> for Error {
    fn from(value: Response) -> Self {
        Self::Response(value)
    }
}

impl From<StatusCode> for Error {
    fn from(value: StatusCode) -> Self {
        Self::Status(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Internal(value.into())
    }
}
