use crate::macro_helpers::helpers::{default_doc, ConvertVariableHelpers};
use darling::FromField;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Dot;
use syn::{Attribute, Data, Error, Expr, Field, Fields, GenericArgument, PathArguments, Result, Type};

#[derive(FromField, Debug, Clone)]
#[darling(attributes(tardis_entity))]
struct TardisEntityMeta {
    ident: Option<Ident>,
    ty: Type,
    /// custom type , optional see [sea-query::tabled::column::ColumnDef]/[map_type_to_db_type]
    #[darling(default)]
    custom_type: Option<String>,
    /// custom len
    /// type: array
    /// ```rust ignore
    /// #[tardis_entity(custom_len = "10", custom_len = "2")]
    /// ```
    #[darling(default)]
    #[darling(multiple)]
    custom_len: Vec<u32>,
    #[darling(default)]
    default_value: Option<Expr>,
}

#[derive(FromField, Debug, Clone)]
#[darling(attributes(sea_orm))]
struct SeaOrmMeta {
    #[darling(default)]
    primary_key: bool,
    #[darling(default)]
    nullable: Option<bool>,
    #[darling(default)]
    extra: Option<String>,
    #[darling(default)]
    ignore: bool,
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
    select_as: Option<String>,
    #[darling(default)]
    #[allow(dead_code)]
    save_as: Option<String>,
}

#[derive(Debug, Clone)]
struct CreateTableMeta {
    ident: Option<Ident>,
    ty: Type,
    custom_type: Option<String>,
    custom_len: Vec<u32>,

    primary_key: bool,
    nullable: Option<bool>,
    extra: Option<String>,
    default_value: Option<Expr>,
    ignore: bool,
}

