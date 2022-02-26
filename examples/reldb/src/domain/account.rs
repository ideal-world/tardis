use tardis::db::sea_orm::*;
use tardis::TardisFuns;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
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

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            id: Set(TardisFuns::field.uuid_str()),
            ..ActiveModelTrait::default()
        }
    }
}
