use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_subscriber::{EnvFilter, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn build_provider(service_name: &str) -> SdkTracerProvider {
    use opentelemetry::KeyValue;
    use opentelemetry_sdk::Resource;

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("failed to create OTLP span exporter");

    SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_attributes([KeyValue::new("service.name", service_name.to_owned())])
                .build(),
        )
        .build()
}

pub struct OtelGuard {
    provider: SdkTracerProvider,
}

impl OtelGuard {
    pub fn shutdown(self) {
        if let Err(e) = self.provider.shutdown() {
            eprintln!("OpenTelemetry shutdown error: {e}");
        }
    }
}

pub fn init_compact_with_otel(service_name: &str) -> OtelGuard {
    let provider = build_provider(service_name);
    let tracer = provider.tracer(service_name.to_owned());
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    Registry::default()
        .with(env_filter)
        .with(fmt::layer().json())
        .with(otel_layer)
        .init();

    OtelGuard { provider }
}
