use tardis::db::sea_orm::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "test_app_account_rel")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub app_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub account_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::app::Entity",
        from = "Column::AppId",
        to = "super::app::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    App,
    #[sea_orm(
        belongs_to = "super::account::Entity",
        from = "Column::AccountId",
        to = "super::account::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Account,
}

impl ActiveModelBehavior for ActiveModel {}
