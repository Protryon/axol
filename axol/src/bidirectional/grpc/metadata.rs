use crate::{Error, FromRequestParts, IntoResponseParts, Result};
use anyhow::anyhow;
use axol_http::{header::HeaderMap, request::RequestPartsRef, response::ResponsePartsRef};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum MetadataValue {
    Ascii(String),
    Binary(Vec<u8>),
}

#[derive(Clone, Debug, Default)]
pub struct Metadata(pub HashMap<String, Vec<MetadataValue>>);

#[async_trait::async_trait]
impl<'a> FromRequestParts<'a> for Metadata {
    async fn from_request_parts(request: RequestPartsRef<'a>) -> Result<Self> {
        let mut out = HashMap::new();
        for (name, value) in request.headers.iter() {
            if name.starts_with("grpc-")
                || name == "te"
                || name == "content-type"
                || name == "user-agent"
            {
                continue;
            }
            if !out.contains_key(name) {
                out.insert(name.to_string(), vec![]);
            }
            let dest = out.get_mut(name).unwrap();
            for value in value.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()) {
                if name.ends_with("-bin") {
                    //TODO: does this error out if there is a pad? GRPC spec says it must not.
                    dest.push(MetadataValue::Binary(
                        STANDARD_NO_PAD.decode(value).map_err(|_| {
                            Error::bad_request("malformed base64 in binary metadata")
                        })?,
                    ));
                } else {
                    dest.push(MetadataValue::Ascii(value.to_string()))
                }
            }
        }
        Ok(Self(out))
    }
}

impl Metadata {
    pub fn append_to(self, headers: &mut HeaderMap) -> Result<()> {
        for (name, values) in self.0.into_iter() {
            let mut is_ascii = false;
            let mut is_binary = false;
            let values = values
                .into_iter()
                .map(|x| {
                    Ok(match x {
                        MetadataValue::Ascii(x) => {
                            if is_binary {
                                return Err(Error::internal(anyhow!(
                                    "cannot mix binary and ascii metadata"
                                )))?;
                            }
                            is_ascii = true;
                            x
                        }
                        MetadataValue::Binary(x) => {
                            if is_ascii {
                                return Err(Error::internal(anyhow!(
                                    "cannot mix binary and ascii metadata"
                                )))?;
                            }
                            is_binary = true;
                            STANDARD_NO_PAD.encode(&x)
                        }
                    })
                })
                .collect::<Result<Vec<String>>>()?;
            headers.append(name, values.join(","));
        }
        Ok(())
    }
}

impl IntoResponseParts for Metadata {
    fn into_response_parts(self, response: &mut ResponsePartsRef<'_>) -> Result<()> {
        self.append_to(&mut response.headers)
    }
}
