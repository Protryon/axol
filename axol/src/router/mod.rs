use std::sync::Arc;

use crate::{EarlyResponseHook, Error, ErrorHook, Handler, LateResponseHook, RequestHook, Result};
use axol_http::{response::Response, Method};
use log::warn;

type Route = Arc<dyn Handler>;

#[derive(PartialEq, Clone)]
enum Segment {
    Literal(String),
    Variable(Arc<str>),
}

impl Default for Segment {
    fn default() -> Self {
        Segment::Literal(String::new())
    }
}

#[derive(Default, Clone)]
pub struct Router {
    segment: Segment,
    subpaths: Vec<Router>,
    methods: Vec<(Method, Route)>,
    request_hooks: Vec<Arc<dyn RequestHook>>,
    early_response_hooks: Vec<Arc<dyn EarlyResponseHook>>,
    late_response_hooks: Vec<Arc<dyn LateResponseHook>>,
    error_hooks: Vec<Arc<dyn ErrorHook>>,
    fallback: Option<Route>,
}

pub struct PathVariables(pub Vec<(Arc<str>, String)>);

fn split_path_reverse(path: &str) -> Vec<Segment> {
    path.trim()
        .split('/')
        .filter(|x| !x.is_empty())
        .map(|x| x.trim())
        .rev()
        .map(|x| {
            if x.starts_with(':') {
                Segment::Variable(x[1..].to_string().into())
            } else {
                Segment::Literal(x.to_string())
            }
        })
        .collect()
}

fn split_raw_path_reverse(path: &str) -> Vec<&str> {
    path.trim()
        .split('/')
        .filter(|x| !x.is_empty())
        .map(|x| x.trim())
        .rev()
        .collect()
}

async fn default_route() -> Result<Response> {
    Err(Error::NotFound)
}

lazy_static::lazy_static! {
    static ref DEFAULT_ROUTE: Arc<dyn Handler> = Arc::new(default_route);
}

pub struct ObservedRoute<'a> {
    pub route: &'a Route,
    pub variables: PathVariables,
    pub request_hooks: Vec<&'a dyn RequestHook>,
    pub error_hooks: Vec<&'a dyn ErrorHook>,
    pub early_response_hooks: Vec<&'a dyn EarlyResponseHook>,
    pub late_response_hooks: Vec<&'a dyn LateResponseHook>,
}

impl<'a> ObservedRoute<'a> {
    fn check(&self) -> ObservedRouteCheck {
        ObservedRouteCheck {
            variables: self.variables.0.len(),
            request_hooks: self.request_hooks.len(),
            error_hooks: self.error_hooks.len(),
            early_response_hooks: self.early_response_hooks.len(),
            late_response_hooks: self.late_response_hooks.len(),
        }
    }

    fn reset(&mut self, check: ObservedRouteCheck) {
        self.variables.0.truncate(check.variables);
        self.request_hooks.truncate(check.request_hooks);
        self.error_hooks.truncate(check.error_hooks);
        self.early_response_hooks
            .truncate(check.early_response_hooks);
        self.late_response_hooks.truncate(check.late_response_hooks);
    }
}

struct ObservedRouteCheck {
    variables: usize,
    request_hooks: usize,
    error_hooks: usize,
    early_response_hooks: usize,
    late_response_hooks: usize,
}

impl Router {
    pub fn new() -> Self {
        Router::default()
    }

