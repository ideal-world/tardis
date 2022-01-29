use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{ColumnDef, Table, TableCreateStatement};
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelBehavior, DbBackend};

use crate::db::domain::tardis_db_del_record;
use crate::TardisFuns;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tardis_del_record")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(indexed)]
    pub entity_name: String,
    #[sea_orm(indexed)]
    pub record_id: String,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    pub creator: String,
    pub create_time: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            id: Set(TardisFuns::field.uuid_str()),
            ..ActiveModelTrait::default()
        }
    }
}

pub fn create_table_statement(_: DbBackend) -> TableCreateStatement {
    Table::create()
        .table(tardis_db_del_record::Entity.table_ref())
        .if_not_exists()
        .col(ColumnDef::new(tardis_db_del_record::Column::Id).not_null().string().primary_key())
        .col(ColumnDef::new(tardis_db_del_record::Column::EntityName).not_null().string())
        .col(ColumnDef::new(tardis_db_del_record::Column::RecordId).not_null().string())
        .col(ColumnDef::new(tardis_db_del_record::Column::Content).not_null().text())
        .col(ColumnDef::new(tardis_db_del_record::Column::Creator).not_null().string())
        .col(ColumnDef::new(tardis_db_del_record::Column::CreateTime).extra("DEFAULT CURRENT_TIMESTAMP".to_string()).date_time())
        .to_owned()
}
