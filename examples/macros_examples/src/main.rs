use tardis::basic::dto::TardisContext;
use tardis::db::reldb_client::TardisActiveModel;
use tardis::db::sea_orm;
use tardis::db::sea_orm::sea_query::TableCreateStatement;
use tardis::db::sea_orm::*;
use tardis::DeriveCreateTable;

fn main() {}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, DeriveCreateTable)]
#[sea_orm(table_name = "examples")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub number: i64,
    pub can_be_null: Option<String>,
    pub _bool: bool,
    pub own_paths: String,
}

impl TardisActiveModel for ActiveModel {
    fn fill_ctx(&mut self, ctx: &TardisContext, is_insert: bool) {
        if is_insert {
            self.own_paths = Set(ctx.own_paths.to_string());
        }
    }
    ///调用macros自动生成的方法 create_table_statement
    fn create_table_statement(db: DbBackend) -> TableCreateStatement {
        create_table_statement(db)
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
