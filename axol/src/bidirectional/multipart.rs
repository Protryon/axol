//! Extractor that parses `multipart/form-data` requests commonly used with file uploads.
//!
//! See [`Multipart`] for more details.

use async_trait::async_trait;
use axol_http::body::BodyComponent;
use axol_http::header::HeaderMap;
use axol_http::mime::{Mime, BOUNDARY};
use axol_http::request::RequestPartsRef;
use axol_http::typed_headers::ContentType;
use axol_http::{Body, StatusCode};
use bytes::Bytes;
use futures_util::stream::Stream;
use std::{
    fmt,
    pin::Pin,
    task::{Context, Poll},
};
use tokio_stream::StreamExt;

use crate::{Error, FromRequest, IntoResponse, Result};

/// Extractor that parses `multipart/form-data` requests (commonly used with file uploads).
///
/// ⚠️ Since extracting multipart form data from the request requires consuming the body, the
/// `Multipart` extractor must be *last* if there are multiple extractors in a handler.
/// See ["the order of extractors"][order-of-extractors]
///
/// [order-of-extractors]: crate::extract#the-order-of-extractors
///
/// # Example
///
/// ```rust,no_run
/// use axum::{
///     extract::Multipart,
///     routing::post,
///     Router,
/// };
/// use futures_util::stream::StreamExt;
///
/// async fn upload(mut multipart: Multipart) {
///     while let Some(mut field) = multipart.next_field().await.unwrap() {
///         let name = field.name().unwrap().to_string();
///         let data = field.bytes().await.unwrap();
///
///         println!("Length of `{}` is {} bytes", name, data.len());
///     }
/// }
///
/// let app = Router::new().route("/upload", post(upload));
/// # async {
/// # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
/// # };
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "multipart")))]
#[derive(Debug)]
pub struct Multipart {
    inner: multer::Multipart<'static>,
}

#[async_trait]
impl<'a> FromRequest<'a> for Multipart {
    async fn from_request(request: RequestPartsRef<'a>, body: Body) -> Result<Self> {
        let boundary = parse_boundary(&request.headers).ok_or_else(|| {
            Error::bad_request("Invalid `boundary` for `multipart/form-data` request")
        })?;
        let stream = body.into_stream().filter_map(|x| {
            Some(match x {
                Err(e) => Err(e),
                Ok(BodyComponent::Trailers(_)) => return None,
                Ok(BodyComponent::Data(data)) => Ok(data),
            })
        });
        let multipart = multer::Multipart::new(stream, boundary);
        Ok(Self { inner: multipart })
    }
}

impl Multipart {
    /// Yields the next [`Field`] if available.
    pub async fn next_field(&mut self) -> Result<Option<Field<'_>>> {
        let field = self
            .inner
            .next_field()
            .await
            .map_err(MultipartError::from_multer)
            .map_err(MultipartError::into_error)?;

        if let Some(field) = field {
            Ok(Some(Field {
                inner: field,
                _multipart: self,
            }))
        } else {
            Ok(None)
        }
    }
}

/// A single field in a multipart stream.
#[derive(Debug)]
pub struct Field<'a> {
    inner: multer::Field<'static>,
    // multer requires there to only be one live `multer::Field` at any point. This enforces that
    // statically, which multer does not do, it returns an error instead.
    _multipart: &'a mut Multipart,
}

impl<'a> Stream for Field<'a> {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner)
            .poll_next(cx)
            .map_err(MultipartError::from_multer)
            .map_err(MultipartError::into_error)
    }
}

impl<'a> Field<'a> {
    /// The field name found in the
    /// [`Content-Disposition`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Disposition)
    /// header.
    pub fn name(&self) -> Option<&str> {
        self.inner.name()
    }

    /// The file name found in the
    /// [`Content-Disposition`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Disposition)
    /// header.
    pub fn file_name(&self) -> Option<&str> {
        self.inner.file_name()
    }

    /// Get the [content type](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Type) of the field.
    pub fn content_type(&self) -> Option<&str> {
        self.inner.content_type().map(|m| m.as_ref())
    }

