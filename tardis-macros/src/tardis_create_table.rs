use crate::macro_helpers::helpers::ConvertVariableHelpers;
use darling::FromField;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use std::collections::HashMap;
use syn::punctuated::Punctuated;
use syn::token::Dot;
use syn::{Attribute, Data, Error, Field, Fields, GenericArgument, ImplItemMethod, ItemImpl, ItemStruct, Meta, PathArguments, Result, Stmt, Type};

#[derive(FromField, Debug)]
#[darling(attributes(sea_orm))]
struct CreateTableMeta {
    ident: Option<Ident>,
    ty: Type,
    #[darling(default)]
    primary_key: bool,
    #[darling(default)]
    is_null: bool,
    #[darling(default)]
    auto_increment: bool,
}

pub(crate) fn create_table(ident: Ident, data: Data, _atr: Vec<Attribute>) -> Result<TokenStream> {
    if ident != "Model" {
        panic!("Struct name must be Model");
    }
    match data {
        Data::Struct(struct_impl) => {
            let col_token = create_col_token_statement(struct_impl.fields)?;
            Ok(quote! {fn create_table_statement(db: DbBackend) -> ::tardis::db::sea_orm::sea_query::TableCreateStatement {
                let mut builder = ::tardis::db::sea_orm::sea_query::Table::create();
                builder
                    .table(Entity.table_ref())
                    .if_not_exists()
                    .#col_token;
                if db == DatabaseBackend::MySql {
                    builder.engine("InnoDB").character_set("utf8mb4").collate("utf8mb4_0900_as_cs");
                }
                builder.to_owned()
            }})
        }
        Data::Enum(_) => Err(Error::new(ident.span(), "enum is not support!")),
        Data::Union(_) => Err(Error::new(ident.span(), "union is not support!")),
    }
}

fn create_col_token_statement(fields: Fields) -> Result<TokenStream> {
    let mut result: Punctuated<_, Dot> = Punctuated::new();
    for field in fields {
        let stream = create_single_col_token_statement(field)?;
        result.push(stream);
    }
    Ok(result.into_token_stream())
}

fn create_single_col_token_statement(field: Field) -> Result<TokenStream> {
    let mut field_create_table_meta: CreateTableMeta = match CreateTableMeta::from_field(&field) {
        Ok(field) => field,
        Err(err) => {
            return Ok(err.write_errors());
        }
    };
    let mut attribute: Punctuated<_, Dot> = Punctuated::new();
    if let Some(ident) = field_create_table_meta.ident {
        if let Type::Path(path) = field_create_table_meta.ty {
            // eprintln!("Type===={path:?}");
            if let Some(path) = path.path.segments.first() {
                if path.ident == "Option" {
                    field_create_table_meta.is_null = true;
                    if let PathArguments::AngleBracketed(patharg) = &path.arguments {
                        if let Some(patharg) = patharg.args.first() {
                            if let GenericArgument::Type(path_type) = patharg {
                                if let Type::Path(path) = path_type {
                                    if let Some(ident) = path.path.get_ident() {
                                        map_type_to_create_table_(ident, &mut attribute)?;
                                    }
                                }
                            }
                        }
                    }
                } else {
                }
            }
            if let Some(ident) = path.path.get_ident() {
                map_type_to_create_table_(ident, &mut attribute)?;
            }
        }

        if !field_create_table_meta.is_null {
            attribute.push(quote!(not_null()))
        }
        if field_create_table_meta.primary_key {
            attribute.push(quote!(primary_key()))
        }

        let ident = Ident::new(&ConvertVariableHelpers::underscore_to_camel(ident.to_string()), ident.span());
        Ok(quote! {col(::tardis::db::sea_orm::sea_query::ColumnDef::new(Column::#ident).#attribute)})
    } else {
        Ok(quote! {})
    }
}
fn map_type_to_create_table_(ident: &Ident, attribute: &mut Punctuated<TokenStream, Dot>) -> Result<()> {
    let map = get_type_map();

    if let Some(tk) = map.get(&ident.to_string()) {
        attribute.push(tk.clone());
        Ok(())
    } else {
        Err(Error::new(ident.span(), "type is not impl!"))
    }
}
fn get_type_map() -> HashMap<String, TokenStream> {
    #[cfg(feature = "reldb-postgres")]
    {
        let mut map: HashMap<String, TokenStream> = HashMap::new();
        map.insert("String".to_string(), quote!(string()));
        map.insert("i64".to_string(), quote!(big_integer()));
        map.insert("bool".to_string(), quote!(boolean()));

        return map;
    }
}
