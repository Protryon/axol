use std::net::{IpAddr, SocketAddr};

use axol_http::{request::Request, response::Response};

use crate::{ConnectInfo, Error, RequestHook, Result};

pub struct RealIp(pub String);

#[async_trait::async_trait]
impl RequestHook for RealIp {
    async fn handle_request(&self, request: &mut Request) -> Result<Option<Response>> {
        let Some(value) = request.headers.get(&self.0) else {
            return Ok(None);
        };
        let connect_info = request.extensions.get::<ConnectInfo>();
        let Some(new_ip) = value.split_once(',').map(|x| x.0).unwrap_or(value).trim().parse::<IpAddr>().ok() else {
            return Err(Error::bad_request(format!("invalid '{}' header", self.0)));
        };
        request.extensions.insert(ConnectInfo(SocketAddr::new(
            new_ip,
            connect_info.map(|x| x.0.port()).unwrap_or_default(),
        )));
        Ok(None)
    }
}
