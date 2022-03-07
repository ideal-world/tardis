use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use sea_orm::sea_query::TableCreateStatement;
use sea_orm::ActiveValue::Set;
use sea_orm::*;
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, EntityTrait, ExecResult, QueryTrait, Schema, Select, Statement};
use sea_query::{IndexCreateStatement, SelectStatement, UpdateStatement};
use sqlparser::ast;
use sqlparser::ast::{SetExpr, TableFactor};
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::{Parser, ParserError};
use url::Url;

use crate::basic::dto::TardisContext;
use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::db::domain::{tardis_db_config, tardis_db_del_record};
use crate::log::info;
use crate::{FrameworkConfig, TardisFuns};

pub struct TardisRelDBClient {
    con: DatabaseConnection,
}

impl TardisRelDBClient {
    pub async fn init_by_conf(conf: &FrameworkConfig) -> TardisResult<TardisRelDBClient> {
        TardisRelDBClient::init(
            &conf.db.url,
            conf.db.max_connections,
            conf.db.min_connections,
            conf.db.connect_timeout_sec,
            conf.db.idle_timeout_sec,
        )
        .await
    }

    pub async fn init(
        str_url: &str,
        max_connections: u32,
        min_connections: u32,
        connect_timeout_sec: Option<u64>,
        idle_timeout_sec: Option<u64>,
    ) -> TardisResult<TardisRelDBClient> {
        let url = Url::parse(str_url).unwrap_or_else(|_| panic!("[Tardis.RelDBClient] Invalid url {}", str_url));
        info!(
            "[Tardis.RelDBClient] Initializing, host:{}, port:{}, max_connections:{}",
            url.host_str().unwrap_or(""),
            url.port().unwrap_or(0),
            max_connections
        );
        let mut opt = ConnectOptions::new(str_url.to_string());
        opt.max_connections(max_connections).min_connections(min_connections).sqlx_logging(true);
        if let Some(connect_timeout_sec) = connect_timeout_sec {
            opt.connect_timeout(Duration::from_secs(connect_timeout_sec));
        }
        if let Some(idle_timeout_sec) = idle_timeout_sec {
            opt.idle_timeout(Duration::from_secs(idle_timeout_sec));
        }
        let con = Database::connect(opt).await?;
        info!(
            "[Tardis.RelDBClient] Initialized, host:{}, port:{}, max_connections:{}",
            url.host_str().unwrap_or(""),
            url.port().unwrap_or(0),
            min_connections
        );
        let client = TardisRelDBClient { con };
        client.init_basic_tables().await?;
        Ok(client)
    }

    pub fn backend(&self) -> DbBackend {
        self.con.get_database_backend()
    }

    pub fn conn(&self) -> TardisRelDBlConnection {
        TardisRelDBlConnection { conn: &self.con, tx: None }
    }

    async fn init_basic_tables(&self) -> TardisResult<()> {
        let tx = self.con.begin().await?;
        let config_create_table_statement = tardis_db_config::ActiveModel::create_table_statement(self.con.get_database_backend());
        TardisRelDBClient::create_table_inner(&config_create_table_statement, &tx).await?;
        let del_record_create_table_statement = tardis_db_del_record::ActiveModel::create_table_statement(self.con.get_database_backend());
        TardisRelDBClient::create_table_inner(&del_record_create_table_statement, &tx).await?;
        let del_record_create_index_statement = tardis_db_del_record::ActiveModel::create_index_statement();
        TardisRelDBClient::create_index_inner(&del_record_create_index_statement, &tx).await?;
        tx.commit().await?;
        Ok(())
    }

    /// TODO 不支持 not_null nullable  default_value  default_expr indexed, unique 等
    pub(self) async fn create_table_from_entity_inner<E, C>(entity: E, db: &C) -> TardisResult<()>
    where
        C: ConnectionTrait,
        E: EntityTrait,
    {
        let builder = db.get_database_backend();
        let schema = Schema::new(builder);
        let table_create_statement = &schema.create_table_from_entity(entity);
        TardisRelDBClient::create_table_inner(table_create_statement, db).await
    }

