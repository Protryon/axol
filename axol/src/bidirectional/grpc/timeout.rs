use std::time::Duration;

use axol_http::typed_headers::{Error as HeaderError, Header, HeaderName, HeaderValue};

#[derive(Debug)]
pub struct GrpcTimeout(pub Duration);

static GRPC_TIMEOUT: HeaderName = HeaderName::from_static("grpc-timeout");

impl Header for GrpcTimeout {
    fn name() -> &'static HeaderName {
        &GRPC_TIMEOUT
    }

    fn decode<'i, I: Iterator<Item = &'i HeaderValue>>(values: &mut I) -> Result<Self, HeaderError>
    where
        Self: Sized,
    {
        let value = values.next().ok_or_else(|| HeaderError::invalid())?;
        let value = value.to_str().map_err(|_| HeaderError::invalid())?;
        if value.len() < 2 {
            return Err(HeaderError::invalid());
        }
        let suffix = value
            .chars()
            .rev()
            .next()
            .ok_or_else(|| HeaderError::invalid())?;
        let count: u64 = value[..value.len() - suffix.len_utf8()]
            .parse()
            .map_err(|_| HeaderError::invalid())?;
        if count > 9999999 {
            return Err(HeaderError::invalid());
        }
        let ns_per_unit: u64 = match suffix {
            'n' => 1,
            'u' => 1000,
            'm' => 1000000,
            'S' => 1000000000,
            'M' => 60000000000,
            'H' => 3600000000000,
            _ => return Err(HeaderError::invalid()),
        };
        let total_ns = count.saturating_mul(ns_per_unit);
        Ok(Self(Duration::from_nanos(total_ns)))
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        let ns = self.0.as_nanos().min(u64::MAX as u128) as u64;
        //TODO: we need to add rounding so we don't go over 8 char limit
        if ns % 3600000000000 == 0 {
            values.extend(std::iter::once(
                HeaderValue::from_str(&format!("{}H", ns / 3600000000000)).unwrap(),
            ));
        } else if ns % 60000000000 == 0 {
            values.extend(std::iter::once(
                HeaderValue::from_str(&format!("{}M", ns / 60000000000)).unwrap(),
            ));
        } else if ns % 1000000000 == 0 {
            values.extend(std::iter::once(
                HeaderValue::from_str(&format!("{}S", ns / 1000000000)).unwrap(),
            ));
        } else if ns % 1000000 == 0 {
            values.extend(std::iter::once(
                HeaderValue::from_str(&format!("{}m", ns / 1000000)).unwrap(),
            ));
        } else if ns % 1000 == 0 {
            values.extend(std::iter::once(
                HeaderValue::from_str(&format!("{}u", ns / 1000)).unwrap(),
            ));
        } else {
            values.extend(std::iter::once(
                HeaderValue::from_str(&format!("{}n", ns)).unwrap(),
            ));
        }
    }
}
