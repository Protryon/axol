use std::{convert::Infallible, fmt};

use strum::EnumProperty;
use thiserror::Error;

/// An error from creating a status code from a u16.
#[derive(Error, Debug)]
pub enum StatusCodeError {
    #[error("infallible")]
    Infallible(#[from] Infallible),
    #[error("status code '{0}' not between 100 and 999 inclusive")]
    OutOfRange(u16),
}

/// An HTTP status code (`status-code` in RFC 7230 et al.).
///
/// Constants are provided for known status codes, including those in the IANA
/// [HTTP Status Code Registry](
/// https://www.iana.org/assignments/http-status-codes/http-status-codes.xhtml).
///
/// Status code values in the range 100-999 (inclusive) are supported by this
/// type. Values in the range 100-599 are semantically classified by the most
/// significant digit. See [`StatusCode::is_success`], etc. Values above 599
/// are unclassified but allowed for legacy compatibility, though their use is
/// discouraged. Applications may interpret such values as protocol errors.
///
/// # Examples
///
/// ```
/// use axol_http::StatusCode;
///
/// assert_eq!(StatusCode::from_u16(200).unwrap(), StatusCode::Ok);
/// assert_eq!(StatusCode::NotFound.as_u16(), 404);
/// assert!(StatusCode::Ok.is_success());
/// ```
#[repr(u16)]
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    strum::EnumProperty,
    strum::FromRepr,
)]
pub enum StatusCode {
    /// 100 Continue
    /// [[RFC7231, Section 6.2.1](https://tools.ietf.org/html/rfc7231#section-6.2.1)]
    #[strum(props(canonical_reason = "Continue"))]
    Continue = 100,
    /// 101 Switching Protocols
    /// [[RFC7231, Section 6.2.2](https://tools.ietf.org/html/rfc7231#section-6.2.2)]
    #[strum(props(canonical_reason = "Switching Protocols"))]
    SwitchingProtocols = 101,
    /// 102 Processing
    /// [[RFC2518](https://tools.ietf.org/html/rfc2518)]
    #[strum(props(canonical_reason = "Processing"))]
    Processing = 102,
    /// 200 OK
    /// [[RFC7231, Section 6.3.1](https://tools.ietf.org/html/rfc7231#section-6.3.1)]
    #[strum(props(canonical_reason = "OK"))]
    #[default]
    Ok = 200,
    /// 201 Created
    /// [[RFC7231, Section 6.3.2](https://tools.ietf.org/html/rfc7231#section-6.3.2)]
    #[strum(props(canonical_reason = "Created"))]
    Created = 201,
    /// 202 Accepted
    /// [[RFC7231, Section 6.3.3](https://tools.ietf.org/html/rfc7231#section-6.3.3)]
    #[strum(props(canonical_reason = "Accepted"))]
    Accepted = 202,
    /// 203 Non-Authoritative Information
    /// [[RFC7231, Section 6.3.4](https://tools.ietf.org/html/rfc7231#section-6.3.4)]
    #[strum(props(canonical_reason = "Non Authoritative Information"))]
    NonAuthoritativeInformation = 203,
    /// 204 No Content
    /// [[RFC7231, Section 6.3.5](https://tools.ietf.org/html/rfc7231#section-6.3.5)]
    #[strum(props(canonical_reason = "No Content"))]
    NoContent = 204,
    /// 205 Reset Content
    /// [[RFC7231, Section 6.3.6](https://tools.ietf.org/html/rfc7231#section-6.3.6)]
    #[strum(props(canonical_reason = "Reset Content"))]
    ResetContent = 205,
    /// 206 Partial Content
    /// [[RFC7233, Section 4.1](https://tools.ietf.org/html/rfc7233#section-4.1)]
    #[strum(props(canonical_reason = "Partial Content"))]
    PartialContent = 206,
    /// 207 Multi-Status
    /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
    #[strum(props(canonical_reason = "Multi-Status"))]
    MultiStatus = 207,
    /// 208 Already Reported
    /// [[RFC5842](https://tools.ietf.org/html/rfc5842)]
    #[strum(props(canonical_reason = "Already Reported"))]
    AlreadyReported = 208,

