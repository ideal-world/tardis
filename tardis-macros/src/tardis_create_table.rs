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
#[darling(attributes(sea_orm))]
struct CreateTableMeta {
    ident: Option<Ident>,
    ty: Type,
    #[darling(default)]
    primary_key: bool,
    #[darling(default)]
    nullable: bool,
    #[darling(default)]
    extra: Option<String>,
    #[darling(default)]
    //todo 兼容支持自定义类型
    column_type: Option<String>,

    //The following fields are not used temporarily
    // in order to be compatible with the original available parameters of sea_orm
    #[warn(dead_code)]
    #[darling(default)]
    auto_increment: bool,
    #[darling(default)]
    column_name: Option<String>,
    #[darling(default)]
    default_value: Option<String>,
    #[darling(default)]
    unique: bool,
    #[darling(default)]
    indexed: bool,
    #[darling(default)]
    ignore: bool,
    #[darling(default)]
    select_as: Option<String>,
    #[darling(default)]
    save_as: Option<String>,
}

pub(crate) fn create_table(ident: Ident, data: Data, _atr: Vec<Attribute>) -> Result<TokenStream> {
    if ident != "Model" {
        panic!("Struct name must be Model");
    }
    match data {
        Data::Struct(struct_impl) => {
            let col_token = create_col_token_statement(struct_impl.fields)?;
            Ok(
                quote! {fn tardis_create_table_statement(db: DbBackend) -> ::tardis::db::sea_orm::sea_query::TableCreateStatement {
                    let mut builder = ::tardis::db::sea_orm::sea_query::Table::create();
                    builder
                        .table(Entity.table_ref())
                        .if_not_exists()
                        .#col_token;
                    if db == DatabaseBackend::MySql {
                        builder.engine("InnoDB").character_set("utf8mb4").collate("utf8mb4_0900_as_cs");
                    }
                    builder.to_owned()
                }},
            )
        }
        Data::Enum(_) => Err(Error::new(ident.span(), "enum is not support!")),
        Data::Union(_) => Err(Error::new(ident.span(), "union is not support!")),
    }
}

fn create_col_token_statement(fields: Fields) -> Result<TokenStream> {
    let mut result: Punctuated<_, Dot> = Punctuated::new();
    for field in fields {
        let field_create_table_meta: CreateTableMeta = match CreateTableMeta::from_field(&field) {
            Ok(field) => field,
            Err(err) => {
                return Ok(err.write_errors());
            }
        };
        let stream = create_single_col_token_statement(field_create_table_meta)?;
        result.push(stream);
    }
    Ok(result.into_token_stream())
}

fn create_single_col_token_statement(field: CreateTableMeta) -> Result<TokenStream> {
    let field_clone = field.clone();
    let mut attribute: Punctuated<_, Dot> = Punctuated::new();
    if let Some(ident) = field_clone.ident {
        if let Type::Path(field_type) = field_clone.ty {
            if let Some(path) = field_type.path.segments.last() {
                //judge packaging types such as `Option<inner_type>` `Vec<inner_type>` `DateTime<inner_type>`
                if path.ident == "Option" {
                    if let PathArguments::AngleBracketed(path_arg) = &path.arguments {
                        if let Some(GenericArgument::Type(Type::Path(path))) = path_arg.args.first() {
                            if path.path.get_ident().is_some() {
                                return create_single_col_token_statement(CreateTableMeta {
                                    ty: Type::Path(path.clone()),
                                    nullable: true,
                                    ..field
                                });
                            }
                        }
                    }
                } else if path.ident == "Vec" {
                    if let PathArguments::AngleBracketed(path_arg) = &path.arguments {
                        if let Some(GenericArgument::Type(Type::Path(path))) = path_arg.args.first() {
                            if let Some(ident) = path.path.get_ident() {
                                map_type_to_create_table_(ident, &mut attribute, Some("Vec"))?;
                            }
                        }
                    }
                } else if path.ident == "DateTime" {
                    if let PathArguments::AngleBracketed(path_arg) = &path.arguments {
                        if let Some(GenericArgument::Type(Type::Path(path))) = path_arg.args.first() {
                            if let Some(ident) = path.path.get_ident() {
                                map_type_to_create_table_(ident, &mut attribute, Some("DateTime"))?;
                            }
                        }
                    }
                } else if let Some(ident) = field_type.path.get_ident() {
                    // basic type
                    map_type_to_create_table_(ident, &mut attribute, None)?;
                } else {
                    return Err(Error::new(path.span(), "[path.segments] not support Type!"));
                }
            }
        }

        if !field.nullable {
            attribute.push(quote!(not_null()))
        }
        if field.primary_key {
            attribute.push(quote!(primary_key()))
        }
        if let Some(ext) = field.extra {
            attribute.push(quote!(extra(#ext.to_string())))
        }

        let ident = Ident::new(ConvertVariableHelpers::underscore_to_camel(ident.to_string()).as_ref(), ident.span());
        Ok(quote! {col(::tardis::db::sea_orm::sea_query::ColumnDef::new(Column::#ident).#attribute)})
    } else {
        Ok(quote! {})
    }
}
fn map_type_to_create_table_(ident: &Ident, attribute: &mut Punctuated<TokenStream, Dot>, segments_type: Option<&str>) -> Result<()> {
    let map: HashMap<String, TokenStream> = get_type_map(segments_type);

    let ident_string = ident.to_string();
    if let Some(tk) = map.get::<str>(ident_string.as_ref()) {
        attribute.push((*tk).clone());
        Ok(())
    } else {
        Err(Error::new(ident.span(), "type is not impl!"))
    }
}
/// Conversion type reference https://www.sea-ql.org/SeaORM/docs/generate-entity/entity-structure/ \
/// for developer: if you want support more type,just add type map.
fn get_type_map(segments_type: Option<&str>) -> HashMap<String, TokenStream> {
    let mut map: HashMap<String, TokenStream> = HashMap::new();
    #[cfg(feature = "reldb-postgres")]
    {
        match segments_type {
            Some("Vec") => {
                map.insert("u8".to_string(), quote!(binary()));
            }
            Some("DateTime") => {
                map.insert("Utc".to_string(), quote!(timestamp_with_time_zone()));
            }
            None => {
                map.insert("String".to_string(), quote!(string()));
                map.insert("i8".to_string(), quote!(tiny_integer()));
                map.insert("i16".to_string(), quote!(small_integer()));
                map.insert("i32".to_string(), quote!(integer()));
                map.insert("i64".to_string(), quote!(big_integer()));
                map.insert("f32".to_string(), quote!(float()));
                map.insert("f64".to_string(), quote!(double()));
                map.insert("bool".to_string(), quote!(boolean()));
            }
            _ => {}
        }
    }
    map
}
