mod map;
pub use map::*;
mod names;
pub use names::*;
mod typed;
pub use typed::TypedHeader;

use std::borrow::Cow;

pub fn header_name(name: &str) -> Cow<'static, str> {
    match names::maybe_static_lowercase(name) {
        Some(x) => Cow::Borrowed(x),
        //TODO: return borrowed if already lowercase
        None => Cow::Owned(name.to_ascii_lowercase()),
    }
}