    /// 226 IM Used
    /// [[RFC3229](https://tools.ietf.org/html/rfc3229)]
    #[strum(props(canonical_reason = "IM Used"))]
    ImUsed = 226,

    /// 300 Multiple Choices
    /// [[RFC7231, Section 6.4.1](https://tools.ietf.org/html/rfc7231#section-6.4.1)]
    #[strum(props(canonical_reason = "Multiple Choices"))]
    MultipleChoices = 300,
    /// 301 Moved Permanently
    /// [[RFC7231, Section 6.4.2](https://tools.ietf.org/html/rfc7231#section-6.4.2)]
    #[strum(props(canonical_reason = "Moved Permanently"))]
    MovedPermanently = 301,
    /// 302 Found
    /// [[RFC7231, Section 6.4.3](https://tools.ietf.org/html/rfc7231#section-6.4.3)]
    #[strum(props(canonical_reason = "Found"))]
    Found = 302,
    /// 303 See Other
    /// [[RFC7231, Section 6.4.4](https://tools.ietf.org/html/rfc7231#section-6.4.4)]
    #[strum(props(canonical_reason = "See Other"))]
    SeeOther = 303,
    /// 304 Not Modified
    /// [[RFC7232, Section 4.1](https://tools.ietf.org/html/rfc7232#section-4.1)]
    #[strum(props(canonical_reason = "Not Modified"))]
    NotModified = 304,
    /// 305 Use Proxy
    /// [[RFC7231, Section 6.4.5](https://tools.ietf.org/html/rfc7231#section-6.4.5)]
    #[strum(props(canonical_reason = "Use Proxy"))]
    UseProxy = 305,
    /// 307 Temporary Redirect
    /// [[RFC7231, Section 6.4.7](https://tools.ietf.org/html/rfc7231#section-6.4.7)]
    #[strum(props(canonical_reason = "Temporary Redirect"))]
    TemporaryRedirect = 307,
    /// 308 Permanent Redirect
    /// [[RFC7238](https://tools.ietf.org/html/rfc7238)]
    #[strum(props(canonical_reason = "Permanent Redirect"))]
    PermanentRedirect = 308,

