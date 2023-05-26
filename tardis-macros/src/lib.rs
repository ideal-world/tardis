//! # Tardis-Macros
//!
//! Tardis-Macros is a macro support library for the Tardis framework, providing additional macros to simplify code generation for DTO (Data Transfer Object) with sea_orm.
//!
//! ## Main Macros
//!
//! - [`TardisCreateEntity`]: Generates code to create entities, combining `TardisCreateIndex` and `TardisCreateTable`.
//!
//! ## Features
//!
//! Tardis-Macros supports the following features, which enable the usage of specific macros:
//!
//! | Feature                        | Macro                   |
//! |--------------------------------|-------------------------|
//! | `reldb-postgres`               | `TardisCreateTable`     |
//! | `reldb-postgres`               | `TardisCreateIndex`     |
//! | `reldb-postgres`               | `TardisCreateEntity`    |
//! | `reldb-postgres`               | `TardisEmptyBehavior`   |
//! | `reldb-postgres`               | `TardisEmptyRelation`   |
//! | `reldb-mysql`                  | `TardisCreateTable`     |
//! | `reldb-mysql`                  | `TardisCreateIndex`     |
//! | `reldb-mysql`                  | `TardisCreateEntity`    |
//! | `reldb-mysql`                  | `TardisEmptyBehavior`   |
//! | `reldb-mysql`                  | `TardisEmptyRelation`   |
//!
//!
//! Please note that the availability of each macro depends on the enabled features. Make sure to enable the corresponding feature to use the desired macro.
//!
//! ## How to Use
//!
//! ### Best Practices
//!
//! Add the `TardisCreateEntity` macro to your struct definition, along with other necessary derive macros like `DeriveEntityModel`, `TardisEmptyBehavior`, and `TardisEmptyRelation`.
//!
//! Example usage:
//!
//! ```rust ignore
//! #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation)]
//! #[sea_orm(table_name = "examples")]
//! pub struct Model {
//!     #[sea_orm(primary_key, auto_increment = false)]
//!     pub id: String,
//!     #[index]
//!     #[fill_ctx(own_paths)]
//!     pub aaa: String,
//! }
//! ```
//! You also can refer to the example code and test cases for the best practices on using the Tardis-Macros library.
//!
//!
//! For more examples and detailed usage, please refer to the documentation of each specific macro.
//!
//! [TardisCreateEntity]

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// # TardisCreateTable
/// Generate table creation statement, compatible with `sea_orm`.
/// see [TardisActiveModel::create_table_statement](https://docs.rs/tardis/latest/tardis/db/reldb_client/trait.TardisActiveModel.html#method.create_table_statement). \
/// According to sea_orm automatically generates `tardis_create_table_statement(db: DbBackend)` method,
/// you can be directly called in the `TardisActiveModel::create_table_statement` method.  \
/// example see [macros_examples::example_for_derive_create_tabled]. \
///
/// ## sea_orm attribute
///
/// - `primary_key`: Specifies if the table has a primary key. (default: `false`)
/// - `nullable`: Specifies if the table columns are nullable. (default: `false`)
/// - `extra`: Additional information about the table. (optional)
/// - `custom_type`: Custom type for the table columns. (optional) See [`sea-query::tabled::column::ColumnDef`] .
/// - `custom_len`: Custom length for the table columns. (optional)
///
/// [`sea-query::tabled::column::ColumnDef`]: https://docs.rs/sea-query/latest/sea_query/table/struct.ColumnDef.html
#[cfg(any(feature = "reldb-postgres", feature = "reldb-mysql"))]
#[proc_macro_derive(TardisCreateTable, attributes(sea_orm))]
pub fn tardis_create_table(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, attrs, .. } = parse_macro_input!(input as DeriveInput);
    match tardis_create_table::create_table(ident, data, attrs) {
        Ok(stream) => stream.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// # TardisCreateIndex
/// Generate index creation statement, compatible with `sea_orm`.
/// see [create_index_statement](https://docs.rs/tardis/latest/tardis/db/reldb_client/trait.TardisActiveModel.html#method.create_index_statement). \
/// According to sea_orm automatically generates `tardis_create_index_statement()` method,
/// you can be directly called in the `TardisActiveModel::create_index_statement` method.  \
/// example see [macros_examples::example_for_derive_create_index].
///
/// ## index attribute
///
/// - `index_id`: ID of the index. (default: "index_id_1")
/// - `name`: Name of the index. (optional)
/// - `primary`: Specifies if the index is a primary index. (default: `false`)
/// - `unique`: Specifies if the index is a unique index. (default: `false`)
/// - `full_text`: Specifies if the index is a full-text index. (default: `false`)
/// - `if_not_exists`: Specifies if the index should be created if it doesn't exist. (default: `false`)
/// - `index_type`: Type of the index. See "Index Types" section for possible values. (optional)
///
/// ### index_id parameter
/// if you want generate different index statement, you must use `index_id` parameter to distinguish. \
/// Same index_id, if there are different variable assignments, only the first one will take effect. \
/// For example,the name of the generated statement is name1 instead of name2.
/// ```ignore
/// #[index(index_id="1",name="name1")]
/// name1:String,
/// #[index(index_id="1",name="name2")]
/// name2:String,
/// ```
///
/// ### Index Types
///
/// The `index_type` value needs to be one of the following:
///
/// - BTree
/// - FullText
/// - Gin
/// - Hash
/// - Custom
///
/// Example for custom:
/// ```ignore
/// #[derive(Clone, Debug, DeriveEntityModel, TardisCreateIndex)]
/// #[sea_orm(table_name = "examples")]
/// pub struct Model {
///     #[sea_orm(primary_key)]
///     pub id: String,
///     #[index(index_id = "index_id_1", index_type = "Custom(Test)")]
///     pub custom_index_col: String,
/// }
///
/// struct Test;
/// impl Iden for Test {
///     todo!()
/// }
/// ```
///
///
#[cfg(any(feature = "reldb-postgres", feature = "reldb-mysql"))]
#[proc_macro_derive(TardisCreateIndex, attributes(index))]
pub fn tardis_create_index(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, attrs, .. } = parse_macro_input!(input as DeriveInput);
    match tardis_create_index::create_index(ident, data, attrs) {
        Ok(stream) => stream.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// # TardisCreateEntity
/// The functionality of `TardisCreateEntity` is equivalent to `TardisCreateIndex` combined with `TardisCreateTable`.
/// Additionally, it introduces a new attribute called fill_ctx, and automatically implements `ActiveModelBehavior`. \
/// see [TardisCreateIndex] and [TardisCreateTable]
#[cfg(any(feature = "reldb-postgres", feature = "reldb-mysql"))]
#[proc_macro_derive(TardisCreateEntity, attributes(sea_orm, index, fill_ctx))]
pub fn tardis_create_entity(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input as DeriveInput);

    match tardis_create_entity::create_entity(ident, data) {
        Ok(stream) => stream.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
/// # TardisEmptyBehavior
/// Generates an empty implementation of `ActiveModelBehavior` for `ActiveModel`.
#[cfg(any(feature = "reldb-postgres", feature = "reldb-mysql"))]
#[proc_macro_derive(TardisEmptyBehavior, attributes(sea_orm, index, fill_ctx))]
pub fn tardis_empty_behavior(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input as DeriveInput);

    match tardis_empty_impl::create_empty_behavior(ident, data) {
        Ok(stream) => stream.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
/// #TardisEmptyRelation
/// Generates an empty `Relation`.
#[cfg(any(feature = "reldb-postgres", feature = "reldb-mysql"))]
#[proc_macro_derive(TardisEmptyRelation, attributes(sea_orm, index, fill_ctx))]
pub fn tardis_empty_relation(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input as DeriveInput);

    match tardis_empty_impl::create_empty_relation(ident, data) {
        Ok(stream) => stream.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

pub(crate) mod macro_helpers;
mod tardis_create_entity;
mod tardis_create_index;
mod tardis_create_table;
mod tardis_empty_impl;
