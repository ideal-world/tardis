pub use git_version::*;

#[macro_export]
macro_rules! pkg_version {
    () => {
        env!("CARGO_PKG_VERSION")
    };
}

#[macro_export]
macro_rules! pkg {
    () => {
        env!("CARGO_PKG_NAME")
    };
}
