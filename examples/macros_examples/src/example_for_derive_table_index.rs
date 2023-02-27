use tardis::basic::dto::TardisContext;
use tardis::chrono::Utc;
use tardis::db::reldb_client::TardisActiveModel;
use tardis::db::sea_orm;
use tardis::db::sea_orm::sea_query::{IndexCreateStatement, TableCreateStatement};
use tardis::db::sea_orm::*;
use tardis::{chrono, DeriveCreateTable};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, DeriveTableIndex)]
#[sea_orm(table_name = "examples")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(column_name = "number8")]
    pub number_i8_for_test: i8,
    pub number16: i16,
    pub number32: i32,
    pub number64: i64,
    // pub number_f32: f32,
    // pub number_f64: f64,
    pub number_u8: Vec<u8>,
    pub can_bool: bool,
    pub can_be_null: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub be_text: String,
    #[sea_orm(extra = "DEFAULT CURRENT_TIMESTAMP")]
    pub create_time: chrono::DateTime<Utc>,
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
