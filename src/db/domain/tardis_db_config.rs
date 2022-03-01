use crate::basic::dto::TardisContext;
use crate::db::reldb_client::TardisActiveModel;
use crate::db::sea_orm::entity::prelude::*;
use crate::db::sea_orm::ActiveValue::Set;
use crate::db::sea_orm::{ActiveModelBehavior, DbBackend};
use crate::db::sea_query::{ColumnDef, Table, TableCreateStatement};
use crate::TardisFuns;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tardis_config")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(indexed)]
    pub k: String,
    #[sea_orm(column_type = "Text")]
    pub v: String,
    pub creator: String,
    pub updater: String,
    pub create_time: DateTime,
    pub update_time: DateTime,
}

impl TardisActiveModel for ActiveModel {
    fn fill_cxt(&mut self, _: &TardisContext, _: bool) {}

    fn create_table_statement(db_type: DbBackend) -> TableCreateStatement {
        match db_type {
            DbBackend::MySql => Table::create()
                .table(Entity.table_ref())
                .if_not_exists()
                .col(ColumnDef::new(Column::Id).not_null().string().primary_key())
                .col(ColumnDef::new(Column::K).not_null().string().unique_key())
                .col(ColumnDef::new(Column::V).not_null().text())
                .col(ColumnDef::new(Column::Creator).not_null().string())
                .col(ColumnDef::new(Column::Updater).not_null().string())
                .col(ColumnDef::new(Column::CreateTime).extra("DEFAULT CURRENT_TIMESTAMP".to_string()).date_time())
                .col(ColumnDef::new(Column::UpdateTime).extra("DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP".to_string()).date_time())
                .to_owned(),
            DbBackend::Postgres => {
                Table::create()
                    .table(Entity.table_ref())
                    .if_not_exists()
                    .col(ColumnDef::new(Column::Id).not_null().string().primary_key())
                    .col(ColumnDef::new(Column::K).not_null().string().unique_key())
                    .col(ColumnDef::new(Column::V).not_null().text())
                    .col(ColumnDef::new(Column::Creator).not_null().string())
                    .col(ColumnDef::new(Column::Updater).not_null().string())
                    .col(ColumnDef::new(Column::CreateTime).extra("DEFAULT CURRENT_TIMESTAMP".to_string()).date_time())
                    // TODO update time
                    .col(ColumnDef::new(Column::UpdateTime).extra("DEFAULT CURRENT_TIMESTAMP".to_string()).date_time())
                    .to_owned()
            }
            DbBackend::Sqlite =>
            // TODO
            {
                Table::create()
                    .table(Entity.table_ref())
                    .if_not_exists()
                    .col(ColumnDef::new(Column::Id).not_null().string().primary_key())
                    .col(ColumnDef::new(Column::K).not_null().string().unique_key())
                    .col(ColumnDef::new(Column::V).not_null().text())
                    .col(ColumnDef::new(Column::Creator).not_null().string())
                    .col(ColumnDef::new(Column::Updater).not_null().string())
                    .col(ColumnDef::new(Column::CreateTime).extra("DEFAULT CURRENT_TIMESTAMP".to_string()).date_time())
                    .col(ColumnDef::new(Column::UpdateTime).extra("DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP".to_string()).date_time())
                    .to_owned()
            }
        }
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

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
