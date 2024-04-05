use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use tracing::subscriber::set_global_default;
use tracing::Level;
use tracing::Span;
use tracing_actix_web::{DefaultRootSpanBuilder, RootSpanBuilder, TracingLogger};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

use crate::prelude::PREFIX;

pub struct Telemetry<T>
where
    T: SubscriberExt + Send + Sync + 'static,
{
    pub subscriber: T,
}

/// Compose multiple layers into a `tracing`'s subscriber.
///
/// # Implementation Notes
///
/// We are using `impl Subscriber` as return type to avoid having to spell out the actual
/// type of the returned subscriber, which is indeed quite complex.
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> Telemetry<impl SubscriberExt + Send + Sync + 'static>
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    // JSON formatting layer
    let formatting_layer: BunyanFormattingLayer<Sink> = BunyanFormattingLayer::new(
        name, // Output the JSON logs to the stdout.
        sink,
    );

    // Filter Layer
    let filter_layer =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    Telemetry {
        subscriber: Registry::default()
            .with(filter_layer)
            .with(JsonStorageLayer)
            .with(formatting_layer),
    }
}

/// Register a subscriber as global default to process span data.
///
/// It should only be called once!
pub fn init_subscriber(subscriber: Telemetry<impl SubscriberExt + Send + Sync + 'static>) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber.subscriber).expect("Failed to set subscriber");
}

pub struct SpanBuilder;

impl RootSpanBuilder for SpanBuilder {
    fn on_request_start(request: &ServiceRequest) -> Span {
        let level = if request.path() == PREFIX.to_owned() + "/health_check"
            || request.path() == PREFIX.to_owned() + "/metrics"
        {
            Level::TRACE
        } else {
            Level::INFO
        };
        tracing_actix_web::root_span!(level = level, request)
    }

    fn on_request_end<B: MessageBody>(
        span: Span,
        outcome: &Result<ServiceResponse<B>, actix_web::Error>,
    ) {
        DefaultRootSpanBuilder::on_request_end(span, outcome);
    }
}

pub struct Tracer {
    tracer: TracingLogger<SpanBuilder>,
}

impl Tracer {
    pub fn new() -> Self {
        let tracer = TracingLogger::<SpanBuilder>::new();

        Tracer { tracer }
    }

    pub fn tracer(&self) -> TracingLogger<SpanBuilder> {
        self.tracer.clone()
    }
}

impl Default for Tracer {
    fn default() -> Self {
        Self::new()
    }
}
