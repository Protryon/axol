use axol_http::{request::RequestPartsRef, response::Response, Extensions};

use crate::{Error, ErrorHook, Result};

pub struct DefaultErrorHook;

#[async_trait::async_trait]
impl ErrorHook for DefaultErrorHook {
    async fn handle_error<'a>(
        &self,
        _request: RequestPartsRef<'a>,
        _: &mut Extensions,
        error: &mut Error,
    ) -> Result<Option<Response>> {
        //TODO: log header
        match &error {
            Error::Internal(e) => {
                log::error!("internal error: {e:#}");
            }
            e => {
                log::debug!("returning error response: {e}");
            }
        }
        Ok(Some(std::mem::take(error).into_response()))
    }
}
