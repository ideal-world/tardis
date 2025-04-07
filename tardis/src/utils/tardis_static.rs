/// A macro to create a static variable that is lazily initialized, using [`std::sync::OnceLock`] or [`tokio::sync::OnceCell`] to store.
/// # Examples
/// ```ignore
/// tardis_static! {
///    pub config: Config = Config::new();
/// }
///
/// // then use it as a function, it will return a static reference.
/// let config = config();
/// ```
///
/// if you want to initialize the static variable asynchronously, you can use `async` keyword.
/// ```ignore
/// tardis_static! {
///     pub async x: Config = async {
///         wait_other_async().await;
///         retrieve_config().await
///     };
/// }
/// ```
/// for those type that implement [`Default`] trait, you emit the initial value.
/// ```ignore
/// tardis_static! {
///    pub config: Config;
/// }
///
/// ```
#[macro_export]
macro_rules! tardis_static {
    () => {
    };
    ($(#[$attr:meta])* $vis:vis async set $ident:ident :$Type:ty; $($rest: tt)*) => {
        $crate::paste::paste! {
            static [<__ $ident:upper _SYNC>]: OnceLock<$Type> = OnceLock::new();
            $vis fn [<set_ $ident>](init: $Type) {
                [<__ $ident:upper _SYNC>].get_or_init(|| init);
            }
            $(#[$attr])*
            $vis async fn $ident() -> &'static $Type {
                loop {
                    match [<__ $ident:upper _SYNC>].get() {
                        Some(val) => break val,
                        None => { $crate::tokio::task::yield_now().await; }
                    }
                }
            }
            $crate::tardis_static!($($rest)*);
        }
    };
    ($(#[$attr:meta])* $vis:vis async $ident:ident :$Type:ty = $init: expr; $($rest: tt)*) => {
        $(#[$attr])*
        $vis async fn $ident() -> &'static $Type {
            use $crate::tokio::sync::OnceCell;
            static STATIC_VAL: OnceCell<$Type> = OnceCell::const_new();

            STATIC_VAL.get_or_init(|| $init).await
        }
        $crate::tardis_static!($($rest)*);
    };

    ($(#[$attr:meta])* $vis:vis $ident:ident :$Type:ty = $init: expr; $($rest: tt)*) => {
        $(#[$attr])*
        $vis fn $ident() -> &'static $Type {
            use std::sync::OnceLock;
            static STATIC_VAL: OnceLock<$Type> = OnceLock::new();

            STATIC_VAL.get_or_init(|| $init)
        }
        $crate::tardis_static!($($rest)*);
    };

    ($(#[$attr:meta])* $vis:vis $ident:ident :$Type:ty; $($rest: tt)*) => {
        $crate::tardis_static!($(#[$attr])* $vis $ident: $Type = Default::default(); $($rest)*);
    };


}

#[cfg(test)]
#[test]
#[allow(dead_code)]
fn test_tardis_static_macro() {
    #[derive(Default, Clone)]
    struct Config {}

    tardis_static! {
        config: Config = Config::default();
    }
    async fn wait_other_async() {}
    async fn retrieve_config() -> Config {
        config().clone()
    }
    tardis_static! {
        async async_config: Config = async {
            wait_other_async().await;
            retrieve_config().await
        };
    }
    tardis_static! {
        config_default: Config;
    }
}
