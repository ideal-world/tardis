use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_attribute]
pub fn struct_copy(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let _ = input;
    unimplemented!()
}

/// 生成建表语句,兼容sea_orm \
/// see [tardis::db::relbd_client::TardisActiveModel::create_table_statement] \
/// 根据sea_orm自动生成tardis_create_table_statement方法，
/// 可以在 TardisActiveModel::create_table_statement 方法中直接调用 \
/// 示例 see [macros_examples::example_for_derive_create_tabled]
#[proc_macro_derive(DeriveCreateTable, attributes(sea_orm))]
#[allow(non_snake_case)]
pub fn TardisCreateTable(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, attrs, .. } = parse_macro_input!(input as DeriveInput);
    match tardis_create_table::create_table(ident, data, attrs) {
        Ok(stream) => stream.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

//todo 扩展sea_orm 自动生成创建索引语句

pub(crate) mod macro_helpers;
mod tardis_create_table;
