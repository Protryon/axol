use crate::Result;
use axol_http::{header::HeaderMap, response::ResponsePartsRef, Extensions, StatusCode};

pub trait IntoResponseParts {
    fn into_response_parts(self, response: &mut ResponsePartsRef<'_>) -> Result<()>;
}

impl<T: IntoResponseParts> IntoResponseParts for Option<T> {
    fn into_response_parts(self, response: &mut ResponsePartsRef<'_>) -> Result<()> {
        match self {
            Some(x) => x.into_response_parts(response),
            None => Ok(()),
        }
    }
}

impl IntoResponseParts for HeaderMap {
    fn into_response_parts(self, response: &mut ResponsePartsRef<'_>) -> Result<()> {
        response.headers.extend(self.into_iter());
        Ok(())
    }
}

impl IntoResponseParts for Extensions {
    fn into_response_parts(self, response: &mut ResponsePartsRef<'_>) -> Result<()> {
        response.extensions.extend(self);
        Ok(())
    }
}

pub struct AppendHeader<K: AsRef<str>, V: Into<String>>(pub K, pub V);

impl IntoResponseParts for StatusCode {
    fn into_response_parts(self, response: &mut ResponsePartsRef<'_>) -> Result<()> {
        *response.status = self;
        Ok(())
    }
}

impl<K: AsRef<str>, V: Into<String>> IntoResponseParts for AppendHeader<K, V> {
    fn into_response_parts(self, response: &mut ResponsePartsRef<'_>) -> Result<()> {
        response.headers.append(self.0, self.1);
        Ok(())
    }
}

impl<K: AsRef<str>, V: Into<String>, const N: usize> IntoResponseParts for [(K, V); N] {
    fn into_response_parts(self, response: &mut ResponsePartsRef<'_>) -> Result<()> {
        for (name, value) in self {
            response.headers.append(name, value);
        }
        Ok(())
    }
}

macro_rules! impl_into_response_parts {
    ( $($ty:ident),* $(,)? ) => {
        #[allow(non_snake_case)]
        #[allow(unused_variables)]
        impl<$($ty,)*> IntoResponseParts for ($($ty,)*)
        where
            $( $ty: IntoResponseParts, )*
        {
            fn into_response_parts(self, response: &mut ResponsePartsRef<'_>) -> Result<()> {
                let ($($ty,)*) = self;

                $(
                    $ty.into_response_parts(response)?;
                )*

                Ok(())
            }
        }
    }
}

all_the_tuples_no_last_special_case!(impl_into_response_parts);
