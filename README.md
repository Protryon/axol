# axol

`axol` is a high-level HTTP framework inspired by `axum`. It's an opinionated project -- it won't support every possible use case, but what it does support, it will support well.

## Motivation

The popular rust HTTP infrastructure (`hyper`, `axum`, `tonic`, etc) are all built to be powerful and universally usable tools. They succeed that that. I'm an engineer that has written a lot of production software using these projects and others like them, and I've found that a few design decisions and constraints imposed aren't needed in your average day-to-day work.

Some features like:
* Non-UTF8 header support (and the clunkiness that comes with it)
* Support for custom/non-standard HTTP methods
* `tower` middleware, that while very powerful, has a lot of boilerplate and clunkiness to write.
* Exposure of type parameters that just end up getting boxed, sometimes painfully. (`Body` in `axum` as a primary example)

So I finally decided to create an alternative project for your general purpose HTTP needs.

I'm not _yet_ trying to rewrite everything, so this project is still based on `hyper` and friends. However, you _shouldn't_ need to import any other crates to make a fully featured web app. No `tower-*`, `tonic`, `hyper`, `axum`, `http`, `http-body`, etc.

Due to some of the inefficiencies added in converting between `http`/`hyper` and `axol`, I intend to eventually fork `hyper`.

## Current Features

* Compatible with existing `hyper`/`tower` middleware (outside of the `axol` routing layer)
* Robust and composable middleware system with hooks for error management and request/response inspection/mutation.
* `axum`-inspired extractors and responders.
* Wrapper over all of `http`, `http-body`, `hyper`.
* gRPC support with integration via `prost`
* APIs are generally similar to `axum`
* Error type is standardized and not able to be opted-out. It's also very flexible and should meet your needs.

## Planned Features

* Currently middleware can act just like a handler and use `FromRequestParts`/`IntoResponse`/etc. I'd like to make extractors composable in a similar way using proc macros, so that you could bundle them together in a struct.
* gRPC build-time support. At the moment, you'll need to hook up your `prost-build` derived types and the `axol` gRPC system manually. I'm not sure how I want to go about this yet, because I don't want to emulate the clunkiness of `tonic`.
* We always want more useful middleware, extractors, and responders that can be bundled.
* Iterate through dog-fooding. Open GitHub issues with minor issues that are bugging you!
* Client HTTP library