use std::fmt::Write;
use tardis::basic::dto::TardisContext;
use tardis::db::reldb_client::TardisActiveModel;
use tardis::db::sea_orm;
use tardis::db::sea_orm::sea_query::IndexCreateStatement;
use tardis::db::sea_orm::*;
use tardis::TardisCreateIndex;

// run `cargo expand example_for_derive_table_index > derive_create_index_expand.rs` \
// to see automatically generated method tardis_create_index_statement()
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, TardisCreateIndex)]
#[sea_orm(table_name = "examples")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[index(repeat(index_id = "index_id_3"), repeat(index_id = "index_id_4"))]
    pub number32: i32,
    #[index(unique)]
    pub number64: i64,
    #[index(full_text, index_id = "index_id_3")]
    pub can_be_null: Option<String>,
    #[index(name = "rename_example")]
    pub be_text: String,
    #[index(index_id = "index_id_2", index_type = "Custom(GiST)", full_text)]
    pub own_paths: String,
}

impl TardisActiveModel for ActiveModel {
    fn fill_ctx(&mut self, ctx: &TardisContext, is_insert: bool) {
        if is_insert {
            self.own_paths = Set(ctx.own_paths.to_string());
        }
    }

    fn create_index_statement() -> Vec<IndexCreateStatement> {
        tardis_create_index_statement()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

struct GiST;

impl Iden for GiST {
    fn unquoted(&self, s: &mut dyn Write) {
        s.write_str("GiST").expect("TODO: panic message");
    }
}
