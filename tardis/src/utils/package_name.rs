#[macro_export]
macro_rules! pkg {
    () => {
        env!("CARGO_PKG_NAME")
    };
}
