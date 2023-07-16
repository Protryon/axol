# axol-http

This project is an opinionated wrapper of the `http` crate.

## Distinctions

This crate:
* Does not allow non-UTF8 header names/values
* Strictly only allows standard HTTP methods (no custom methods)
* Uses an enum for `StatusCode`s, but does not strictly enforce they are standard ones.