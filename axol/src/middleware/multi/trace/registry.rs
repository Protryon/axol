use std::sync::Arc;

use tracing::{Subscriber, Metadata, subscriber::Interest, span, Event};
use tracing_core::span::Current;
use tracing_subscriber::{Registry, registry::{LookupSpan, Data}, filter::FilterId};

#[derive(Clone)]
pub struct RegistryWrapper(pub Arc<Registry>);

impl From<Registry> for RegistryWrapper {
    fn from(value: Registry) -> Self {
        Self(Arc::new(value))
    }
}

impl Subscriber for RegistryWrapper {
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        self.0.register_callsite(metadata)
    }

    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.0.enabled(metadata)
    }

    #[inline]
    fn new_span(&self, attrs: &span::Attributes<'_>) -> span::Id {
        self.0.new_span(attrs)
    }

    #[inline]
    fn record(&self, id: &span::Id, record: &span::Record<'_>) {
        self.0.record(id, record)
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        self.0.record_follows_from(span, follows)
    }

    fn event_enabled(&self, event: &Event<'_>) -> bool {
        self.0.event_enabled(event)
    }

    fn event(&self, event: &Event<'_>) {
        self.0.event(event)
    }

    fn enter(&self, id: &span::Id) {
        self.0.enter(id)
    }

    fn exit(&self, id: &span::Id) {
        self.0.exit(id)
    }

    fn clone_span(&self, id: &span::Id) -> span::Id {
        self.0.clone_span(id)
    }

    fn current_span(&self) -> Current {
        self.0.current_span()
    }

    fn try_close(&self, id: span::Id) -> bool {
        self.0.try_close(id)
    }
}

impl<'a> LookupSpan<'a> for RegistryWrapper {
    type Data = Data<'a>;

    fn span_data(&'a self, id: &span::Id) -> Option<Self::Data> {
        self.0.span_data(id)
    }

    fn register_filter(&mut self) -> FilterId {
        if let Some(self_) = Arc::get_mut(&mut self.0) {
            self_.register_filter()
        } else {
            panic!("called register_filter after RegistryWrapper was cloned");
        }
    }
}
