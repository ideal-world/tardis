use std::net::IpAddr;

// IP addresses
pub const IP_LOCALHOST: IpAddr = IpAddr::V4(std::net::Ipv4Addr::LOCALHOST);
pub const IP_BROADCAST: IpAddr = IpAddr::V4(std::net::Ipv4Addr::BROADCAST);
pub const IP_UNSPECIFIED: IpAddr = IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED);

pub const IPV6_LOCALHOST: IpAddr = IpAddr::V6(std::net::Ipv6Addr::LOCALHOST);
pub const IPV6_UNSPECIFIED: IpAddr = IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED);

// env var keys

// opentelemetry
pub const OTEL_EXPORTER_OTLP_ENDPOINT: &str = "OTEL_EXPORTER_OTLP_ENDPOINT";
pub const OTEL_EXPORTER_OTLP_PROTOCOL: &str = "OTEL_EXPORTER_OTLP_PROTOCOL";
pub const OTEL_EXPORTER_OTLP_HEADERS: &str = "OTEL_EXPORTER_OTLP_HEADERS";
pub const OTEL_SERVICE_NAME: &str = "OTEL_SERVICE_NAME";

// shortcuts for build info
pub const TARDIS_VERSION: &str = env!("CARGO_PKG_VERSION");
