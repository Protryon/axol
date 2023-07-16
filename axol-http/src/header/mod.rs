mod map;
pub use map::*;
mod names;
pub use names::*;
mod typed;
pub use typed::TypedHeader;

use std::borrow::Cow;

fn header_name(name: &str) -> Cow<'static, str> {
    match names::maybe_static_lowercase(name) {
        Some(x) => Cow::Borrowed(x),
        None => Cow::Owned(name.to_ascii_lowercase()),
    }
}
