use tardis::basic::dto::TardisContext;
use tardis::db::reldb_client::TardisActiveModel;
use tardis::db::sea_orm;
use tardis::db::sea_orm::*;
use tardis::TardisFuns;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "test_account")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl Related<super::app::Entity> for super::account::Entity {
    fn to() -> RelationDef {
        super::app_account_rel::Relation::App.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::app_account_rel::Relation::Account.def().rev())
    }
}

impl TardisActiveModel for ActiveModel {
    fn fill_ctx(&mut self, _: &TardisContext, is_insert: bool) {
        if is_insert {
            self.id = Set(TardisFuns::field.nanoid());
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}
