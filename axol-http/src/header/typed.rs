use headers::HeaderValue;

use crate::typed_headers::Header as HttpHeader;

use super::HeaderMap;

pub trait TypedHeader {
    fn name() -> &'static str;

    fn encode(&self, map: &mut HeaderMap);

    fn encode_to_string(&self) -> Vec<String> {
        let mut out = HeaderMap::default();
        self.encode(&mut out);
        out.into_iter()
            .map(|(_, value)| value.into_owned())
            .collect()
    }

    fn decode(from: &str) -> Result<Self, crate::typed_headers::Error>
    where
        Self: Sized;
}

struct HeaderValueProxy<'a> {
    name: &'static str,
    target: &'a mut HeaderMap,
}

impl<'a> Extend<http::HeaderValue> for HeaderValueProxy<'a> {
    fn extend<T: IntoIterator<Item = http::HeaderValue>>(&mut self, iter: T) {
        //TODO: unsafe transmute to get raw bytes out of HeaderValue?
        self.target.extend(iter.into_iter().map(|x| {
            (
                self.name,
                x.to_str()
                    .expect("typed header had non-utf8 value")
                    .to_string(),
            )
        }))
    }
}

impl<H: HttpHeader> TypedHeader for H {
    fn name() -> &'static str {
        <Self as HttpHeader>::name().as_str()
    }

    fn encode(&self, map: &mut HeaderMap) {
        <Self as HttpHeader>::encode(
            self,
            &mut HeaderValueProxy {
                name: <Self as TypedHeader>::name(),
                target: map,
            },
        );
    }

    fn decode(from: &str) -> Result<Self, crate::typed_headers::Error> {
        let header = HeaderValue::from_bytes(from.as_bytes()).expect("invalid header value");
        <Self as HttpHeader>::decode(&mut std::iter::once(&header))
    }
}
