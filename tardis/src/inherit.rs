use poem::Route;
use std::collections::HashMap;

use crate::web::web_server::TardisWebServer;
use std::cell::Cell;
pub trait InheritableModule {
    type Inherit;
    fn drop(self) -> Self::Inherit
    where
        Self: Sized;
    fn load(&mut self, inherit: Self::Inherit)
    where
        Self: Sized;
}

// pub struct TardisFunsInherit {
//     #[cfg(feature = "web-server")]
//     pub web_server: Option<<TardisWebServer as InheritDrop>::Inherit>,
// }

macro_rules! define_tardis_funs_inherit {
    ($($feat: literal => $module: ident: $module_ty: ty);*) => {
        #[derive(Default)]
        pub struct TardisFunsInherit {
            $(
                #[cfg(feature = $feat)]
                pub $module: Option<<$module_ty as InheritableModule>::Inherit>,
            ),*
        }
    };
}

define_tardis_funs_inherit!{
    "web-server" => web_server: TardisWebServer
}