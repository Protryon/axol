use std::time::Instant;

use axol_http::{
    request::{Request, RequestPartsRef},
    response::Response,
};
use log::Level;

use crate::{ConnectInfo, LateResponseHook, Plugin, RequestHook, Result, Router};

#[derive(Clone)]
pub struct Logger {
    pub default_log_level: Level,
}

impl Default for Logger {
    fn default() -> Self {
        Logger {
            default_log_level: Level::Info,
        }
    }
}

struct LogInfo {
    start: Instant,
}

#[async_trait::async_trait]
impl RequestHook for Logger {
    async fn handle_request(&self, request: &mut Request) -> Result<Option<Response>> {
        println!("logger req");
        request.extensions.insert(LogInfo {
            start: Instant::now(),
        });
        Ok(None)
    }
}

#[async_trait::async_trait]
impl LateResponseHook for Logger {
    async fn handle_response<'a>(&self, request: RequestPartsRef<'a>, response: &mut Response) {
        let Some(log_info) = request.extensions.get::<LogInfo>() else {
            // we got inserted part-way through?
            return;
        };
        let elapsed = log_info.start.elapsed();
        let Some(remote) = request.extensions.get::<ConnectInfo>() else {
            // not a remote connection
            return;
        };
        let mut level = self.default_log_level;
        if let Some(new_level) = request.extensions.get::<Level>() {
            level = *new_level;
        }
        if let Some(new_level) = response.extensions.get::<Level>() {
            level = *new_level;
        }
        //TODO: configurable log format
        log::log!(
            level,
            "[{}] {} {} -> {} [{:.02} ms]",
            remote.0,
            request.method,
            request.uri,
            response.status,
            elapsed.as_secs_f64() * 1000.0
        );
    }
}

impl Plugin for Logger {
    fn apply(&self, router: Router, path: &str) -> Router {
        router
            .request_hook(path, self.clone())
            .late_response_hook(path, self.clone())
    }
}
