use crate::macro_helpers::helpers::{ConvertVariableHelpers, TypeToTokenHelpers};
use darling::{FromAttributes, FromField, FromMeta};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use std::collections::HashMap;
use syn::punctuated::Punctuated;
use syn::token::{Comma, Dot};
use syn::{Attribute, Data, Error, Fields, GenericArgument, PathArguments, Result, Type};

#[derive(FromField, Debug, Clone)]
#[darling(attributes(index))]
struct CreateIndexMeta {
    ident: Option<Ident>,
    ty: Type,
    #[darling(default = "default_index_id")]
    index_id: String,
    #[darling(default)]
    name: Option<String>,
    #[darling(default)]
    primary: bool,
    #[darling(default)]
    unique: bool,
    #[darling(default)]
    full_text: bool,
    #[darling(default)]
    index_type:Option<IndexType>,
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
    let mut statement: Punctuated<TokenStream, Comma> = Punctuated::new();
    let mut map: HashMap<String, Box<Vec<CreateIndexMeta>>> = HashMap::new();
    for field in fields {
        for attr in field.attrs.clone() {
            if let Some(ident) = attr.path.get_ident() {
                if ident == "index" {
                    // eprintln!("{:?}====={:?}", field.ident, attr.path.get_ident());
                    let field_create_index_meta: CreateIndexMeta = match CreateIndexMeta::from_field(&field) {
                        Ok(field) => field,
                        Err(err) => {
                            return Ok(err.write_errors());
                        }
                    };
                    if let Some(vec) = map.get_mut(&field_create_index_meta.index_id) {
                        vec.push(field_create_index_meta)
                    } else {
                        map.insert(field_create_index_meta.index_id.clone(), Box::new(vec![field_create_index_meta]));
                    }
                    // out of attr for loop, into next field
                    break;
                }
            }
        }
    }
    for k in map.keys() {
        statement.push(single_create_index_statement(map.get(k).unwrap())?);
    }
    Ok(quote! {#statement})
}
fn single_create_index_statement(index_metas: &Vec<CreateIndexMeta>) -> Result<TokenStream> {
    let mut create_statement: Punctuated<TokenStream, Dot> = Punctuated::new();
    let mut column: Punctuated<TokenStream, Dot> = Punctuated::new();
    let mut name = None;
    let mut primary = false;
    let mut unique = false;
    let mut full_text = false;

    for index_meta in index_metas {
        if let Some(ident) = index_meta.ident.clone() {
            let ident = Ident::new(ConvertVariableHelpers::underscore_to_camel(ident.to_string()).as_ref(), ident.span());
            //add Column
            column.push(quote!(col(Column::#ident)))
        }
        if name.is_none() && index_meta.name.is_some() {
            name = index_meta.name.clone();
        }
        if index_meta.primary {
            primary = true;
        }
        if index_meta.unique {
            unique = true;
        }
        if index_meta.full_text {
            full_text = true;
        }
    }

    if primary {
        create_statement.push(quote!(primary()))
    }
    if unique {
        create_statement.push(quote!(unique()))
    }
    if full_text {
        create_statement.push(quote!(full_text()))
    }

    let all_statement = if create_statement.is_empty() {
        quote! {#column}
    } else {
        quote! {#column.#create_statement}
    };
    if column.is_empty() {
        Ok(quote! {})
    } else {
        let name = if let Some(name) = name {
            TypeToTokenHelpers::string_literal(&Some(name))
        } else {
            //todo 随机生成name
            quote! {&format!("idx-{}-idx1", Entity.table_name())}
        };
        Ok(quote! {::tardis::db::sea_orm::sea_query::Index::create().name(#name).table(Entity).#all_statement.to_owned()})
    }
}
