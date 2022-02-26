use tardis::db::sea_orm::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "todos")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub description: String,
    pub done: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