    pub fn resolve_path(&self, method: Method, path: &str) -> ObservedRoute<'_> {
        let mut out = ObservedRoute {
            route: &DEFAULT_ROUTE,
            variables: PathVariables(vec![]),
            request_hooks: vec![],
            error_hooks: vec![],
            early_response_hooks: vec![],
            late_response_hooks: vec![],
        };
        if let Some(route) = self.do_resolve_path(&mut out, method, &split_raw_path_reverse(path)) {
            out.route = &*route;
        }
        out
    }

    fn do_resolve_path(
        &self,
        observed: &mut ObservedRoute<'_>,
        method: Method,
        segments: &[&str],
    ) -> Option<&Route> {
        let Some(segment) = segments.first() else {
            if let Some((_, route)) = self.methods.iter().find(|x| x.0 == method) {
                return Some(route);
            }
            if method == Method::Head {
                if let Some((_, route)) = self.methods.iter().find(|x| x.0 == Method::Get) {
                    return Some(route);
                }
            }
            return self.fallback.as_ref();
        };
        // find existing segment
        let mut variable_subpath: Option<&Router> = None;
        for subpath in self.subpaths.iter() {
            match &subpath.segment {
                Segment::Literal(literal) => {
                    if literal == segment {
                        let check = observed.check();
                        if let Some(route) =
                            subpath.do_resolve_path(observed, method, &segments[1..])
                        {
                            return Some(route);
                        }
                        observed.reset(check);
                    }
                }
                Segment::Variable(_) => {
                    variable_subpath = Some(subpath);
                    // we delay using the variable path in case there is a literal that supersedes it below
                }
            }
        }
        if let Some(subpath) = variable_subpath {
            let name = match &subpath.segment {
                Segment::Variable(x) => x,
                _ => unreachable!(),
            };
            let check = observed.check();
            observed
                .variables
                .0
                .push((name.clone(), segment.to_string()));
            if let Some(route) = subpath.do_resolve_path(observed, method, &segments[1..]) {
                return Some(route);
            }
            observed.reset(check);
        }

        self.fallback.as_ref()
    }

    fn resolve_segments_mut(&mut self, mut segments: Vec<Segment>) -> &mut Router {
        let Some(segment) = segments.pop() else {
            return self;
        };
        // find existing segment
        let mut subpath_index = None::<usize>;
        for (i, subpath) in self.subpaths.iter().enumerate() {
            if subpath.segment == segment {
                subpath_index = Some(i);
            }
        }
        // bizarre borrow checker shenanigans
        if let Some(i) = subpath_index {
            return self.subpaths[i].resolve_segments_mut(segments);
        }
        if matches!(segment, Segment::Variable(_))
            && self
                .subpaths
                .iter()
                .filter(|x| matches!(x.segment, Segment::Variable(_)))
                .count()
                > 0
        {
            panic!("each routing level at the same superpath must use the same variable name. i.e. `/api/:var` and `/api/:variable` are invalid");
        }
        let mut subrouter = Router::new();
        subrouter.segment = segment;
        self.subpaths.push(subrouter);
        self.subpaths.last_mut().unwrap()
    }

    fn append_segment(&mut self, segments: Vec<Segment>, method: Method, route: Route) {
        let target = self.resolve_segments_mut(segments);
        if let Some(handler) = target
            .methods
            .iter_mut()
            .find(|(current_method, _)| current_method == &method)
        {
            warn!("overwriting route for method {method}");
            handler.1 = route;
        } else {
            target.methods.push((method, route));
        }
    }

    pub fn method(mut self, path: &str, method: Method, route: impl Handler) -> Self {
        let handler: Arc<dyn Handler> = Arc::new(route);
        self.append_segment(split_path_reverse(path), method, handler);
        self
    }

    pub fn get(self, path: &str, route: impl Handler) -> Self {
        self.method(path, Method::Get, route)
    }

    pub fn post(self, path: &str, route: impl Handler) -> Self {
        self.method(path, Method::Post, route)
    }

    pub fn put(self, path: &str, route: impl Handler) -> Self {
        self.method(path, Method::Put, route)
    }

    pub fn delete(self, path: &str, route: impl Handler) -> Self {
        self.method(path, Method::Delete, route)
    }

    pub fn head(self, path: &str, route: impl Handler) -> Self {
        self.method(path, Method::Head, route)
    }

    pub fn options(self, path: &str, route: impl Handler) -> Self {
        self.method(path, Method::Options, route)
    }

    pub fn connect(self, path: &str, route: impl Handler) -> Self {
        self.method(path, Method::Connect, route)
    }

    pub fn patch(self, path: &str, route: impl Handler) -> Self {
        self.method(path, Method::Patch, route)
    }

    pub fn trace(self, path: &str, route: impl Handler) -> Self {
        self.method(path, Method::Trace, route)
    }

    pub fn fallback(mut self, path: &str, fallback: impl Handler) -> Self {
        let segments = split_path_reverse(path);
        let handler: Arc<dyn Handler> = Arc::new(fallback);
        let target = self.resolve_segments_mut(segments);
        if let Some(fallback) = target.fallback.as_mut() {
            warn!("overwriting route for fallback");
            *fallback = handler;
        } else {
            target.fallback = Some(handler);
        }
        self
    }

    pub fn error_hook(mut self, path: &str, hook: impl ErrorHook) -> Self {
        let segments = split_path_reverse(path);
        let hook: Arc<dyn ErrorHook> = Arc::new(hook);
        let target = self.resolve_segments_mut(segments);
        target.error_hooks.push(hook);
        self
    }

    pub fn request_hook(mut self, path: &str, hook: impl RequestHook) -> Self {
        let segments = split_path_reverse(path);
        let hook: Arc<dyn RequestHook> = Arc::new(hook);
        let target = self.resolve_segments_mut(segments);
        target.request_hooks.push(hook);
        self
    }

    pub fn early_response_hook(mut self, path: &str, hook: impl EarlyResponseHook) -> Self {
        let segments = split_path_reverse(path);
        let hook: Arc<dyn EarlyResponseHook> = Arc::new(hook);
        let target = self.resolve_segments_mut(segments);
        target.early_response_hooks.push(hook);
        self
    }

    pub fn late_response_hook(mut self, path: &str, hook: impl LateResponseHook) -> Self {
        let segments = split_path_reverse(path);
        let hook: Arc<dyn LateResponseHook> = Arc::new(hook);
        let target = self.resolve_segments_mut(segments);
        target.late_response_hooks.push(hook);
        self
    }

    pub fn nest(mut self, path: &str, router: Router) -> Self {
        let segments = split_path_reverse(path);
        let target = self.resolve_segments_mut(segments);
        target.do_merge(router);
        self
    }

    /// Same as nest with path = '/'
    pub fn merge(self, router: Router) -> Self {
        self.nest("/", router)
    }

    fn do_merge(&mut self, router: Router) {
        for (method, route) in router.methods {
            self.append_segment(vec![], method, route);
        }
        if let Some(fallback) = router.fallback {
            self.fallback = Some(fallback);
        }
        for subpath in router.subpaths {
            let subtarget = self.resolve_segments_mut(vec![subpath.segment.clone()]);
            subtarget.do_merge(subpath);
        }
    }
}
