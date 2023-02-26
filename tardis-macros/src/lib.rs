use proc_macro::TokenStream;
use syn::{parse_macro_input, AttributeArgs, DeriveInput, ItemImpl};

#[proc_macro_attribute]
pub fn struct_copy(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let _ = input;
    unimplemented!()
}

/// 生成建表语句
/// see [tardis::db::relbd_client::TardisActiveModel::create_table_statement]
#[proc_macro_derive(DeriveCreateTable, attributes(sea_orm))]
#[allow(non_snake_case)]
pub fn TardisCreateTable(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, attrs, .. } = parse_macro_input!(input as DeriveInput);
    match tardis_create_table::create_table(ident, data, attrs) {
        Ok(stream) => stream.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

pub(crate) mod macro_helpers;
mod tardis_create_table;
