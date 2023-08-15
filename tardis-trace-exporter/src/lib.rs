use opentelemetry::global::Error;


pub fn create_otlp_tracer() -> Result<opentelemetry::sdk::trace::Tracer, Error> {
    use opentelemetry_otlp::WithExportConfig;

    let protocol = std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL").unwrap_or("grpc".to_string());
    let mut tracer = opentelemetry_otlp::new_pipeline().tracing();
    match protocol.as_str() {
        "grpc" => {
            let mut exporter = opentelemetry_otlp::new_exporter().tonic().with_env();
            // Check if we need TLS
            if let Ok(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
                if endpoint.to_lowercase().starts_with("https") {
                    exporter = exporter.with_tls_config(Default::default());
                }
            }
            tracer = tracer.with_exporter(exporter)
        }
        "http/protobuf" => {
            let headers = parse_otlp_headers_from_env();
            let exporter = opentelemetry_otlp::new_exporter().http().with_headers(headers.into_iter().collect()).with_env();
            tracer = tracer.with_exporter(exporter)
        }
        // p => return Err(TardisError::conflict(&format!("[Tracing] Unsupported protocol {p}"), "")),
        p => return Err(Error::Other(format!("[Tracing] Unsupported protocol {p}"))),
    };
    Ok(tracer.install_batch(opentelemetry::runtime::Tokio)?)
}

fn parse_otlp_headers_from_env() -> Vec<(String, String)> {
    let mut headers = Vec::new();

    if let Ok(hdrs) = std::env::var("OTEL_EXPORTER_OTLP_HEADERS") {
        hdrs.split(',')
            .map(|header| header.split_once('=').expect("Header should contain '=' character"))
            .for_each(|(name, value)| headers.push((name.to_owned(), value.to_owned())));
    }
    headers
}
