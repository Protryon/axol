mod request_hook;
pub use request_hook::*;

mod response_hook;
pub use response_hook::*;

mod error_hook;
pub use error_hook::*;

mod multi;
pub use multi::*;

use crate::Router;

pub trait Plugin {
    fn apply(&self, router: Router, path: &str) -> Router;
}
