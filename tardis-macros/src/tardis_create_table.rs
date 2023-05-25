use crate::macro_helpers::helpers::{default_doc, ConvertVariableHelpers};
use darling::FromField;
use proc_macro2::{Ident, Span, TokenStream};
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
    /// custom type , optional see [sea-query::tabled::column::ColumnDef]/[map_type_to_db_type]
    #[darling(default)]
    custom_type: Option<String>,
    /// custom len , when enabled, it needs to be used with custom_type attribute. \
    /// And it will only be enabled when a specific type. \
    /// such as `char(len)`
    #[darling(default)]
    custom_len: Option<u32>,

    /// The following fields are not used temporarily
    /// in order to be compatible with the original available parameters of sea_orm
    #[allow(dead_code)]
    #[darling(default)]
    auto_increment: bool,
    #[allow(dead_code)]
    #[darling(default)]
    column_type: Option<String>,
    #[allow(dead_code)]
    #[darling(default)]
    column_name: Option<String>,
    #[allow(dead_code)]
    #[darling(default)]
    default_value: Option<String>,
    #[allow(dead_code)]
    #[darling(default)]
    unique: bool,
    #[allow(dead_code)]
    #[darling(default)]
    indexed: bool,
    #[allow(dead_code)]
    #[darling(default)]
    ignore: bool,
    #[allow(dead_code)]
    #[darling(default)]
    select_as: Option<String>,
    #[darling(default)]
    #[allow(dead_code)]
    save_as: Option<String>,
}

pub(crate) fn create_table(ident: Ident, data: Data, _atr: impl IntoIterator<Item = Attribute>) -> Result<TokenStream> {
    if ident != "Model" {
        panic!("Struct name must be Model");
    }
    match data {
        Data::Struct(data_struct) => {
            let col_token = create_col_token_statement(data_struct.fields)?;
            let doc = default_doc();
            Ok(quote! {
                #doc
                fn tardis_create_table_statement(db: DbBackend) -> ::tardis::db::sea_orm::sea_query::TableCreateStatement {
                    let mut builder = ::tardis::db::sea_orm::sea_query::Table::create();
                    builder
                        .table(Entity.table_ref())
                        .if_not_exists()
                        .#col_token;
                    if db == DatabaseBackend::MySql {
                        builder.engine("InnoDB").character_set("utf8mb4").collate("utf8mb4_0900_as_cs");
                    }
                    builder.to_owned()
                }
            })
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
        // Priority according to custom_type specifies the corresponding database type to be created/优先根据custom_type指定创建对应数据库类型
        if let Some(custom_column_type) = field.custom_type {
            let db_type = map_type_to_db_type(&custom_column_type, field.custom_len, ident.span())?;
            attribute.push(db_type);
        } else {
            //Automatically convert to corresponding type according to type/根据type自动转换到对应数据库类型
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
                        return Err(Error::new(path.span(), "[path.segments] not support type!"));
                    }
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
    #[cfg(feature = "reldb-mysql")]
    {
        match segments_type {
            Some("Vec") => {
                map.insert("u8".to_string(), quote!(binary()));
            }
            Some("DateTime") => {
                map.insert("Utc".to_string(), quote!(timestamp()));
            }
            None => {
                map.insert("String".to_string(), quote!(string()));
                map.insert("i8".to_string(), quote!(tiny_integer()));
                map.insert("u8".to_string(), quote!(tiny_unsigned()));
                map.insert("i16".to_string(), quote!(small_integer()));
                map.insert("u16".to_string(), quote!(small_unsigned()));
                map.insert("i32".to_string(), quote!(integer()));
                map.insert("u32".to_string(), quote!(unsigned()));
                map.insert("i64".to_string(), quote!(big_integer()));
                map.insert("u64".to_string(), quote!(big_unsigned()));
                map.insert("f32".to_string(), quote!(float()));
                map.insert("f64".to_string(), quote!(double()));
                map.insert("bool".to_string(), quote!(boolean()));
            }
            _ => {}
        }
    }
    map
}
fn map_type_to_db_type(custom_column_type: &str, custom_len: Option<u32>, span: Span) -> Result<TokenStream> {
    let result = match custom_column_type {
        "Char" | "char" => {
            if let Some(len) = custom_len {
                quote!(char_len(#len))
            } else {
                quote!(char())
            }
        }
        "String" | "string" => {
            if let Some(len) = custom_len {
                quote!(string_len(#len))
            } else {
                quote!(string())
            }
        }
        "Text" | "text" => {
            quote!(text())
        }
        "TinyInteger" | "tiny_integer" => {
            quote!(tiny_integer())
        }
        "SmallInteger" | "small_integer" => {
            quote!(small_integer())
        }
        "Integer" | "integer" => {
            quote!(integer())
        }
        "BigInteger" | "big_integer" => {
            quote!(big_integer())
        }
        "TinyUnsigned" | "tiny_unsigned" => {
            quote!(tiny_unsigned())
        }
        "SmallUnsigned" | "small_unsigned" => {
            quote!(small_unsigned())
        }
        "Unsigned" | "unsigned" => {
            quote!(unsigned())
        }
        "BigUnsigned" | "big_unsigned" => {
            quote!(big_unsigned())
        }
        "Float" | "float" => {
            quote!(float())
        }
        "Double" | "double" => {
            quote!(double())
        }
        "Decimal" | "decimal" => {
            quote!(decimal())
        }
        "DateTime" | "date_time" => {
            quote!(date_time())
        }
        "Timestamp" | "timestamp" => {
            quote!(timestamp())
        }
        "TimestampWithTimeZone" | "timestamp_with_time_zone" => {
            quote!(timestamp_with_time_zone())
        }
        "Time" | "time" => {
            quote!(time())
        }
        "Date" | "date" => {
            quote!(date())
        }
        "Binary" | "binary" => {
            if let Some(len) = custom_len {
                quote!(binary_len(#len))
            } else {
                quote!(binary())
            }
        }
        "VarBinary" | "var_binary" => {
            if let Some(len) = custom_len {
                quote!(var_binary(#len))
            } else {
                return Err(Error::new(span, "column_type:var_binary must have custom_len!".to_string()));
            }
        }
        "Bit" | "bit" => {
            if let Some(len) = custom_len {
                quote!(bit(Some(#len)))
            } else {
                quote!(bit(None))
            }
        }
        "VarBit" | "varbit" => {
            if let Some(len) = custom_len {
                quote!(varbit(#len))
            } else {
                return Err(Error::new(span, "column_type:varbit must have custom_len!".to_string()));
            }
        }
        "Boolean" | "boolean" => {
            quote!(boolean())
        }
        "Money" | "money" => {
            quote!(money())
        }
        "Json" | "json" => {
            quote!(json())
        }
        "JsonBinary" | "json_binary" => {
            quote!(json_binary())
        }
        "UUID" | "Uuid" | "uuid" => {
            quote!(uuid())
        }
        "CIDR" | "Cidr" | "cidr" => {
            quote!(cidr())
        }
        "Inet" | "inet" => {
            quote!(inet())
        }
        "MacAddress" | "mac_address" => {
            quote!(mac_address())
        }
        _ => {
            return Err(Error::new(span, format!("column_type:{custom_column_type} is a not support custom type!")));
        }
    };
    Ok(result)
}