    pub(self) async fn create_table_inner<'a, C>(statement: &TableCreateStatement, db: &'a C) -> TardisResult<()>
    where
        C: ConnectionTrait,
    {
        let statement = db.get_database_backend().build(statement);
        match TardisRelDBClient::execute_inner(statement, db).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub(self) async fn create_index_inner<'a, C>(statements: &[IndexCreateStatement], db: &'a C) -> TardisResult<()>
    where
        C: ConnectionTrait,
    {
        for statement in statements {
            let statement = db.get_database_backend().build(statement);
            if let Err(e) = TardisRelDBClient::execute_inner(statement, db).await {
                return Err(e);
            }
        }
        Ok(())
    }

    pub(self) async fn execute_inner<C>(statement: Statement, db: &C) -> TardisResult<ExecResult>
    where
        C: ConnectionTrait,
    {
        let result = db.execute(statement).await;
        match result {
            Ok(ok) => TardisResult::Ok(ok),
            Err(err) => TardisResult::Err(TardisError::from(err)),
        }
    }

    pub(self) async fn get_dto_inner<'a, C, D>(select_statement: &SelectStatement, db: &'a C) -> TardisResult<Option<D>>
    where
        C: ConnectionTrait,
        D: FromQueryResult,
    {
        let result = D::find_by_statement(db.get_database_backend().build(select_statement)).one(db).await;
        match result {
            Ok(r) => TardisResult::Ok(r),
            Err(err) => TardisResult::Err(TardisError::from(err)),
        }
    }

    pub(self) async fn find_dtos_inner<'a, C, D>(select_statement: &SelectStatement, db: &'a C) -> TardisResult<Vec<D>>
    where
        C: ConnectionTrait,
        D: FromQueryResult,
    {
        let result = D::find_by_statement(db.get_database_backend().build(select_statement)).all(db).await;
        match result {
            Ok(r) => TardisResult::Ok(r),
            Err(err) => TardisResult::Err(TardisError::from(err)),
        }
    }

    pub(self) async fn paginate_dtos_inner<'a, C, D>(select_statement: &SelectStatement, page_number: u64, page_size: u64, db: &'a C) -> TardisResult<(Vec<D>, u64)>
    where
        C: ConnectionTrait,
        D: FromQueryResult,
    {
        let statement = db.get_database_backend().build(select_statement);
        let select_sql = format!("{} LIMIT {} , {}", statement.sql, (page_number - 1) * page_size, page_size);
        let query_statement = Statement {
            sql: select_sql,
            values: statement.values,
            db_backend: statement.db_backend,
        };
        let query_result = D::find_by_statement(query_statement).all(db).await?;
        let count_result = TardisRelDBClient::count_inner(select_statement, db).await?;
        Ok((query_result, count_result))
    }

    pub(self) async fn count_inner<'a, C>(select_statement: &SelectStatement, db: &'a C) -> TardisResult<u64>
    where
        C: ConnectionTrait,
    {
        let statement = db.get_database_backend().build(select_statement);
        let count_sql = format!(
            "SELECT COUNT(1) AS count FROM ( {} ) _{}",
            statement.sql,
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
        );
        let count_statement = Statement {
            sql: count_sql.clone(),
            values: statement.values,
            db_backend: statement.db_backend,
        };
        let count_result = CountResp::find_by_statement(count_statement).one(db).await?;
        match count_result {
            Some(r) => TardisResult::Ok(r.count as u64),
            None => TardisResult::Err(TardisError::InternalError(format!(
                "[Tardis.RelDBClient] No results found for count query by {}",
                count_sql
            ))),
        }
    }

    pub(self) async fn insert_one_inner<'a, T, C>(mut model: T, db: &'a C, cxt: &TardisContext) -> TardisResult<InsertResult<T>>
    where
        C: ConnectionTrait,
        T: TardisActiveModel,
    {
        model.fill_cxt(cxt, true);
        let result = EntityTrait::insert(model).exec(db).await?;
        Ok(result)
    }

    pub(self) async fn insert_many_inner<'a, T, C>(mut models: Vec<T>, db: &'a C, cxt: &TardisContext) -> TardisResult<()>
    where
        C: ConnectionTrait,
        T: TardisActiveModel,
    {
        models.iter_mut().for_each(|m| m.fill_cxt(cxt, true));
        EntityTrait::insert_many(models).exec(db).await?;
        Ok(())
    }

    pub(self) async fn update_one_inner<'a, T, C>(mut model: T, db: &'a C, cxt: &TardisContext) -> TardisResult<()>
    where
        C: ConnectionTrait,
        T: TardisActiveModel,
    {
        model.fill_cxt(cxt, false);
        let update = EntityTrait::update(model);
        TardisRelDBClient::execute_inner(db.get_database_backend().build(update.as_query()), db).await?;
        Ok(())
    }

    pub(self) async fn update_many_inner<'a, C>(update_statement: &UpdateStatement, db: &'a C) -> TardisResult<()>
    where
        C: ConnectionTrait,
    {
        TardisRelDBClient::execute_inner(db.get_database_backend().build(update_statement), db).await?;
        Ok(())
    }

    pub(self) async fn soft_delete_inner<'a, E, C>(select: Select<E>, delete_user: &str, db: &'a C) -> TardisResult<u64>
    where
        C: ConnectionTrait,
        E: EntityTrait,
    {
        select.soft_delete(delete_user, db).await
    }

    pub(self) async fn soft_delete_custom_inner<'a, E, C>(select: Select<E>, custom_pk_field: &str, delete_user: &str, db: &'a C) -> TardisResult<u64>
    where
        C: ConnectionTrait,
        E: EntityTrait,
    {
        select.soft_delete_custom(custom_pk_field, delete_user, db).await
    }
}

