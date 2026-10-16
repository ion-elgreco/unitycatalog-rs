//! Logging and optional OpenTelemetry export.
//!
//! Logs go to stdout and, with the `log.dir` setting, to `server.log` in
//! that folder, rolled at 10 MB as upstream's ([`crate::observability::log_file`]).
//! The level comes from `RUST_LOG` (default `info`). With `--features
//! otel` and `OTEL_EXPORTER_OTLP_ENDPOINT` set, spans are also shipped
//! over OTLP gRPC.

use std::sync::Arc;

use tracing_subscriber::{EnvFilter, Layer, Registry, prelude::*};

use crate::observability::log_file::{RollingFile, UpstreamFormat};

type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync>;

/// The stdout layer, and upstream's log file in `log_dir`.
fn layers(log_dir: Option<&std::path::Path>) -> Vec<BoxedLayer> {
    let filter = || EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let mut layers: Vec<BoxedLayer> = vec![
        tracing_subscriber::fmt::layer()
            .with_filter(filter())
            .boxed(),
    ];
    if let Some(dir) = log_dir {
        match RollingFile::new(dir) {
            Ok(file) => layers.push(
                tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .event_format(UpstreamFormat)
                    .with_writer(Arc::new(file))
                    .with_filter(filter())
                    .boxed(),
            ),
            Err(e) => eprintln!("cannot open the log file in {}: {e}", dir.display()),
        }
    }
    layers
}

#[cfg(not(feature = "otel"))]
pub fn init_tracing(log_dir: Option<&std::path::Path>) {
    tracing_subscriber::registry().with(layers(log_dir)).init();
}

#[cfg(feature = "otel")]
pub fn init_tracing(log_dir: Option<&std::path::Path>) {
    use opentelemetry::trace::TracerProvider;

    let registry = tracing_subscriber::registry().with(layers(log_dir));
    if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_err() {
        registry.init();
        return;
    }
    match opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
    {
        Ok(exporter) => {
            let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
                .with_batch_exporter(exporter)
                .with_resource(
                    opentelemetry_sdk::Resource::builder()
                        .with_service_name("uc-server")
                        .build(),
                )
                .build();
            let tracer = provider.tracer("uc-server");
            registry
                .with(tracing_opentelemetry::layer().with_tracer(tracer))
                .init();
        }
        Err(err) => {
            eprintln!("OTel exporter init failed (continuing without OTel): {err}");
            registry.init();
        }
    }
}