    /// 400 Bad Request
    /// [[RFC7231, Section 6.5.1](https://tools.ietf.org/html/rfc7231#section-6.5.1)]
    #[strum(props(canonical_reason = "Bad Request"))]
    BadRequest = 400,
    /// 401 Unauthorized
    /// [[RFC7235, Section 3.1](https://tools.ietf.org/html/rfc7235#section-3.1)]
    #[strum(props(canonical_reason = "Unauthorized"))]
    Unauthorized = 401,
    /// 402 Payment Required
    /// [[RFC7231, Section 6.5.2](https://tools.ietf.org/html/rfc7231#section-6.5.2)]
    #[strum(props(canonical_reason = "Payment Required"))]
    PaymentRequired = 402,
    /// 403 Forbidden
    /// [[RFC7231, Section 6.5.3](https://tools.ietf.org/html/rfc7231#section-6.5.3)]
    #[strum(props(canonical_reason = "Forbidden"))]
    Forbidden = 403,
    /// 404 Not Found
    /// [[RFC7231, Section 6.5.4](https://tools.ietf.org/html/rfc7231#section-6.5.4)]
    #[strum(props(canonical_reason = "Not Found"))]
    NotFound = 404,
    /// 405 Method Not Allowed
    /// [[RFC7231, Section 6.5.5](https://tools.ietf.org/html/rfc7231#section-6.5.5)]
    #[strum(props(canonical_reason = "Method Not Allowed"))]
    MethodNotAllowed = 405,
    /// 406 Not Acceptable
    /// [[RFC7231, Section 6.5.6](https://tools.ietf.org/html/rfc7231#section-6.5.6)]
    #[strum(props(canonical_reason = "Not Acceptable"))]
    NotAcceptable = 406,
    /// 407 Proxy Authentication Required
    /// [[RFC7235, Section 3.2](https://tools.ietf.org/html/rfc7235#section-3.2)]
    #[strum(props(canonical_reason = "Proxy Authentication Required"))]
    ProxyAuthenticationRequired = 407,
    /// 408 Request Timeout
    /// [[RFC7231, Section 6.5.7](https://tools.ietf.org/html/rfc7231#section-6.5.7)]
    #[strum(props(canonical_reason = "Request Timeout"))]
    RequestTimeout = 408,
    /// 409 Conflict
    /// [[RFC7231, Section 6.5.8](https://tools.ietf.org/html/rfc7231#section-6.5.8)]
    #[strum(props(canonical_reason = "Conflict"))]
    Conflict = 409,
    /// 410 Gone
    /// [[RFC7231, Section 6.5.9](https://tools.ietf.org/html/rfc7231#section-6.5.9)]
    #[strum(props(canonical_reason = "Gone"))]
    Gone = 410,
    /// 411 Length Required
    /// [[RFC7231, Section 6.5.10](https://tools.ietf.org/html/rfc7231#section-6.5.10)]
    #[strum(props(canonical_reason = "Length Required"))]
    LengthRequired = 411,
    /// 412 Precondition Failed
    /// [[RFC7232, Section 4.2](https://tools.ietf.org/html/rfc7232#section-4.2)]
    #[strum(props(canonical_reason = "Precondition Failed"))]
    PreconditionFailed = 412,
    /// 413 Payload Too Large
    /// [[RFC7231, Section 6.5.11](https://tools.ietf.org/html/rfc7231#section-6.5.11)]
    #[strum(props(canonical_reason = "Payload Too Large"))]
    PayloadTooLarge = 413,
    /// 414 URI Too Long
    /// [[RFC7231, Section 6.5.12](https://tools.ietf.org/html/rfc7231#section-6.5.12)]
    #[strum(props(canonical_reason = "URI Too Long"))]
    UriTooLong = 414,
    /// 415 Unsupported Media Type
    /// [[RFC7231, Section 6.5.13](https://tools.ietf.org/html/rfc7231#section-6.5.13)]
    #[strum(props(canonical_reason = "Unsupported Media Type"))]
    UnsupportedMediaType = 415,
    /// 416 Range Not Satisfiable
    /// [[RFC7233, Section 4.4](https://tools.ietf.org/html/rfc7233#section-4.4)]
    #[strum(props(canonical_reason = "Range Not Satisfiable"))]
    RangeNotSatisfiable = 416,
    /// 417 Expectation Failed
    /// [[RFC7231, Section 6.5.14](https://tools.ietf.org/html/rfc7231#section-6.5.14)]
    #[strum(props(canonical_reason = "Expectation Failed"))]
    ExpectationFailed = 417,
    /// 418 I'm a teapot
    /// [curiously not registered by IANA but [RFC2324](https://tools.ietf.org/html/rfc2324)]
    #[strum(props(canonical_reason = "I'm a teapot"))]
    ImATeapot = 418,

    /// 421 Misdirected Request
    /// [RFC7540, Section 9.1.2](http://tools.ietf.org/html/rfc7540#section-9.1.2)
    #[strum(props(canonical_reason = "Misdirected Request"))]
    MisdirectedRequest = 421,
    /// 422 Unprocessable Entity
    /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
    #[strum(props(canonical_reason = "Unprocessable Entity"))]
    UnprocessableEntity = 422,
    /// 423 Locked
    /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
    #[strum(props(canonical_reason = "Locked"))]
    Locked = 423,
    /// 424 Failed Dependency
    /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
    #[strum(props(canonical_reason = "Failed Dependency"))]
    FailedDependency = 424,

    /// 426 Upgrade Required
    /// [[RFC7231, Section 6.5.15](https://tools.ietf.org/html/rfc7231#section-6.5.15)]
    #[strum(props(canonical_reason = "Upgrade Required"))]
    UpgradeRequired = 426,

    /// 428 Precondition Required
    /// [[RFC6585](https://tools.ietf.org/html/rfc6585)]
    #[strum(props(canonical_reason = "Precondition Required"))]
    PreconditionRequired = 428,
    /// 429 Too Many Requests
    /// [[RFC6585](https://tools.ietf.org/html/rfc6585)]
    #[strum(props(canonical_reason = "Too Many Requests"))]
    TooManyRequests = 429,

