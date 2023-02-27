use crate::macro_helpers::helpers::ConvertVariableHelpers;
use darling::FromField;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use std::collections::HashMap;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Dot;
use syn::{Attribute, Data, Error, Fields, GenericArgument, PathArguments, Result, Type};

#[derive(FromField, Debug, Clone)]
#[darling(attributes(index))]
struct CreateIndexMeta {
    ident: Option<Ident>,
    ty: Type,
    #[darling(default = "default_index_id")]
    index_id: String,
}
fn default_index_id() -> String {
    "index_id_1".to_string()
}
pub(crate) fn create_index(ident: Ident, data: Data, _atr: Vec<Attribute>) -> Result<TokenStream> {
    if ident != "Model" {
        panic!("Struct name must be Model");
    }
    match data {
        Data::Struct(struct_impl) => {
            let col_token = create_col_token_statement(struct_impl.fields)?;

            Ok(quote! {fn tardis_create_index_statement() -> Vec<::tardis::db::sea_orm::sea_query::IndexCreateStatement> {
                vec![
                    #col_token
                    ]
            }})
        }
        Data::Enum(_) => Err(Error::new(ident.span(), "enum is not support!")),
        Data::Union(_) => Err(Error::new(ident.span(), "union is not support!")),
    }
}

fn create_col_token_statement(fields: Fields) -> Result<TokenStream> {
    for field in fields {
        let field_create_index_meta: CreateIndexMeta = match CreateIndexMeta::from_field(&field) {
            Ok(field) => field,
            Err(err) => {
                return Ok(err.write_errors());
            }
        };
    }
    Ok(quote! {})
}