pub struct TardisRelDBlConnection<'a> {
    conn: &'a DatabaseConnection,
    tx: Option<DatabaseTransaction>,
}

impl<'a> TardisRelDBlConnection<'a> {
    pub fn raw_conn(&self) -> &DatabaseConnection {
        self.conn
    }

    pub fn raw_tx(&self) -> TardisResult<&DatabaseTransaction> {
        if let Some(tx) = &self.tx {
            Ok(tx)
        } else {
            Err(TardisError::NotFound("[Tardis.RelDBClient] The current connection  has no transactions".to_string()))
        }
    }

    pub async fn begin(&mut self) -> TardisResult<()> {
        self.tx = Some(self.conn.begin().await?);
        Ok(())
    }

    pub async fn commit(self) -> TardisResult<()> {
        if let Some(tx) = self.tx {
            tx.commit().await?;
        }
        Ok(())
    }

    pub async fn rollback(self) -> TardisResult<()> {
        if let Some(tx) = self.tx {
            tx.rollback().await?;
        }
        Ok(())
    }

    pub async fn create_table_from_entity<E>(&self, entity: E) -> TardisResult<()>
    where
        E: EntityTrait,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::create_table_from_entity_inner(entity, tx).await
        } else {
            TardisRelDBClient::create_table_from_entity_inner(entity, self.conn).await
        }
    }

    pub async fn create_table(&self, statement: &TableCreateStatement) -> TardisResult<()> {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::create_table_inner(statement, tx).await
        } else {
            TardisRelDBClient::create_table_inner(statement, self.conn).await
        }
    }

    pub async fn create_index(&self, statements: &[IndexCreateStatement]) -> TardisResult<()> {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::create_index_inner(statements, tx).await
        } else {
            TardisRelDBClient::create_index_inner(statements, self.conn).await
        }
    }

    pub async fn get_dto<D>(&self, select_statement: &SelectStatement) -> TardisResult<Option<D>>
    where
        D: FromQueryResult,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::get_dto_inner(select_statement, tx).await
        } else {
            TardisRelDBClient::get_dto_inner(select_statement, self.conn).await
        }
    }

    pub async fn find_dtos<D>(&self, select_statement: &SelectStatement) -> TardisResult<Vec<D>>
    where
        D: FromQueryResult,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::find_dtos_inner(select_statement, tx).await
        } else {
            TardisRelDBClient::find_dtos_inner(select_statement, self.conn).await
        }
    }

    pub async fn paginate_dtos<D>(&self, select_statement: &SelectStatement, page_number: u64, page_size: u64) -> TardisResult<(Vec<D>, u64)>
    where
        D: FromQueryResult,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::paginate_dtos_inner(select_statement, page_number, page_size, tx).await
        } else {
            TardisRelDBClient::paginate_dtos_inner(select_statement, page_number, page_size, self.conn).await
        }
    }

    pub async fn count(&self, select_statement: &SelectStatement) -> TardisResult<u64> {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::count_inner(select_statement, tx).await
        } else {
            TardisRelDBClient::count_inner(select_statement, self.conn).await
        }
    }

    pub async fn execute(&self, statement: Statement) -> TardisResult<ExecResult> {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::execute_inner(statement, tx).await
        } else {
            TardisRelDBClient::execute_inner(statement, self.conn).await
        }
    }

    pub async fn insert_one<T>(&self, model: T, cxt: &TardisContext) -> TardisResult<InsertResult<T>>
    where
        T: TardisActiveModel,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::insert_one_inner(model, tx, cxt).await
        } else {
            TardisRelDBClient::insert_one_inner(model, self.conn, cxt).await
        }
    }

    pub async fn insert_many<T>(&self, models: Vec<T>, cxt: &TardisContext) -> TardisResult<()>
    where
        T: TardisActiveModel,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::insert_many_inner(models, tx, cxt).await
        } else {
            TardisRelDBClient::insert_many_inner(models, self.conn, cxt).await
        }
    }

    pub async fn update_one<T>(&self, model: T, cxt: &TardisContext) -> TardisResult<()>
    where
        T: TardisActiveModel,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::update_one_inner(model, tx, cxt).await
        } else {
            TardisRelDBClient::update_one_inner(model, self.conn, cxt).await
        }
    }

    pub async fn update_many<T>(&self, update_statement: &UpdateStatement) -> TardisResult<()> {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::update_many_inner(update_statement, tx).await
        } else {
            TardisRelDBClient::update_many_inner(update_statement, self.conn).await
        }
    }

    pub async fn soft_delete<E>(&self, select: Select<E>, delete_user: &str) -> TardisResult<u64>
    where
        E: EntityTrait,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::soft_delete_inner(select, delete_user, tx).await
        } else {
            TardisRelDBClient::soft_delete_inner(select, delete_user, self.conn).await
        }
    }

    pub async fn soft_delete_custom<E>(&self, select: Select<E>, custom_pk_field: &str, delete_user: &str) -> TardisResult<u64>
    where
        E: EntityTrait,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::soft_delete_custom_inner(select, custom_pk_field, delete_user, tx).await
        } else {
            TardisRelDBClient::soft_delete_custom_inner(select, custom_pk_field, delete_user, self.conn).await
        }
    }
}