impl CreateTableMeta {
    pub fn from(meta1: TardisEntityMeta, meta2: SeaOrmMeta) -> Self {
        Self {
            ident: meta1.ident,
            ty: meta1.ty,
            custom_type: meta1.custom_type,
            custom_len: meta1.custom_len,
            primary_key: meta2.primary_key,
            nullable: meta2.nullable,
            extra: meta2.extra,
            ignore: meta2.ignore,
            default_value: meta1.default_value,
        }
    }
    pub fn from_field(field: &Field) -> darling::Result<Self> {
        let field_create_table_meta: TardisEntityMeta = TardisEntityMeta::from_field(field)?;
        let field_sea_orm_meta: SeaOrmMeta = SeaOrmMeta::from_field(field)?;
        Ok(Self::from(field_create_table_meta, field_sea_orm_meta))
    }
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
                fn tardis_create_table_statement(db: ::tardis::db::sea_orm::DbBackend) -> ::tardis::db::sea_orm::sea_query::TableCreateStatement {
                    let mut builder = ::tardis::db::sea_orm::sea_query::Table::create();
                    builder
                        .table(Entity.table_ref())
                        .if_not_exists()
                        .#col_token;
                    if db == ::tardis::db::sea_orm::DatabaseBackend::MySql {
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
        let field_create_table_meta = match CreateTableMeta::from_field(&field) {
            Ok(field) => field,
            Err(err) => {
                return Ok(err.write_errors());
            }
        };
        if field_create_table_meta.ignore {
            continue;
        }
        let stream = create_single_col_token_statement(field_create_table_meta)?;
        result.push(stream);
    }
    Ok(result.into_token_stream())
}

fn create_single_col_token_statement(field: CreateTableMeta) -> Result<TokenStream> {
    let field_clone = field.clone();
    let mut attribute: Punctuated<_, Dot> = Punctuated::new();
    let mut col_type = TokenStream::new();
    if let Some(ident) = field_clone.ident {
        if let Type::Path(field_type) = field_clone.ty {
            if let Some(path) = field_type.path.segments.last() {
                // Judge nullable/判断是否为nullable
                if path.ident == "Option" && field.nullable.is_none() {
                    if let PathArguments::AngleBracketed(path_arg) = &path.arguments {
                        if let Some(GenericArgument::Type(Type::Path(path))) = path_arg.args.first() {
                            return create_single_col_token_statement(CreateTableMeta {
                                ty: Type::Path(path.clone()),
                                nullable: Some(true),
                                ..field
                            });
                        }
                    };
                }
                // Priority according to custom_type specifies the corresponding database type to be created/优先根据custom_type指定创建对应数据库类型
                if let Some(custom_column_type) = field.custom_type {
                    col_type = map_custom_type_to_sea_type(&custom_column_type, field.custom_len, ident.span())?;
                } else {
                    // Automatically convert to corresponding type according to type/根据type自动转换到对应数据库类型

                    // Judge packaging types such as `Vec<inner_type>` `DateTime<inner_type>`
                    if path.ident == "Vec" {
                        if let PathArguments::AngleBracketed(path_arg) = &path.arguments {
                            if let Some(GenericArgument::Type(Type::Path(path))) = path_arg.args.first() {
                                if let Some(ident) = path.path.get_ident() {
                                    let custom_ty = map_rust_ty_custom_ty(ident, Some("Vec"))?;
                                    col_type = map_custom_type_to_sea_type(&custom_ty, field.custom_len, ident.span())?;
                                }
                            }
                        }
                    } else if path.ident == "DateTime" {
                        if let PathArguments::AngleBracketed(path_arg) = &path.arguments {
                            if let Some(GenericArgument::Type(Type::Path(path))) = path_arg.args.first() {
                                if let Some(ident) = path.path.get_ident() {
                                    let custom_ty = map_rust_ty_custom_ty(ident, Some("DateTime"))?;
                                    col_type = map_custom_type_to_sea_type(&custom_ty, field.custom_len, ident.span())?;
                                }
                            }
                        }
                    } else if let Some(ident) = field_type.path.get_ident() {
                        // single literal type
                        let custom_ty = map_rust_ty_custom_ty(ident, None)?;
                        col_type = map_custom_type_to_sea_type(&custom_ty, field.custom_len, ident.span())?;
                    } else {
                        return Err(Error::new(path.span(), "[path.segments] not support yet! please use single literal"));
                    }
                }
            }
        }
        if field.nullable.is_none() || !field.nullable.unwrap() {
            attribute.push(quote!(not_null()))
        }
        if field.primary_key {
            attribute.push(quote!(primary_key()))
        }
        if let Some(ext) = field.extra {
            attribute.push(quote!(extra(#ext.to_string())))
        }
        if let Some(default_value) = field.default_value {
            attribute.push(quote!(default(#default_value)))
        }

        let ident = Ident::new(ConvertVariableHelpers::underscore_to_camel(ident.to_string()).as_ref(), ident.span());
        if col_type.is_empty() {
            return Err(Error::new(
                field.ident.span(),
                "Failed to parse the type. Please try using custom_type to specify the type.",
            ));
        }
        if attribute.is_empty() {
            Ok(quote! {col(::tardis::db::sea_orm::sea_query::ColumnDef::new_with_type(Column::#ident,#col_type))})
        } else {
            Ok(quote! {col(::tardis::db::sea_orm::sea_query::ColumnDef::new_with_type(Column::#ident,#col_type).#attribute)})
        }
    } else {
        Ok(quote! {})
    }
}

/// # Map rust to intermediate type
///
/// Convert to an intermediate type based on the rust data type.
/// This intermediate type is finally converted to the sea_type type through the [map_custom_type_to_sea_type] method.
///
/// Conversion type reference https://www.sea-ql.org/SeaORM/docs/generate-entity/entity-structure/
#[allow(clippy::wildcard_in_or_patterns)]
fn map_rust_ty_custom_ty(ident: &Ident, segments_type: Option<&str>) -> Result<String> {
    let ident_string = ident.to_string();
    let span = ident.span();
    let custom_ty = match ident_string.as_str() {
        "String" | "Decimal" => ident_string.as_str(),
        "i8" => "tiny_integer",
        "u8" => {
            if let Some("Vec") = segments_type {
                "var_binary"
            } else if cfg!(feature = "reldb-postgres") {
                return Err(Error::new(
                    span,
                    "Not supported! u8 is not supported when the 'reldb-postgres' feature is enabled. ".to_string(),
                ));
            } else {
                "tiny_unsigned"
            }
        }
        "i16" => "small_integer",
        "u16" => {
            if cfg!(feature = "reldb-postgres") {
                return Err(Error::new(
                    span,
                    "Not supported! u16 is not supported when the 'reldb-postgres' feature is enabled. ".to_string(),
                ));
            } else {
                "small_unsigned"
            }
        }
        "i32" => "integer",
        "u32" => {
            if cfg!(feature = "reldb-postgres") {
                return Err(Error::new(
                    span,
                    "Not supported! u32 is not supported when the 'reldb-postgres' feature is enabled. ".to_string(),
                ));
            } else {
                "unsigned"
            }
        }
        "i64" => "big_integer",
        "u64" => {
            if cfg!(feature = "reldb-mysql") {
                "big_unsigned"
            } else {
                return Err(Error::new(
                    span,
                    "Not supported! u64 only supported when the 'reldb-mysql' feature is enabled. ".to_string(),
                ));
            }
        }
        "f32" => "float",
        "f64" => "double",
        "bool" => "boolean",
        "NaiveDate" | "Date" => "date",
        "NaiveDateTime" | "PrimitiveDateTime" => "DateTime",
        "Local" | "Utc" => {
            if let Some("DateTime") = segments_type {
                if cfg!(feature = "reldb-postgres") {
                    "TimestampWithTimeZone"
                } else {
                    "Timestamp"
                }
            } else {
                return Err(Error::new(span, "not supported type!".to_string()));
            }
        }
        "FixedOffset" | "OffsetDateTime" => "TimestampWithTimeZone",
        "Value" | "Json" | _ => "Json",
    };
    let result = if let Some("Vec") = segments_type {
        if custom_ty != "var_binary" && custom_ty != "binary" {
            format!("array.{custom_ty}")
        } else {
            custom_ty.to_string()
        }
    } else {
        custom_ty.to_string()
    };
    Ok(result)
}

fn map_custom_type_to_sea_type(custom_column_type: &str, custom_len: Vec<u32>, span: Span) -> Result<TokenStream> {
    let mut type_split: Vec<_> = custom_column_type.split('.').collect();
    let first_type = if !type_split.is_empty() {
        type_split.remove(0)
    } else {
        return Err(Error::new(span, "column_type can't be empty!".to_string()));
    };
    let result = match first_type {
        "Char" | "char" => {
            if let Some(len) = custom_len.first() {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::Char(Some(#len)))
            } else {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::Char(None))
            }
        }
        "String" | "string" => {
            if let Some(len) = custom_len.first() {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::String(::tardis::db::sea_orm::sea_query::StringLen::N(#len)))
            } else {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::String(::tardis::db::sea_orm::sea_query::StringLen::None))
            }
        }
        "Text" | "text" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::Text)
        }
        "TinyInteger" | "tiny_integer" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::TinyInteger)
        }
        "SmallInteger" | "small_integer" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::SmallInteger)
        }
        "Integer" | "integer" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::Integer)
        }
        "BigInteger" | "big_integer" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::BigInteger)
        }
        "TinyUnsigned" | "tiny_unsigned" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::TinyUnsigned)
        }
        "SmallUnsigned" | "small_unsigned" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::SmallUnsigned)
        }
        "Unsigned" | "unsigned" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::Unsigned)
        }
        "BigUnsigned" | "big_unsigned" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::BigUnsigned)
        }
        "Float" | "float" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::Float)
        }
        "Double" | "double" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::Double)
        }
        "Decimal" | "decimal" => {
            if let (Some(precision), Some(scale)) = (custom_len.first(), custom_len.get(1)) {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::Decimal(Some((#precision,#scale))))
            } else {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::Decimal(None))
            }
        }
        "DateTime" | "date_time" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::DateTime)
        }
        "Timestamp" | "timestamp" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::Timestamp)
        }
        "TimestampWithTimeZone" | "timestamp_with_time_zone" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::TimestampWithTimeZone)
        }
        "Time" | "time" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::Time)
        }
        "Date" | "date" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::Date)
        }
        "Binary" | "binary" => {
            if let Some(len) = custom_len.first() {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::Binary(#len))
            } else {
                return Err(Error::new(span, "column_type:binary must have custom_len!".to_string()));
            }
        }
        "VarBinary" | "var_binary" => {
            if let Some(len) = custom_len.first() {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::VarBinary(::tardis::db::sea_orm::sea_query::StringLen::N(#len)))
            } else {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::VarBinary(::tardis::db::sea_orm::sea_query::StringLen::None))
            }
        }
        "Bit" | "bit" => {
            if let Some(len) = custom_len.first() {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::Bit(Some(#len)))
            } else {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::Bit(None))
            }
        }
        "VarBit" | "varbit" => {
            if let Some(len) = custom_len.first() {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::VarBit(#len))
            } else {
                return Err(Error::new(span, "column_type:varbit must have custom_len!".to_string()));
            }
        }
        "Boolean" | "boolean" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::Boolean)
        }
        "Money" | "money" => {
            if let (Some(precision), Some(scale)) = (custom_len.first(), custom_len.get(1)) {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::Money(Some((#precision,#scale))))
            } else {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::Money(None))
            }
        }
        "Json" | "json" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::Json)
        }
        "JsonBinary" | "json_binary" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::JsonBinary)
        }
        "UUID" | "Uuid" | "uuid" => {
            quote!(::tardis::db::sea_orm::sea_query::ColumnType::Uuid)
        }
        "Array" | "array" => {
            if cfg!(feature = "reldb-postgres") {
                let item_type = map_custom_type_to_sea_type(type_split.join(".").as_str(), custom_len, span)?;
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::Array(::std::sync::Arc::new(#item_type)))
            } else {
                return Err(Error::new(
                    span,
                    format!("column_type:{custom_column_type} only supported when the 'reldb-postgres' feature is enabled. "),
                ));
            }
        }
        "CIDR" | "Cidr" | "cidr" => {
            if cfg!(feature = "reldb-postgres") {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::Cidr)
            } else {
                return Err(Error::new(
                    span,
                    format!("column_type:{custom_column_type} only supported when the 'reldb-postgres' feature is enabled. "),
                ));
            }
        }
        "Inet" | "inet" => {
            if cfg!(feature = "reldb-postgres") {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::Inet)
            } else {
                return Err(Error::new(
                    span,
                    format!("column_type:{custom_column_type} only supported when the 'reldb-postgres' feature is enabled. "),
                ));
            }
        }
        "MacAddress" | "mac_address" => {
            if cfg!(feature = "reldb-postgres") {
                quote!(::tardis::db::sea_orm::sea_query::ColumnType::MacAddr)
            } else {
                return Err(Error::new(
                    span,
                    format!("column_type:{custom_column_type} only supported when the 'reldb-postgres' feature is enabled. "),
                ));
            }
        }
        _any => {
            //try `type(len)` type
            if _any.contains('(') && _any.contains(')') {
                let type_split: Vec<&str> = _any.split('(').collect();
                let len_split: Vec<&str> = type_split[1].split(')').collect();
                let mut custom_len = vec![];
                let parse_lens: Vec<_> = len_split[0]
                    .split(',')
                    .collect::<Vec<_>>()
                    .iter()
                    .map(|x| x.parse::<u32>().map_err(|_| Error::new(span, format!("column_type:{custom_column_type} is a not support custom type!"))))
                    .collect();
                for parse_len in parse_lens {
                    match parse_len {
                        Ok(len) => custom_len.push(len),
                        Err(_) => {
                            return Err(Error::new(
                                span,
                                format!("column_type:{custom_column_type} is a not support yet! The parentheses must contain numbers and not other characters"),
                            ));
                        }
                    }
                }

                map_custom_type_to_sea_type(type_split[0], custom_len, span)?
            } else {
                return Err(Error::new(span, format!("column_type:{custom_column_type} is a not support custom type!")));
            }
        }
    };
    Ok(result)
}