    /// 431 Request Header Fields Too Large
    /// [[RFC6585](https://tools.ietf.org/html/rfc6585)]
    #[strum(props(canonical_reason = "Request Header Fields Too Large"))]
    RequestHeaderFieldsTooLarge = 431,

    /// 451 Unavailable For Legal Reasons
    /// [[RFC7725](http://tools.ietf.org/html/rfc7725)]
    #[strum(props(canonical_reason = "Unavailable For Legal Reasons"))]
    UnavailableForLegalReasons = 451,

    /// 500 Internal Server Error
    /// [[RFC7231, Section 6.6.1](https://tools.ietf.org/html/rfc7231#section-6.6.1)]
    #[strum(props(canonical_reason = "Internal Server Error"))]
    InternalServerError = 500,
    /// 501 Not Implemented
    /// [[RFC7231, Section 6.6.2](https://tools.ietf.org/html/rfc7231#section-6.6.2)]
    #[strum(props(canonical_reason = "Not Implemented"))]
    NotImplemented = 501,
    /// 502 Bad Gateway
    /// [[RFC7231, Section 6.6.3](https://tools.ietf.org/html/rfc7231#section-6.6.3)]
    #[strum(props(canonical_reason = "Bad Gateway"))]
    BadGateway = 502,
    /// 503 Service Unavailable
    /// [[RFC7231, Section 6.6.4](https://tools.ietf.org/html/rfc7231#section-6.6.4)]
    #[strum(props(canonical_reason = "Service Unavailable"))]
    ServiceUnavailable = 503,
    /// 504 Gateway Timeout
    /// [[RFC7231, Section 6.6.5](https://tools.ietf.org/html/rfc7231#section-6.6.5)]
    #[strum(props(canonical_reason = "Gateway Timeout"))]
    GatewayTimeout = 504,
    /// 505 HTTP Version Not Supported
    /// [[RFC7231, Section 6.6.6](https://tools.ietf.org/html/rfc7231#section-6.6.6)]
    #[strum(props(canonical_reason = "HTTP Version Not Supported"))]
    HttpVersionNotSupported = 505,
    /// 506 Variant Also Negotiates
    /// [[RFC2295](https://tools.ietf.org/html/rfc2295)]
    #[strum(props(canonical_reason = "Variant Also Negotiates"))]
    VariantAlsoNegotiates = 506,
    /// 507 Insufficient Storage
    /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
    #[strum(props(canonical_reason = "Insufficient Storage"))]
    InsufficientStorage = 507,
    /// 508 Loop Detected
    /// [[RFC5842](https://tools.ietf.org/html/rfc5842)]
    #[strum(props(canonical_reason = "Loop Detected"))]
    LoopDetected = 508,

    /// 510 Not Extended
    /// [[RFC2774](https://tools.ietf.org/html/rfc2774)]
    #[strum(props(canonical_reason = "Not Extended"))]
    NotExtended = 510,
    /// 511 Network Authentication Required
    /// [[RFC6585](https://tools.ietf.org/html/rfc6585)]
    #[strum(props(canonical_reason = "Network Authentication Required"))]
    NetworkAuthenticationRequired = 511,

    /// An unknown (invalid) status code. It is a logic error that will do unexpected things if this is initialized to a valid status code.
    Other(u16) = u16::MAX,
}

impl Into<http::StatusCode> for StatusCode {
    fn into(self) -> http::StatusCode {
        http::StatusCode::from_u16(self.as_u16())
            .expect("out of range status code where not expected")
    }
}

impl From<http::StatusCode> for StatusCode {
    fn from(value: http::StatusCode) -> Self {
        Self::from_u16(value.as_u16()).expect("out of range status code where not expected")
    }
}

impl TryFrom<u16> for StatusCode {
    type Error = StatusCodeError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::from_u16(value)
    }
}