#[async_trait]
pub trait TardisSeaORMExtend {
    async fn soft_delete<C>(self, delete_user: &str, db: &C) -> TardisResult<u64>
    where
        C: ConnectionTrait;

    async fn soft_delete_custom<C>(self, custom_pk_field: &str, delete_user: &str, db: &C) -> TardisResult<u64>
    where
        C: ConnectionTrait;
}

#[async_trait]
impl<E> TardisSeaORMExtend for Select<E>
where
    E: EntityTrait,
{
    async fn soft_delete<C>(self, delete_user: &str, db: &C) -> TardisResult<u64>
    where
        C: ConnectionTrait,
    {
        self.soft_delete_custom("id", delete_user, db).await
    }

    async fn soft_delete_custom<C>(self, custom_pk_field: &str, delete_user: &str, db: &C) -> TardisResult<u64>
    where
        C: ConnectionTrait,
    {
        let db_backend: DbBackend = db.get_database_backend();

        let sql = self.build(db_backend).sql.replace("?", "''");
        let ast = match Parser::parse_sql(&MySqlDialect {}, &sql)?.pop() {
            Some(ast) => ast,
            None => return Err(TardisError::BadRequest("[Tardis.RelDBClient] Sql parsing error, no valid Statement found".to_string())),
        };
        let mut table_name = String::new();
        if let ast::Statement::Query(query) = ast {
            if let SetExpr::Select(select) = (*query).body {
                if let TableFactor::Table { name, .. } = &select.from[0].relation {
                    table_name = name.0[0].value.clone();
                }
            }
        }
        if table_name.is_empty() {
            return TardisResult::Err(TardisError::Conflict(
                "sql parsing error, the name of the table \
            to be soft deleted was not found"
                    .to_string(),
            ));
        }

        let mut ids: Vec<Value> = Vec::new();

        let rows = self.into_json().all(db).await?;
        for row in rows {
            let id = row[custom_pk_field].clone();
            let json = TardisFuns::json.obj_to_string(&row)?;
            if id.is_string() {
                ids.push(
                    id.as_str()
                        .as_ref()
                        .unwrap_or_else(|| panic!("[Tardis.RelDBClient] The primary key [{}] in a soft delete operation is not a character type", id))
                        .to_string()
                        .into(),
                );
            } else {
                ids.push(id.as_u64().unwrap_or_else(|| panic!("[Tardis.RelDBClient] The primary key [{}] in a soft delete operation is not a number type", id)).into());
            }
            tardis_db_del_record::ActiveModel {
                entity_name: Set(table_name.to_string()),
                record_id: Set(id.to_string()),
                content: Set(json),
                creator: Set(delete_user.to_string()),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
        let delete_num = ids.len();
        if delete_num == 0 {
            return Ok(0);
        }
        let statement = Statement::from_sql_and_values(
            db_backend,
            match db_backend {
                DbBackend::Postgres => format!("DELETE FROM {} WHERE id in ($1)", table_name),
                _ => format!("DELETE FROM {} WHERE id in (?)", table_name),
            }
            .as_str(),
            ids,
        );
        let result = db.execute(statement).await;
        match result {
            Ok(_) => TardisResult::Ok(delete_num as u64),
            Err(err) => TardisResult::Err(TardisError::from(err)),
        }
    }
}

#[async_trait]
pub trait TardisActiveModel: ActiveModelBehavior {
    fn fill_cxt(&mut self, cxt: &TardisContext, is_insert: bool);

    fn create_table_statement(_: DbBackend) -> TableCreateStatement {
        TableCreateStatement::new()
    }

    fn create_index_statement() -> Vec<IndexCreateStatement> {
        vec![IndexCreateStatement::new()]
    }
}

#[derive(Debug, FromQueryResult)]
struct CountResp {
    count: i64,
}

impl From<DbErr> for TardisError {
    fn from(error: DbErr) -> Self {
        TardisError::Box(Box::new(error))
    }
}

impl From<ParserError> for TardisError {
    fn from(error: ParserError) -> Self {
        TardisError::Box(Box::new(error))
    }
}
