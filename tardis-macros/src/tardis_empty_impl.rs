use crate::macro_helpers::helpers::default_doc;

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use syn::{Data, Error, Result};

pub(crate) fn create_empty_behavior(ident: Ident, data: Data) -> Result<TokenStream> {
    if ident != "Model" {
        panic!("Struct name must be Model");
    }
    match data {
        Data::Struct(_) => {
            let doc = default_doc();

            Ok(quote! {
                #doc
                impl ::tardis::db::sea_orm::ActiveModelBehavior for ActiveModel {}
            })
        }
        Data::Enum(_) => Err(Error::new(ident.span(), "enum is not support!")),
        Data::Union(_) => Err(Error::new(ident.span(), "union is not support!")),
    }
}

pub(crate) fn create_empty_relation(ident: Ident, data: Data) -> Result<TokenStream> {
    if ident != "Model" {
        panic!("Struct name must be Model");
    }
    match data {
        Data::Struct(_) => {
            let doc = default_doc();

            Ok(quote! {
                #doc
                #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
                pub enum Relation {}
            })
        }
        Data::Enum(_) => Err(Error::new(ident.span(), "enum is not support!")),
        Data::Union(_) => Err(Error::new(ident.span(), "union is not support!")),
    }
}