impl Into<u16> for StatusCode {
    fn into(self) -> u16 {
        self.as_u16()
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.as_str(), self.canonical_reason())
    }
}

impl StatusCode {
    /// Converts a u16 to a status code.
    ///
    /// The function validates the correctness of the supplied u16. It must be
    /// greater or equal to 100 and less than 1000.
    ///
    /// # Example
    ///
    /// ```
    /// use axol_http::StatusCode;
    ///
    /// let ok = StatusCode::from_u16(200).unwrap();
    /// assert!(matches!(ok, StatusCode::Ok);
    ///
    /// let err = StatusCode::from_u16(99);
    /// assert!(err.is_err());
    /// ```
    pub fn from_u16(value: u16) -> Result<Self, StatusCodeError> {
        if value >= 1000 || value < 100 {
            return Err(StatusCodeError::OutOfRange(value));
        }
        Ok(match Self::from_repr(value) {
            Some(StatusCode::Other(_)) => StatusCode::Other(u16::MAX),
            Some(x) => x,
            None => StatusCode::Other(value),
        })
    }

    /// Returns the `u16` corresponding to this `StatusCode`.
    ///
    /// # Note
    ///
    /// This is the same as the `From<StatusCode>` implementation, but
    /// included as an inherent method because that implementation doesn't
    /// appear in rustdocs, as well as a way to force the type instead of
    /// relying on inference.
    ///
    /// # Example
    ///
    /// ```
    /// let status = axol_http::StatusCode::Ok;
    /// assert_eq!(status.as_u16(), 200);
    /// ```
    pub fn as_u16(&self) -> u16 {
        match self {
            Self::Other(x) => *x,
            x => x.discriminant(),
        }
    }

    /// Returns a &str representation of the `StatusCode`
    ///
    /// The return value only includes a numerical representation of the
    /// status code. The canonical reason is not included.
    ///
    /// # Example
    ///
    /// ```
    /// let status = axol_http::StatusCode::Ok;
    /// assert_eq!(status.as_str(), "200");
    /// ```
    pub fn as_str(&self) -> &'static str {
        let inner: http::StatusCode = (*self).into();
        // SAFETY: http::StatusCode::as_str is returning a reference to a const str.
        // to maintain safety, we must pin http crate version
        unsafe { std::mem::transmute(inner.as_str()) }
    }

    /// Get the standardised `reason-phrase` for this status code.
    ///
    /// This is mostly here for servers writing responses, but could potentially have application
    /// at other times.
    ///
    /// The reason phrase is defined as being exclusively for human readers. You should avoid
    /// deriving any meaning from it at all costs.
    ///
    /// Bear in mind also that in HTTP/2.0 and HTTP/3.0 the reason phrase is abolished from
    /// transmission, and so this canonical reason phrase really is the only reason phrase you’ll
    /// find.
    ///
    /// This is empty for `Other` or invalid status codes.
    ///
    /// # Example
    ///
    /// ```
    /// let status = axol_http::StatusCode::OK;
    /// assert_eq!(status.canonical_reason(), "OK");
    /// ```
    pub fn canonical_reason(&self) -> &'static str {
        self.get_str("canonical_reason").unwrap_or_default()
    }

    /// Check if status is within 100-199.
    pub fn is_informational(&self) -> bool {
        200 > self.discriminant() && self.discriminant() >= 100
    }

    /// Check if status is within 200-299.
    pub fn is_success(&self) -> bool {
        300 > self.discriminant() && self.discriminant() >= 200
    }

    /// Check if status is within 300-399.
    pub fn is_redirection(&self) -> bool {
        400 > self.discriminant() && self.discriminant() >= 300
    }

    /// Check if status is within 400-499.
    pub fn is_client_error(&self) -> bool {
        500 > self.discriminant() && self.discriminant() >= 400
    }

    /// Check if status is within 500-599.
    pub fn is_server_error(&self) -> bool {
        600 > self.discriminant() && self.discriminant() >= 500
    }

    // https://doc.rust-lang.org/reference/items/enumerations.html#pointer-casting
    fn discriminant(&self) -> u16 {
        unsafe { *(self as *const Self as *const u16) }
    }
}
