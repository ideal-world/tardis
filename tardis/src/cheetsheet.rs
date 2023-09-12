use std::net::IpAddr;

// IP addresses
pub const IP_LOCALHOST: IpAddr = IpAddr::V4(std::net::Ipv4Addr::LOCALHOST);
pub const IP_BROADCAST: IpAddr = IpAddr::V4(std::net::Ipv4Addr::BROADCAST);
pub const IP_UNSPECIFIED: IpAddr = IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED);

pub const IPV6_LOCALHOST: IpAddr = IpAddr::V6(std::net::Ipv6Addr::LOCALHOST);
pub const IPV6_UNSPECIFIED: IpAddr = IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED);