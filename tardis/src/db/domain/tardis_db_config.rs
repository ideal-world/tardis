use chrono::Utc;
use log::trace;
use sea_orm::*;

use crate::basic::dto::TardisContext;
use crate::db::domain::tardis_db_config;
use crate::db::reldb_client::{TardisActiveModel, TardisRelDBlConnection};
use crate::db::sea_orm::sea_query::{ColumnDef, Table, TableCreateStatement};
use crate::db::sea_orm::ActiveValue::Set;
use crate::db::sea_orm::{ActiveModelBehavior, DbBackend};
use crate::TardisResult;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "tardis_config")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub k: String,
    #[sea_orm(column_type = "Text")]
    pub v: String,
    pub creator: String,
    pub updater: String,
    pub create_time: chrono::DateTime<Utc>,
    pub update_time: chrono::DateTime<Utc>,
}

impl TardisActiveModel for ActiveModel {
    fn fill_ctx(&mut self, _: &TardisContext, _: bool) {}

    fn create_table_statement(db_type: DbBackend) -> TableCreateStatement {
        match db_type {
            DbBackend::MySql => Table::create()
                .table(Entity.table_ref())
                .if_not_exists()
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_0900_as_cs")
                .col(ColumnDef::new(Column::K).not_null().string().primary_key())
                .col(ColumnDef::new(Column::V).not_null().text())
                .col(ColumnDef::new(Column::Creator).not_null().string())
                .col(ColumnDef::new(Column::Updater).not_null().string())
                .col(ColumnDef::new(Column::CreateTime).extra("DEFAULT CURRENT_TIMESTAMP".to_string()).timestamp())
                .col(ColumnDef::new(Column::UpdateTime).extra("DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP".to_string()).timestamp())
                .to_owned(),
            DbBackend::Postgres => Table::create()
                .table(Entity.table_ref())
                .if_not_exists()
                .col(ColumnDef::new(Column::K).not_null().string().primary_key())
                .col(ColumnDef::new(Column::V).not_null().text())
                .col(ColumnDef::new(Column::Creator).not_null().string())
                .col(ColumnDef::new(Column::Updater).not_null().string())
                .col(ColumnDef::new(Column::CreateTime).extra("DEFAULT CURRENT_TIMESTAMP".to_string()).timestamp_with_time_zone())
                // TODO update time
                .col(ColumnDef::new(Column::UpdateTime).extra("DEFAULT CURRENT_TIMESTAMP".to_string()).timestamp_with_time_zone())
                .to_owned(),
            DbBackend::Sqlite => Table::create()
                .table(Entity.table_ref())
                .if_not_exists()
                .col(ColumnDef::new(Column::K).not_null().string().primary_key())
                .col(ColumnDef::new(Column::V).not_null().text())
                .col(ColumnDef::new(Column::Creator).not_null().string())
                .col(ColumnDef::new(Column::Updater).not_null().string())
                .col(ColumnDef::new(Column::CreateTime).extra("DEFAULT CURRENT_TIMESTAMP".to_string()).timestamp())
                .col(ColumnDef::new(Column::UpdateTime).extra("DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP".to_string()).timestamp())
                .to_owned(),
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

pub struct TardisDataDict;

impl TardisDataDict {
    pub async fn get(&self, key: &str, db: &TardisRelDBlConnection) -> TardisResult<Option<TardisDictResp>> {
        let model = tardis_db_config::Entity::find_by_id(key.to_string()).into_model::<TardisDictResp>();
        let result = if db.has_tx() { model.one(db.raw_tx()?).await? } else { model.one(db.raw_conn()).await? };
        Ok(result)
    }

    pub async fn find_like(&self, key: &str, db: &TardisRelDBlConnection) -> TardisResult<Vec<TardisDictResp>> {
        let model = tardis_db_config::Entity::find().filter(tardis_db_config::Column::K.like(format!("{key}%").as_str())).into_model::<TardisDictResp>();
        let result = if db.has_tx() { model.all(db.raw_tx()?).await? } else { model.all(db.raw_conn()).await? };
        Ok(result)
    }

    pub async fn find_all(&self, db: &TardisRelDBlConnection) -> TardisResult<Vec<TardisDictResp>> {
        let model = tardis_db_config::Entity::find().into_model::<TardisDictResp>();
        let result = if db.has_tx() { model.all(db.raw_tx()?).await? } else { model.all(db.raw_conn()).await? };
        Ok(result)
    }

    pub async fn add(&self, key: &str, value: &str, creator: &str, db: &TardisRelDBlConnection) -> TardisResult<()> {
        trace!("[Tardis.RelDBClient] [db_config] add key: {}, value: {}", key, value);
        let model = tardis_db_config::ActiveModel {
            k: Set(key.to_string()),
            v: Set(value.to_string()),
            creator: Set(creator.to_string()),
            updater: Set(creator.to_string()),
            update_time: Set(Utc::now()),
            ..Default::default()
        };
        if db.has_tx() {
            model.insert(db.raw_tx()?).await?;
        } else {
            model.insert(db.raw_conn()).await?;
        }
        Ok(())
    }

    pub async fn update(&self, key: &str, value: &str, updater: &str, db: &TardisRelDBlConnection) -> TardisResult<()> {
        trace!("[Tardis.RelDBClient] [db_config] update key: {}, value: {}", key, value);
        let model = tardis_db_config::ActiveModel {
            k: Set(key.to_string()),
            v: Set(value.to_string()),
            updater: Set(updater.to_string()),
            ..Default::default()
        };
        if db.has_tx() {
            model.update(db.raw_tx()?).await?;
        } else {
            model.update(db.raw_conn()).await?;
        }
        Ok(())
    }

    pub async fn delete(&self, key: &str, db: &TardisRelDBlConnection) -> TardisResult<()> {
        trace!("[Tardis.RelDBClient] [db_config] delete key: {}", key);
        let model = tardis_db_config::Entity::delete_many().filter(tardis_db_config::Column::K.eq(key.to_string()));
        if db.has_tx() {
            model.exec(db.raw_tx()?).await?;
        } else {
            model.exec(db.raw_conn()).await?;
        }
        Ok(())
    }
}

#[derive(Debug, FromQueryResult)]
pub struct TardisDictResp {
    pub k: String,
    pub v: String,
    pub creator: String,
    pub updater: String,
    pub create_time: chrono::DateTime<Utc>,
    pub update_time: chrono::DateTime<Utc>,
}
