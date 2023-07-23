#[macro_use]
mod macros;

mod request;
pub use request::*;

mod response;
pub use response::*;

mod bidirectional;
pub use bidirectional::*;

mod error;
pub use error::*;

mod router;
pub use router::*;

mod handler;
pub use handler::*;

mod server;
pub use server::*;

mod middleware;
pub use middleware::*;

pub use axol_http as http;

pub mod prelude {
    pub use crate::{
        bidirectional::{Form, Json, Typed},
        error::*,
        request::{FromRequest, FromRequestParts},
        request::{Path, Query},
        response::{IntoResponse, IntoResponseParts},
        router::Router,
    };
    pub use axol_http::{Body, Request, RequestPartsRef, Response, StatusCode};
}