    /// Get a map of headers as [`HeaderMap`].
    pub fn headers(&self) -> Result<HeaderMap> {
        self.inner
            .headers()
            .clone()
            .try_into()
            .map_err(|_| Error::BadUtf8)
    }

    /// Get the full data of the field as [`Bytes`].
    pub async fn bytes(self) -> Result<Bytes> {
        self.inner
            .bytes()
            .await
            .map_err(MultipartError::from_multer)
            .map_err(MultipartError::into_error)
    }

    /// Get the full field data as text.
    pub async fn text(self) -> Result<String> {
        self.inner
            .text()
            .await
            .map_err(MultipartError::from_multer)
            .map_err(MultipartError::into_error)
    }

    /// Stream a chunk of the field data.
    ///
    /// When the field data has been exhausted, this will return [`None`].
    ///
    /// Note this does the same thing as `Field`'s [`Stream`] implementation.
    ///
    /// # Example
    ///
    /// ```
    /// use axum::{
    ///    extract::Multipart,
    ///    routing::post,
    ///    response::IntoResponse,
    ///    http::StatusCode,
    ///    Router,
    /// };
    ///
    /// async fn upload(mut multipart: Multipart) -> Result<(), (StatusCode, String)> {
    ///     while let Some(mut field) = multipart
    ///         .next_field()
    ///         .await
    ///         .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?
    ///     {
    ///         while let Some(chunk) = field
    ///             .chunk()
    ///             .await
    ///             .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?
    ///         {
    ///             println!("received {} bytes", chunk.len());
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    ///
    /// let app = Router::new().route("/upload", post(upload));
    /// # let _: Router = app;
    /// ```
    pub async fn chunk(&mut self) -> Result<Option<Bytes>> {
        self.inner
            .chunk()
            .await
            .map_err(MultipartError::from_multer)
            .map_err(MultipartError::into_error)
    }
}

/// Errors associated with parsing `multipart/form-data` requests.
#[derive(Debug)]
struct MultipartError {
    source: multer::Error,
}

impl MultipartError {
    fn from_multer(multer: multer::Error) -> Self {
        Self { source: multer }
    }

    /// Get the response body text used for this rejection.
    pub fn body_text(&self) -> String {
        self.source.to_string()
    }

    /// Get the status code used for this rejection.
    pub fn status(&self) -> StatusCode {
        status_code_from_multer_error(&self.source)
    }
}

fn status_code_from_multer_error(err: &multer::Error) -> StatusCode {
    match err {
        multer::Error::UnknownField { .. }
        | multer::Error::IncompleteFieldData { .. }
        | multer::Error::IncompleteHeaders
        | multer::Error::ReadHeaderFailed(..)
        | multer::Error::DecodeHeaderName { .. }
        | multer::Error::DecodeContentType(..)
        | multer::Error::NoBoundary
        | multer::Error::DecodeHeaderValue { .. }
        | multer::Error::NoMultipart
        | multer::Error::IncompleteStream => StatusCode::BadRequest,
        multer::Error::FieldSizeExceeded { .. } | multer::Error::StreamSizeExceeded { .. } => {
            StatusCode::PayloadTooLarge
        }
        multer::Error::StreamReadFailed(err) => {
            if let Some(err) = err.downcast_ref::<multer::Error>() {
                return status_code_from_multer_error(err);
            }

            StatusCode::InternalServerError
        }
        _ => StatusCode::InternalServerError,
    }
}

impl fmt::Display for MultipartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error parsing `multipart/form-data` request")
    }
}

impl std::error::Error for MultipartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

impl MultipartError {
    fn into_error(self) -> Error {
        Error::Response((self.status(), self.body_text()).into_response().unwrap())
    }
}

fn parse_boundary(headers: &HeaderMap) -> Option<String> {
    let mime: Mime = headers.get_typed::<ContentType>()?.into();
    Some(mime.get_param(BOUNDARY)?.as_str().to_string())
}
