use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_attribute]
pub fn struct_copy(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let _ = input;
    unimplemented!()
}

/// Generate table creation statement, compatible with sea_orm \
/// see [tardis::db::relbd_client::TardisActiveModel::create_table_statement] \
/// According to sea_orm automatically generates `tardis_create_table_Statement` method,
/// you can be directly called in the `TardisActiveModel::create_table_statement` method  \
///  see [macros_examples::example_for_derive_create_tabled]
#[proc_macro_derive(DeriveCreateTable, attributes(sea_orm))]
#[allow(non_snake_case)]
pub fn TardisCreateTable(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, attrs, .. } = parse_macro_input!(input as DeriveInput);
    match tardis_create_table::create_table(ident, data, attrs) {
        Ok(stream) => stream.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

///Generate index creation statement, compatible with sea_orm \
/// see [tardis::db::relbd_client::TardisActiveModel::create_index_statement] \
/// According to sea_orm automatically generates `tardis_create_index_Statement` method,
/// you can be directly called in the `TardisActiveModel::create_index_statement` method  \
///  see [macros_examples::example_for_derive_create_index]
#[proc_macro_derive(DeriveTableIndex, attributes(index))]
#[allow(non_snake_case)]
pub fn TardisCreateIndex(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, attrs, .. } = parse_macro_input!(input as DeriveInput);
    match tardis_create_index::create_index(ident, data, attrs) {
        Ok(stream) => stream.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

pub(crate) mod macro_helpers;
mod tardis_create_index;
mod tardis_create_table;
