use std::sync::Arc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use sea_orm::sea_query::TableCreateStatement;
use sea_orm::sea_query::{IndexCreateStatement, SelectStatement, UpdateStatement};
use sea_orm::ActiveValue::Set;
use sea_orm::*;
use sqlparser::ast;
use sqlparser::ast::{SetExpr, TableFactor};
use sqlparser::dialect::{MySqlDialect, PostgreSqlDialect, SQLiteDialect};
use sqlparser::parser::{Parser, ParserError};
use sqlx::Executor;
use tracing::{error, info, instrument, trace};
use url::Url;

use crate::basic::dto::TardisContext;
use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::config::config_dto::component::db::CompatibleType;
use crate::config::config_dto::component::db::DBModuleConfig;
use crate::db::domain::{tardis_db_config, tardis_db_del_record};
use crate::serde::{Deserialize, Serialize};
use crate::utils::initializer::InitBy;
use crate::TardisFuns;

/// Relational database handle / 关系型数据库操作
///
/// Encapsulates common operations of MySQL and PostgreSQL. Two styles of operations are provided:
///
/// 1. Wrapper based on `sea-orm` for simple relational operations, see `examples/reldb` for examples
/// 1. Wrapper based on `sea-query` for complex, custom processing operations, see `https://github.com/ideal-world/bios` for examples.
///
/// 封装了对MySQL、PostgreSQL的常用操作.提供了两种风格的操作:
///
/// 1. 基于 `sea-orm` 的封装，适用于简单的关系处理操作，操作示例见 `examples/reldb`
/// 1. 基于 `sea-query` 的封装，适用复杂的、自定义的处理操作，操作示例见 `https://github.com/ideal-world/bios`
///
/// # Steps to use / 使用步骤
///
/// 1. Create the database configuration / 创建数据库配置, @see [DBConfig](crate::basic::config::DBConfig)
///
/// 2.  Create the `domain` object / 创建 `domain` 对象, E.g:
/// ```ignore
/// use sea_orm::{DeriveRelation, EnumIter};
/// mod tardis_db_config{
///     use tardis::db::sea_orm::*;
///     use tardis::db::sea_orm::sea_query::*;
///     use tardis::basic::dto::TardisContext;
/// use tardis::db::domain::tardis_db_config;
///     use tardis::db::reldb_client::TardisActiveModel;
///     use tardis::TardisFuns;
///     // Reference DeriveEntityModel macro for skeleton code generation (sea-orm function)
///     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
///     // Define table name (sea-orm function)
///     #[sea_orm(table_name = "tardis_config")]
///     pub struct Model {
///     // Define primary key (sea-orm function)
///         #[sea_orm(primary_key, auto_increment = false)]
///         pub id: String,
///         #[sea_orm(indexed)]
///         pub k: String,
///         #[sea_orm(column_type = "Text")]
///         pub v: String,
///         pub creator: String,
///         pub updater: String,
///     }
///    
///     // Define extended information (Tardis function)
///     impl TardisActiveModel for ActiveModel {
///         fn fill_ctx(&mut self, ctx: &TardisContext, is_insert: bool) {}
///    
///         fn create_table_statement(db_type: DbBackend) -> TableCreateStatement {
///              Table::create()
///                 .table(tardis_db_config::Entity.table_ref())
///                 .if_not_exists()
///                 .col(ColumnDef::new(tardis_db_config::Column::Id).not_null().string().primary_key())
///                 .col(ColumnDef::new(tardis_db_config::Column::K).not_null().string().unique_key())
///                 .col(ColumnDef::new(tardis_db_config::Column::V).not_null().text())
///                 .col(ColumnDef::new(tardis_db_config::Column::Creator).not_null().string())
///                 .col(ColumnDef::new(tardis_db_config::Column::Updater).not_null().string())
///                 .to_owned()
///         }
///     }
///    
///     impl ActiveModelBehavior for ActiveModel {
///         fn new() -> Self {
///             Self {
///                 id: Set(TardisFuns::field.nanoid()),
///                 ..ActiveModelTrait::default()
///             }
///         }
///     }
/// }
///
/// // Define association relationships (sea-orm function)
/// #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// pub enum Relation {}
/// ```
///
/// 3. Create TardisContext / 创建TardisContext [TardisContext]
///
/// 4. Use `TardisRelDBClient` to operate database / 使用 `TardisRelDBClient` 操作数据库, E.g:
/// ```ignore
/// use std::process::id;
/// use tardis::basic::dto::TardisContext;
/// use tardis::db::domain::tardis_db_config;
/// use tardis::db::reldb_client::IdResp;
/// use tardis::db::sea_orm::sea_query::*;
/// use tardis::db::sea_orm::*;
/// use tardis::TardisFuns;
/// let ctx = TardisContext{
/// // Define Context
///   ..Default::default()
/// };
/// let conn = TardisFuns::reldb().conn();
/// conn.insert_one(tardis_db_config::ActiveModel {
///     k: Set("ke".to_string()),
///     v: Set("ve".to_string()),
///     ..Default::default()
/// },&ctx).await.unwrap();
///
/// conn.paginate_dtos::<IdResp>(&Query::select()
///     .column(tardis_db_config::Column::Id)
///     .from(tardis_db_config::Entity),
///     1,10
/// ).await.unwrap();
/// ```
pub struct TardisRelDBClient {
    con: Arc<DatabaseConnection>,
    compatible_type: CompatibleType,
}

#[async_trait::async_trait]
impl InitBy<DBModuleConfig> for TardisRelDBClient {
    async fn init_by(config: &DBModuleConfig) -> TardisResult<Self> {
        Self::init(config).await
    }
}

impl TardisRelDBClient {
    /// Initialize configuration / 初始化配置
    pub async fn init(
        DBModuleConfig {
            url: str_url,
            max_connections,
            min_connections,
            connect_timeout_sec,
            idle_timeout_sec,
            compatible_type,
        }: &DBModuleConfig,
    ) -> TardisResult<TardisRelDBClient> {
        use crate::utils::redact::Redact;
        let url = Url::parse(str_url).map_err(|_| TardisError::format_error(&format!("[Tardis.RelDBClient] Invalid url {str_url}"), "406-tardis-reldb-url-error"))?;
        info!(
            "[Tardis.RelDBClient] Initializing, host:{}, port:{}, max_connections:{}",
            url.host_str().unwrap_or(""),
            url.port().unwrap_or(0),
            max_connections
        );
        let mut opt = ConnectOptions::new(url.to_string());
        opt.max_connections(*max_connections).min_connections(*min_connections).sqlx_logging(true);
        if let Some(connect_timeout_sec) = connect_timeout_sec {
            opt.connect_timeout(Duration::from_secs(*connect_timeout_sec));
        }
        if let Some(idle_timeout_sec) = idle_timeout_sec {
            opt.idle_timeout(Duration::from_secs(*idle_timeout_sec));
        }
        let con = if let Some(timezone) = url.query_pairs().find(|x| x.0.to_lowercase() == "timezone").map(|x| x.1.to_string()) {
            match url.scheme().to_lowercase().as_str() {
                #[cfg(feature = "reldb-mysql")]
                "mysql" => {
                    let mut raw_opt = opt.get_url().parse::<sqlx::mysql::MySqlConnectOptions>().map_err(|error| DbErr::Conn(RuntimeErr::Internal(error.to_string())))?;
                    use sqlx::ConnectOptions;
                    if !opt.get_sqlx_logging() {
                        raw_opt = raw_opt.disable_statement_logging();
                    } else {
                        raw_opt = raw_opt.log_statements(opt.get_sqlx_logging_level());
                    }
                    let result = opt
                        .sqlx_pool_options::<sqlx::MySql>()
                        .after_connect(move |conn, _| {
                            let timezone = timezone.clone();
                            Box::pin(async move {
                                conn.execute(format!("SET time_zone = '{timezone}';").as_str()).await?;
                                Ok(())
                            })
                        })
                        .connect_with(raw_opt)
                        .await;
                    match result {
                        Ok(pool) => Ok(SqlxMySqlConnector::from_sqlx_mysql_pool(pool)),
                        Err(error) => Err(TardisError::format_error(
                            &format!("[Tardis.RelDBClient] {} Initialization error: {error}", url.redact()),
                            "406-tardis-reldb-conn-init-error",
                        )),
                    }
                }
                #[cfg(feature = "reldb-postgres")]
                "postgres" => {
                    let mut raw_opt = opt.get_url().parse::<sqlx::postgres::PgConnectOptions>().map_err(|error| DbErr::Conn(RuntimeErr::Internal(error.to_string())))?;
                    use sqlx::ConnectOptions;
                    if opt.get_sqlx_logging() {
                        raw_opt = raw_opt.log_statements(opt.get_sqlx_logging_level());
                    } else {
                        raw_opt = raw_opt.disable_statement_logging();
                    }
                    let result = opt
                        .sqlx_pool_options::<sqlx::Postgres>()
                        .after_connect(move |conn, _| {
                            let timezone = timezone.clone();
                            Box::pin(async move {
                                conn.execute(format!("SET TIME ZONE '{timezone}';").as_str()).await?;
                                Ok(())
                            })
                        })
                        .connect_with(raw_opt)
                        .await;
                    match result {
                        Ok(pool) => Ok(SqlxPostgresConnector::from_sqlx_postgres_pool(pool)),
                        Err(error) => Err(TardisError::format_error(
                            &format!("[Tardis.RelDBClient] {} Initialization error: {error}", url.redact()),
                            "406-tardis-reldb-conn-init-error",
                        )),
                    }
                }
                _ => Err(TardisError::format_error(
                    &format!("[Tardis.RelDBClient] {} , current database does not support setting timezone", url.redact()),
                    "406-tardis-reldb-conn-init-error",
                )),
            }
        } else {
            Database::connect(opt).await.map_err(|error| {
                TardisError::format_error(
                    &format!("[Tardis.RelDBClient] {} Initialization error: {error}", url.redact()),
                    "406-tardis-reldb-conn-init-error",
                )
            })
        }?;
        info!(
            "[Tardis.RelDBClient] Initialized, host:{}, port:{}, max_connections:{}",
            url.host_str().unwrap_or(""),
            url.port().unwrap_or(0),
            min_connections
        );
        Ok(TardisRelDBClient {
            con: Arc::new(con),
            compatible_type: *compatible_type,
        })
    }

    /// Get database instance implementation / 获取数据库实例的实现
    pub fn backend(&self) -> DbBackend {
        self.con.get_database_backend()
    }

    /// Get database compatible type / 获取数据库兼容类型
    /// eg. porlardb is compatible with Oracle
    pub fn compatible_type(&self) -> CompatibleType {
        self.compatible_type
    }

    /// Get database connection
    ///
    /// 获取数据库操作连接
    pub fn conn(&self) -> TardisRelDBlConnection {
        TardisRelDBlConnection { conn: self.con.clone(), tx: None }
    }

    /// Initialize basic tables / 初始化基础表
    pub async fn init_basic_tables(&self) -> TardisResult<()> {
        trace!("[Tardis.RelDBClient] Initializing basic tables");
        let tx = self.con.begin().await?;
        let create_all = tardis_db_config::ActiveModel::init(self.con.get_database_backend(), Some("update_time"), self.compatible_type);
        TardisRelDBClient::create_table_inner(&create_all.0, &tx).await?;
        TardisRelDBClient::create_index_inner(&create_all.1, &tx).await?;
        for function_sql in create_all.2 {
            TardisRelDBClient::execute_one_inner(&function_sql, Vec::new(), &tx).await?;
        }
        let create_all = tardis_db_del_record::ActiveModel::init(self.con.get_database_backend(), None, self.compatible_type);
        TardisRelDBClient::create_table_inner(&create_all.0, &tx).await?;
        TardisRelDBClient::create_index_inner(&create_all.1, &tx).await?;
        tx.commit().await?;
        Ok(())
    }

    /// TODO 不支持 not_null nullable  default_value  default_expr indexed, unique 等
    pub(self) async fn create_table_from_entity_inner<E, C>(entity: E, db: &C) -> TardisResult<()>
    where
        C: ConnectionTrait,
        E: EntityTrait,
    {
        trace!("[Tardis.RelDBClient] Creating table from entity {}", entity.table_name());
        let builder = db.get_database_backend();
        let schema = Schema::new(builder);
        let table_create_statement = &schema.create_table_from_entity(entity);
        TardisRelDBClient::create_table_inner(table_create_statement, db).await
    }

    pub(self) async fn create_table_inner<C>(statement: &TableCreateStatement, db: &C) -> TardisResult<()>
    where
        C: ConnectionTrait,
    {
        trace!("[Tardis.RelDBClient] Creating table");
        let statement = db.get_database_backend().build(statement);
        match Self::execute_inner(statement, db).await {
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub(self) async fn create_index_inner<C>(statements: &[IndexCreateStatement], db: &C) -> TardisResult<()>
    where
        C: ConnectionTrait,
    {
        trace!("[Tardis.RelDBClient] Creating index from statements");
        for statement in statements {
            let statement = db.get_database_backend().build(statement);
            Self::execute_inner(statement, db).await?;
        }
        Ok(())
    }

    pub(self) async fn execute_one_inner<C>(sql: &str, params: Vec<Value>, db: &C) -> TardisResult<ExecResult>
    where
        C: ConnectionTrait,
    {
        trace!("[Tardis.RelDBClient] Executing one sql: {}, params:{:?}", sql, params);
        let execute_stmt = Statement::from_sql_and_values(db.get_database_backend(), sql, params);
        Self::execute_inner(execute_stmt, db).await
    }

    pub(self) async fn execute_many_inner<C>(sql: &str, params: Vec<Vec<Value>>, db: &C) -> TardisResult<()>
    where
        C: ConnectionTrait,
    {
        trace!("[Tardis.RelDBClient] Executing many sql: {}, some params:{:?}", sql, params[0]);
        // TODO Performance Optimization
        for param in params {
            let execute_stmt = Statement::from_sql_and_values(db.get_database_backend(), sql, param);
            Self::execute_inner(execute_stmt, db).await?;
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
            Err(error) => TardisResult::Err(TardisError::from(error)),
        }
    }

    pub(self) async fn query_one_inner<C>(sql: &str, params: Vec<Value>, db: &C) -> TardisResult<Option<QueryResult>>
    where
        C: ConnectionTrait,
    {
        trace!("[Tardis.RelDBClient] Querying one sql: {}, params:{:?}", sql, params);
        let query_stmt = Statement::from_sql_and_values(db.get_database_backend(), sql, params);
        let result = db.query_one(query_stmt).await;
        match result {
            Ok(ok) => TardisResult::Ok(ok),
            Err(error) => TardisResult::Err(TardisError::from(error)),
        }
    }

    pub(self) async fn query_all_inner<C>(sql: &str, params: Vec<Value>, db: &C) -> TardisResult<Vec<QueryResult>>
    where
        C: ConnectionTrait,
    {
        trace!("[Tardis.RelDBClient] Querying all sql: {}, params:{:?}", sql, params);
        let query_stmt = Statement::from_sql_and_values(db.get_database_backend(), sql, params);
        let result = db.query_all(query_stmt).await;
        match result {
            Ok(ok) => TardisResult::Ok(ok),
            Err(error) => TardisResult::Err(TardisError::from(error)),
        }
    }

    pub(self) async fn get_dto_inner<C, D>(select_statement: &SelectStatement, db: &C) -> TardisResult<Option<D>>
    where
        C: ConnectionTrait,
        D: FromQueryResult,
    {
        Self::do_get_dto_inner(db.get_database_backend().build(select_statement), db).await
    }

    pub(self) async fn get_dto_by_sql_inner<C, D>(sql: &str, params: Vec<Value>, db: &C) -> TardisResult<Option<D>>
    where
        C: ConnectionTrait,
        D: FromQueryResult,
    {
        Self::do_get_dto_inner(Statement::from_sql_and_values(db.get_database_backend(), sql, params), db).await
    }

    async fn do_get_dto_inner<C, D>(select_statement: Statement, db: &C) -> TardisResult<Option<D>>
    where
        C: ConnectionTrait,
        D: FromQueryResult,
    {
        let result = D::find_by_statement(select_statement).one(db).await;
        match result {
            Ok(r) => TardisResult::Ok(r),
            Err(error) => TardisResult::Err(TardisError::from(error)),
        }
    }

    pub(self) async fn find_dtos_inner<C, D>(select_statement: &SelectStatement, db: &C) -> TardisResult<Vec<D>>
    where
        C: ConnectionTrait,
        D: FromQueryResult,
    {
        Self::do_find_dtos_inner(db.get_database_backend().build(select_statement), db).await
    }

    pub(self) async fn find_dtos_by_sql_inner<C, D>(sql: &str, params: Vec<Value>, db: &C) -> TardisResult<Vec<D>>
    where
        C: ConnectionTrait,
        D: FromQueryResult,
    {
        Self::do_find_dtos_inner(Statement::from_sql_and_values(db.get_database_backend(), sql, params), db).await
    }

    async fn do_find_dtos_inner<C, D>(select_statement: Statement, db: &C) -> TardisResult<Vec<D>>
    where
        C: ConnectionTrait,
        D: FromQueryResult,
    {
        let result = D::find_by_statement(select_statement).all(db).await;
        match result {
            Ok(r) => TardisResult::Ok(r),
            Err(error) => TardisResult::Err(TardisError::from(error)),
        }
    }

    pub(self) async fn paginate_dtos_inner<C, D>(select_statement: &SelectStatement, page_number: u64, page_size: u64, db: &C) -> TardisResult<(Vec<D>, u64)>
    where
        C: ConnectionTrait,
        D: FromQueryResult,
    {
        Self::do_paginate_dtos_inner(db.get_database_backend().build(select_statement), page_number, page_size, db).await
    }

    pub(self) async fn paginate_dtos_by_sql_inner<C, D>(sql: &str, params: Vec<Value>, page_number: u64, page_size: u64, db: &C) -> TardisResult<(Vec<D>, u64)>
    where
        C: ConnectionTrait,
        D: FromQueryResult,
    {
        Self::do_paginate_dtos_inner(Statement::from_sql_and_values(db.get_database_backend(), sql, params), page_number, page_size, db).await
    }

    async fn do_paginate_dtos_inner<C, D>(select_statement: Statement, page_number: u64, page_size: u64, db: &C) -> TardisResult<(Vec<D>, u64)>
    where
        C: ConnectionTrait,
        D: FromQueryResult,
    {
        let select_sql = format!("{} LIMIT {} OFFSET {}", select_statement.sql, page_size, (page_number - 1) * page_size);
        let query_statement = Statement {
            sql: select_sql,
            values: select_statement.values.clone(),
            db_backend: select_statement.db_backend,
        };
        let query_result = D::find_by_statement(query_statement).all(db).await?;
        let count_result = TardisRelDBClient::do_count_inner(select_statement, db).await?;
        Ok((query_result, count_result))
    }

    pub(self) async fn count_inner<C>(select_statement: &SelectStatement, db: &C) -> TardisResult<u64>
    where
        C: ConnectionTrait,
    {
        Self::do_count_inner(db.get_database_backend().build(select_statement), db).await
    }

    pub(self) async fn count_by_sql_inner<C>(sql: &str, params: Vec<Value>, db: &C) -> TardisResult<u64>
    where
        C: ConnectionTrait,
    {
        Self::do_count_inner(Statement::from_sql_and_values(db.get_database_backend(), sql, params), db).await
    }

    async fn do_count_inner<C>(select_statement: Statement, db: &C) -> TardisResult<u64>
    where
        C: ConnectionTrait,
    {
        let count_sql = format!(
            "SELECT COUNT(1) AS count FROM ( {} ) _{}",
            select_statement.sql,
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
        );
        let count_statement = Statement {
            sql: count_sql.clone(),
            values: select_statement.values,
            db_backend: select_statement.db_backend,
        };
        let count_result = CountResp::find_by_statement(count_statement).one(db).await?;
        match count_result {
            Some(r) => TardisResult::Ok(r.count as u64),
            None => TardisResult::Err(TardisError::internal_error(
                &format!("[Tardis.RelDBClient] No results found for count query by {count_sql}"),
                "500-tardis-reldb-count-empty",
            )),
        }
    }

    pub(self) async fn insert_one_inner<T, C>(mut model: T, db: &C, ctx: &TardisContext) -> TardisResult<InsertResult<T>>
    where
        C: ConnectionTrait,
        T: TardisActiveModel,
    {
        trace!("[Tardis.RelDBClient] Inserting one model");
        model.fill_ctx(ctx, true);
        let result = EntityTrait::insert(model).exec(db).await?;
        Ok(result)
    }

    pub(self) async fn insert_many_inner<T, C>(mut models: Vec<T>, db: &C, ctx: &TardisContext) -> TardisResult<()>
    where
        C: ConnectionTrait,
        T: TardisActiveModel,
    {
        trace!("[Tardis.RelDBClient] Inserting many models");
        models.iter_mut().for_each(|m| m.fill_ctx(ctx, true));
        EntityTrait::insert_many(models).exec(db).await?;
        Ok(())
    }

    pub(self) async fn update_one_inner<T, C>(mut model: T, db: &C, ctx: &TardisContext) -> TardisResult<()>
    where
        C: ConnectionTrait,
        T: TardisActiveModel,
    {
        trace!("[Tardis.RelDBClient] Updating one model");
        model.fill_ctx(ctx, false);
        let update = EntityTrait::update(model);
        TardisRelDBClient::execute_inner(db.get_database_backend().build(update.as_query()), db).await?;
        Ok(())
    }

    pub(self) async fn update_many_inner<C>(update_statement: &UpdateStatement, db: &C) -> TardisResult<()>
    where
        C: ConnectionTrait,
    {
        trace!("[Tardis.RelDBClient] Updating many models");
        TardisRelDBClient::execute_inner(db.get_database_backend().build(update_statement), db).await?;
        Ok(())
    }

    pub(self) async fn soft_delete_inner<E, C>(select: Select<E>, delete_user: &str, db: &C) -> TardisResult<u64>
    where
        C: ConnectionTrait,
        E: EntityTrait,
    {
        trace!("[Tardis.RelDBClient] Soft deleting");
        select.soft_delete(delete_user, db).await
    }

    pub(self) async fn soft_delete_custom_inner<E, C>(select: Select<E>, custom_pk_field: &str, db: &C) -> TardisResult<Vec<DeleteEntity>>
    where
        C: ConnectionTrait,
        E: EntityTrait,
    {
        trace!("[Tardis.RelDBClient] Soft deleting custom");
        select.soft_delete_custom(custom_pk_field, db).await
    }
}

/// Database operation connection object / 数据库操作连接对象
pub struct TardisRelDBlConnection {
    conn: Arc<DatabaseConnection>,
    tx: Option<DatabaseTransaction>,
}

impl TardisRelDBlConnection {
    /// Get original connection (generally not recommended) / 获取原始连接(一般不推荐使用)
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// let raw_conn = TardisFuns::reldb().conn().raw_conn();
    /// ```
    pub fn raw_conn(&self) -> &DatabaseConnection {
        self.conn.as_ref()
    }

    /// Get original transaction (if a transaction exists for the current object) (generally not recommended) / 获取原始事务(如果当前对象存在事务的话）(一般不推荐使用)
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// let raw_tx = TardisFuns::reldb().conn().raw_tx().unwrap();
    /// ```
    pub fn raw_tx(&self) -> TardisResult<&DatabaseTransaction> {
        if let Some(tx) = &self.tx {
            Ok(tx)
        } else {
            Err(TardisError::not_found(
                "[Tardis.RelDBClient] The current connection  has no transactions",
                "404-tardis-reldb-tx-empty",
            ))
        }
    }

    pub fn has_tx(&self) -> bool {
        self.tx.is_some()
    }

    /// Open a transaction / 开启一个事务
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// let tx = conn.begin().await.unwrap();
    /// ```
    pub async fn begin(&mut self) -> TardisResult<()> {
        self.tx = Some(self.conn.begin().await?);
        Ok(())
    }

    /// Commit current transaction / 提交当前事务
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// let tx = conn.begin().await.unwrap();
    /// tx.commit().await.unwrap();
    /// ```
    pub async fn commit(self) -> TardisResult<()> {
        if let Some(tx) = self.tx {
            tx.commit().await?;
        }
        Ok(())
    }

    /// Rollback current transaction / 回滚当前事务
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// let tx = conn.begin().await.unwrap();
    /// tx.rollback().await.unwrap();
    /// ```
    pub async fn rollback(self) -> TardisResult<()> {
        if let Some(tx) = self.tx {
            tx.rollback().await?;
        }
        Ok(())
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Create a table from an entity / 从实体中创建表
    ///
    /// # Arguments
    ///
    ///  * `entity` - entity / 实体
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// conn.create_table_from_entity(tardis_db_config::Entity).await.unwrap();
    /// ```
    pub async fn create_table_from_entity<E>(&self, entity: E) -> TardisResult<()>
    where
        E: EntityTrait,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::create_table_from_entity_inner(entity, tx).await
        } else {
            TardisRelDBClient::create_table_from_entity_inner(entity, self.conn.as_ref()).await
        }
    }

    /// Create table index and functions / 创建表、索引和函数
    ///
    /// # Arguments
    ///
    ///  * `params.0` -  Statement for creating table  / 创建表的Statement
    ///  * `params.1` -  Statement for creating index / 创建索引的Statements
    ///  * `params.2` -  sql for functions / 创建函数的sqls
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::reldb_client::TardisActiveModel;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// conn.init(&tardis_db_config::ActiveModel::init(TardisFuns::reldb().backend(),Some("update_time"))).await.unwrap();
    /// ```
    pub async fn init(&self, params: (TableCreateStatement, Vec<IndexCreateStatement>, Vec<String>)) -> TardisResult<()> {
        self.create_table(&params.0).await?;
        self.create_index(&params.1).await?;
        for function_sql in params.2 {
            self.execute_one(&function_sql, Vec::new()).await?;
        }
        Ok(())
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Create table  / 创建表
    ///
    /// # Arguments
    ///
    ///  * `statement` -  Statement for creating a table / 创建表的Statement
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::reldb_client::TardisActiveModel;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// conn.create_table(&tardis_db_config::ActiveModel::create_table_statement(TardisFuns::reldb().backend())).await.unwrap();
    /// ```
    pub async fn create_table(&self, statement: &TableCreateStatement) -> TardisResult<()> {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::create_table_inner(statement, tx).await
        } else {
            TardisRelDBClient::create_table_inner(statement, self.conn.as_ref()).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Create index / 创建索引
    ///
    /// # Arguments
    ///
    ///  * `statement` -  Statement for creating index / 创建索引的Statement
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::reldb_client::TardisActiveModel;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// conn.create_index(&tardis_db_config::ActiveModel::create_index_statement()).await.unwrap();
    /// ```
    pub async fn create_index(&self, statements: &[IndexCreateStatement]) -> TardisResult<()> {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::create_index_inner(statements, tx).await
        } else {
            TardisRelDBClient::create_index_inner(statements, self.conn.as_ref()).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Get a record, return a custom structure / 获取一条记录，返回自定义结构体
    ///
    /// # Arguments
    ///
    ///  * `select_statement` - Statement of the query / 查询的Statement
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_orm::sea_query::*;
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::reldb_client::TardisActiveModel;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// let resp = conn.get_dto(&Query::select()
    ///     .column(tardis_db_config::Column::Id)
    ///     .column(tardis_db_config::Column::Name)
    ///     .from(tardis_db_config::Entity)
    ///     .and_where(Expr::col(tardis_db_config::Column::Id).eq("xxx"))
    /// ).await.unwrap();
    /// ```
    pub async fn get_dto<D>(&self, select_statement: &SelectStatement) -> TardisResult<Option<D>>
    where
        D: FromQueryResult,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::get_dto_inner(select_statement, tx).await
        } else {
            TardisRelDBClient::get_dto_inner(select_statement, self.conn.as_ref()).await
        }
    }

    /// Get a record, return a custom structure / 获取一条记录，返回自定义结构体
    ///
    /// # Arguments
    ///
    ///  * `sql` - sql of the query / 查询SQL
    ///  * `params` - params of the query / 查询参数
    ///
    pub async fn get_dto_by_sql<D>(&self, sql: &str, params: Vec<Value>) -> TardisResult<Option<D>>
    where
        D: FromQueryResult,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::get_dto_by_sql_inner(sql, params, tx).await
        } else {
            TardisRelDBClient::get_dto_by_sql_inner(sql, params, self.conn.as_ref()).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Get multiple rows and return a custom structure / 获取多条记录，返回自定义结构体
    ///
    /// # Arguments
    ///
    ///  * `select_statement` - Statement of the query / 查询的Statement
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_orm::sea_query::*;
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::reldb_client::TardisActiveModel;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// let resp = conn.find_dtos(&Query::select()
    ///     .column(tardis_db_config::Column::Id)
    ///     .column(tardis_db_config::Column::Name)
    ///     .from(tardis_db_config::Entity)
    /// ).await.unwrap();
    /// ```
    pub async fn find_dtos<D>(&self, select_statement: &SelectStatement) -> TardisResult<Vec<D>>
    where
        D: FromQueryResult,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::find_dtos_inner(select_statement, tx).await
        } else {
            TardisRelDBClient::find_dtos_inner(select_statement, self.conn.as_ref()).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Get multiple rows and return a custom structure / 获取多条记录，返回自定义结构体
    ///
    /// # Arguments
    ///
    ///  * `sql` - sql of the query / 查询SQL
    ///  * `params` - params of the query / 查询参数
    ///
    pub async fn find_dtos_by_sql<D>(&self, sql: &str, params: Vec<Value>) -> TardisResult<Vec<D>>
    where
        D: FromQueryResult,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::find_dtos_by_sql_inner(sql, params, tx).await
        } else {
            TardisRelDBClient::find_dtos_by_sql_inner(sql, params, self.conn.as_ref()).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Paging to get multiple records and the total number of records, returning a custom structure / 分页获取多条记录及总记录数，返回自定义结构体
    ///
    /// # Arguments
    ///
    ///  * `select_statement` - Statement of the query / 查询的Statement
    ///  * `page_number` -  Current page number, starting from 1 / 当前页码，从1开始
    ///  * `page_size` -  Number of records per page / 每页记录数
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_orm::sea_query::*;
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::reldb_client::TardisActiveModel;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// let resp = conn.paginate_dtos(&Query::select()
    ///     .column(tardis_db_config::Column::Id)
    ///     .column(tardis_db_config::Column::Name)
    ///     .from(tardis_db_config::Entity),
    ///     1,10
    /// ).await.unwrap();
    /// ```
    pub async fn paginate_dtos<D>(&self, select_statement: &SelectStatement, page_number: u64, page_size: u64) -> TardisResult<(Vec<D>, u64)>
    where
        D: FromQueryResult,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::paginate_dtos_inner(select_statement, page_number, page_size, tx).await
        } else {
            TardisRelDBClient::paginate_dtos_inner(select_statement, page_number, page_size, self.conn.as_ref()).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Paging to get multiple records and the total number of records, returning a custom structure / 分页获取多条记录及总记录数，返回自定义结构体
    ///
    /// # Arguments
    ///
    ///  * `sql` - sql of the query / 查询SQL
    ///  * `params` - params of the query / 查询参数
    ///  * `page_number` -  Current page number, starting from 1 / 当前页码，从1开始
    ///  * `page_size` -  Number of records per page / 每页记录数
    ///
    pub async fn paginate_dtos_by_sql<D>(&self, sql: &str, params: Vec<Value>, page_number: u64, page_size: u64) -> TardisResult<(Vec<D>, u64)>
    where
        D: FromQueryResult,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::paginate_dtos_by_sql_inner(sql, params, page_number, page_size, tx).await
        } else {
            TardisRelDBClient::paginate_dtos_by_sql_inner(sql, params, page_number, page_size, self.conn.as_ref()).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Get number of records / 获取记录数量
    ///
    /// # Arguments
    ///
    ///  * `select_statement` - Statement of the query / 查询的Statement
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_orm::sea_query::*;
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::reldb_client::TardisActiveModel;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// let resp = conn.count(&Query::select()
    ///     .column(tardis_db_config::Column::Id)
    ///     .column(tardis_db_config::Column::Name)
    ///     .from(tardis_db_config::Entity)
    /// ).await.unwrap();
    /// ```
    pub async fn count(&self, select_statement: &SelectStatement) -> TardisResult<u64> {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::count_inner(select_statement, tx).await
        } else {
            TardisRelDBClient::count_inner(select_statement, self.conn.as_ref()).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Get number of records / 获取记录数量
    ///
    /// # Arguments
    ///
    ///  * `sql` - sql of the query / 查询SQL
    ///  * `params` - params of the query / 查询参数
    ///
    /// ```
    pub async fn count_by_sql(&self, sql: &str, params: Vec<Value>) -> TardisResult<u64> {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::count_by_sql_inner(sql, params, tx).await
        } else {
            TardisRelDBClient::count_by_sql_inner(sql, params, self.conn.as_ref()).await
        }
    }

    #[instrument(skip_all)]
    /// Execute SQL operations (provide custom SQL processing capabilities) / 执行SQL操作（提供自定义SQL处理能力）
    ///
    /// # Arguments
    ///
    ///  * `statement` -  Custom statement / 自定义Statement
    ///
    pub async fn execute<S>(&self, statement: &S) -> TardisResult<ExecResult>
    where
        S: StatementBuilder,
    {
        let statement = self.conn.get_database_backend().build(statement);
        if let Some(tx) = &self.tx {
            TardisRelDBClient::execute_inner(statement, tx).await
        } else {
            TardisRelDBClient::execute_inner(statement, self.conn.as_ref()).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Execute SQL operations (provide custom SQL processing capabilities) / 执行SQL操作（提供自定义SQL处理能力）
    pub async fn execute_one(&self, sql: &str, params: Vec<Value>) -> TardisResult<ExecResult> {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::execute_one_inner(sql, params, tx).await
        } else {
            TardisRelDBClient::execute_one_inner(sql, params, self.conn.as_ref()).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    // Execute SQL operations (provide custom SQL processing capabilities) / 执行SQL操作（提供自定义SQL处理能力）
    pub async fn execute_many(&self, sql: &str, params: Vec<Vec<Value>>) -> TardisResult<()> {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::execute_many_inner(sql, params, tx).await
        } else {
            TardisRelDBClient::execute_many_inner(sql, params, self.conn.as_ref()).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    pub async fn query_one(&self, sql: &str, params: Vec<Value>) -> TardisResult<Option<QueryResult>> {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::query_one_inner(sql, params, tx).await
        } else {
            TardisRelDBClient::query_one_inner(sql, params, self.conn.as_ref()).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    pub async fn query_all(&self, sql: &str, params: Vec<Value>) -> TardisResult<Vec<QueryResult>> {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::query_all_inner(sql, params, tx).await
        } else {
            TardisRelDBClient::query_all_inner(sql, params, self.conn.as_ref()).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Insert a record and return primary key value / 插入一条记录，返回主键值
    ///
    /// # Arguments
    ///
    ///  * `model` -  Record to be inserted / 要插入的记录
    ///  * `ctx` -  TardisContext
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_orm::*;
    /// use tardis::db::sea_orm::sea_query::*;
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::reldb_client::TardisActiveModel;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// let resp = conn.insert_one(tardis_db_config::ActiveModel {
    ///     k: Set("ke".to_string()),
    ///     v: Set("ve".to_string()),
    ///     ..Default::default()
    /// },&ctx).await.unwrap();
    /// ```
    pub async fn insert_one<T>(&self, model: T, ctx: &TardisContext) -> TardisResult<InsertResult<T>>
    where
        T: TardisActiveModel,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::insert_one_inner(model, tx, ctx).await
        } else {
            TardisRelDBClient::insert_one_inner(model, self.conn.as_ref(), ctx).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Insert multiple records / 插入多条记录
    ///
    /// # Arguments
    ///
    ///  * `models` -  Set of records to be inserted / 要插入的记录集
    ///  * `ctx` -  TardisContext
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_orm::*;
    /// use tardis::db::sea_orm::sea_query::*;
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::reldb_client::TardisActiveModel;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// let resp = conn.insert_many(vec![
    ///     tardis_db_config::ActiveModel {
    ///          k: Set("ke".to_string()),
    ///          v: Set("ve".to_string()),
    ///          ..Default::default()
    ///     }
    ///  ],&ctx).await.unwrap();
    /// ```
    pub async fn insert_many<T>(&self, models: Vec<T>, ctx: &TardisContext) -> TardisResult<()>
    where
        T: TardisActiveModel,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::insert_many_inner(models, tx, ctx).await
        } else {
            TardisRelDBClient::insert_many_inner(models, self.conn.as_ref(), ctx).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Update a record / 更新一条记录
    ///
    /// # Arguments
    ///
    ///  * `model` -  Records to be inserted / 要插入的记录
    ///  * `ctx` -  TardisContext
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_orm::*;
    /// use tardis::db::sea_orm::sea_query::*;
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::reldb_client::TardisActiveModel;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// let resp = conn.update_one(tardis_db_config::ActiveModel {
    ///     id: Set("111".to_string()),
    ///     k: Set("ke".to_string()),
    ///     v: Set("ve".to_string()),
    ///     ..Default::default()
    /// },&ctx).await.unwrap();
    /// ```
    pub async fn update_one<T>(&self, model: T, ctx: &TardisContext) -> TardisResult<()>
    where
        T: TardisActiveModel,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::update_one_inner(model, tx, ctx).await
        } else {
            TardisRelDBClient::update_one_inner(model, self.conn.as_ref(), ctx).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Update multiple records / 更新多条记录
    ///
    /// # Arguments
    ///
    ///  * `update_statement` -  Statement to be updated / 要更新的Statement
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_orm::*;
    /// use tardis::db::sea_orm::sea_query::*;
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::reldb_client::TardisActiveModel;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// let resp = conn.update_many(Query::update()
    ///     .table(tardis_db_config::Entity)
    ///     .values(vec![
    ///       (tardis_db_config::Column::k, Set("ke".to_string())),
    ///     ])
    ///     .and_where(Expr::col(tardis_db_config::Column::id).eq("111"))).await.unwrap();
    /// ```
    pub async fn update_many(&self, update_statement: &UpdateStatement) -> TardisResult<()> {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::update_many_inner(update_statement, tx).await
        } else {
            TardisRelDBClient::update_many_inner(update_statement, self.conn.as_ref()).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Soft delete record(s) (primary key is Id) / 软删除记录(主键为Id)
    ///
    /// # Arguments
    ///
    ///  * `select` -  Select object / Select对象
    ///  * `delete_user` -  Delete user / 删除人
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_orm::*;
    /// use tardis::db::sea_orm::sea_query::*;
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::reldb_client::TardisActiveModel;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// let resp = conn.soft_delete(tardis_db_config::Entity::find().filter(Expr::col(tardis_db_config::Column::Id).eq("111")),"admin").await.unwrap();
    /// ```
    pub async fn soft_delete<E>(&self, select: Select<E>, delete_user: &str) -> TardisResult<u64>
    where
        E: EntityTrait,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::soft_delete_inner(select, delete_user, tx).await
        } else {
            TardisRelDBClient::soft_delete_inner(select, delete_user, self.conn.as_ref()).await
        }
    }

    #[instrument(name = "reldb_query", skip_all)]
    /// Soft delete record(s) (custom primary key) / 软删除记录(自定义主键)
    ///
    /// # Arguments
    ///
    ///  * `select` -  Select object / Select对象
    ///  * `custom_pk_field` -  Custom Primary Key / 自定义主键
    ///  * `delete_user` -  Delete user / 删除人
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_orm::*;
    /// use tardis::db::sea_orm::sea_query::*;
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::reldb_client::TardisActiveModel;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// let resp = conn.soft_delete_custom(tardis_db_config::Entity::find().filter(Expr::col(tardis_db_config::Column::Id).eq("111")),"iam_id").await.unwrap();
    /// ```
    pub async fn soft_delete_custom<E>(&self, select: Select<E>, custom_pk_field: &str) -> TardisResult<Vec<DeleteEntity>>
    where
        E: EntityTrait,
    {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::soft_delete_custom_inner(select, custom_pk_field, tx).await
        } else {
            TardisRelDBClient::soft_delete_custom_inner(select, custom_pk_field, self.conn.as_ref()).await
        }
    }
}

#[async_trait]
pub trait TardisSeaORMExtend {
    async fn soft_delete<C>(self, delete_user: &str, db: &C) -> TardisResult<u64>
    where
        C: ConnectionTrait;

    async fn soft_delete_with_pk<C>(self, custom_pk_field: &str, delete_user: &str, db: &C) -> TardisResult<u64>
    where
        C: ConnectionTrait;

    async fn soft_delete_custom<C>(self, custom_pk_field: &str, db: &C) -> TardisResult<Vec<DeleteEntity>>
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
        self.soft_delete_with_pk("id", delete_user, db).await
    }

    async fn soft_delete_with_pk<C>(self, custom_pk_field: &str, delete_user: &str, db: &C) -> TardisResult<u64>
    where
        C: ConnectionTrait,
    {
        let delete_entities = self.soft_delete_custom(custom_pk_field, db).await?;
        let count = delete_entities.len() as u64;
        for delete_entity in delete_entities {
            tardis_db_del_record::ActiveModel {
                entity_name: Set(delete_entity.entity_name.to_string()),
                record_id: Set(delete_entity.record_id.to_string()),
                content: Set(delete_entity.content),
                creator: Set(delete_user.to_string()),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
        Ok(count)
    }

    async fn soft_delete_custom<C>(self, custom_pk_field: &str, db: &C) -> TardisResult<Vec<DeleteEntity>>
    where
        C: ConnectionTrait,
    {
        let db_backend: DbBackend = db.get_database_backend();

        let sql = self.build(db_backend).sql.replace('?', "''");
        let ast = match Parser::parse_sql(
            match db.get_database_backend() {
                DatabaseBackend::MySql => &MySqlDialect {},
                DatabaseBackend::Postgres => &PostgreSqlDialect {},
                DatabaseBackend::Sqlite => &SQLiteDialect {},
            },
            &sql,
        )?
        .pop()
        {
            Some(ast) => ast,
            None => {
                return Err(TardisError::format_error(
                    "[Tardis.RelDBClient] Sql parsing error, no valid Statement found",
                    "406-tardis-reldb-sql-error",
                ))
            }
        };
        let mut table_name = String::new();
        if let ast::Statement::Query(query) = ast {
            if let SetExpr::Select(select) = query.body.as_ref() {
                if let TableFactor::Table { name, .. } = &select.from[0].relation {
                    if let Some(table_ident) = name.0.first().and_then(ast::ObjectNamePart::as_ident) {
                        table_name.clone_from(&table_ident.value);
                    }
                }
            }
        }
        if table_name.is_empty() {
            return TardisResult::Err(TardisError::not_found(
                "[Tardis.RelDBClient] Sql parsing error, the name of the table to be soft deleted was not found",
                "404-tardis-reldb-soft-delete-table-not-exit",
            ));
        }

        let rows = self.into_json().all(db).await?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let mut ids: Vec<Value> = Vec::new();
        let mut delete_entities = Vec::with_capacity(ids.len());
        for row in rows {
            let id = row[custom_pk_field].clone();
            let json = TardisFuns::json.obj_to_string(&row)?;
            if id.is_string() {
                ids.push(
                    (*id.as_str().as_ref().ok_or_else(|| {
                        TardisError::internal_error(
                            &format!("[Tardis.RelDBClient] The primary key [{id}] in a soft delete operation is not a character type"),
                            "500-tardis-reldb-id-not-char",
                        )
                    })?)
                    .to_string()
                    .into(),
                );
            } else {
                ids.push(
                    id.as_u64()
                        .ok_or_else(|| {
                            TardisError::internal_error(
                                &format!("[Tardis.RelDBClient] The primary key [{id}] in a soft delete operation is not a number type"),
                                "500-tardis-reldb-id-not-num",
                            )
                        })?
                        .into(),
                );
            }
            delete_entities.push(DeleteEntity {
                entity_name: table_name.to_string(),
                record_id: id.to_string(),
                content: json,
            });
        }
        let statement = Statement::from_sql_and_values(
            db_backend,
            match db_backend {
                DatabaseBackend::Postgres => format!(
                    "DELETE FROM {} WHERE {} IN ({})",
                    table_name,
                    custom_pk_field,
                    ids.iter().enumerate().map(|(idx, _)| format!("${}", idx + 1)).collect::<Vec<String>>().join(",")
                ),
                _ => format!(
                    "DELETE FROM {} WHERE {} IN ({})",
                    table_name,
                    custom_pk_field,
                    ids.iter().map(|_| "?").collect::<Vec<&str>>().join(",")
                ),
            }
            .as_str(),
            ids,
        );
        let result = db.execute(statement).await;
        match result {
            Ok(_) => Ok(delete_entities),
            Err(error) => Err(TardisError::from(error)),
        }
    }
}

/// 对 `ActiveModelBehavior` 的扩展操作
#[async_trait]
pub trait TardisActiveModel: ActiveModelBehavior {
    /// Fill TardisContext / 填充TardisContext
    ///
    /// # Arguments
    ///
    ///  * `ctx` -  TardisContext
    ///  * `is_insert` -  whether to insert the operation / 是否插入操作
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::basic::dto::TardisContext;
    /// use tardis::db::sea_orm::*;
    ///
    /// fn fill_ctx(&mut self, ctx: &TardisContext, is_insert: bool) {
    ///     if is_insert {
    ///         self.rel_app_code = Set(ctx.app_code.to_string());
    ///     }
    ///     self.updater_code = Set(ctx.account_code.to_string());
    /// }
    /// ```
    fn fill_ctx(&mut self, ctx: &TardisContext, is_insert: bool);

    /// Create table and index / 创建表和索引
    ///
    /// # Arguments
    ///
    ///  * `db` -  database instance type / 数据库实例类型
    ///  * `update_time_field` -  update time field / 更新字段
    fn init(db: DbBackend, update_time_field: Option<&str>, compatible_type: CompatibleType) -> (TableCreateStatement, Vec<IndexCreateStatement>, Vec<String>) {
        let create_table_statement = Self::create_table_statement(db);
        let create_index_statement = Self::create_index_statement();
        if let Some(table_name) = create_table_statement.get_table_name() {
            let table_name = match table_name {
                sea_query::TableRef::Table(t)
                | sea_query::TableRef::SchemaTable(_, t)
                | sea_query::TableRef::DatabaseSchemaTable(_, _, t)
                | sea_query::TableRef::TableAlias(t, _)
                | sea_query::TableRef::SchemaTableAlias(_, t, _)
                | sea_query::TableRef::DatabaseSchemaTableAlias(_, _, t, _) => t.to_string(),
                _ => unimplemented!(),
            };
            let create_function_sql = Self::create_function_sqls(db, &table_name, update_time_field, compatible_type);
            return (create_table_statement, create_index_statement, create_function_sql);
        }
        (create_table_statement, create_index_statement, Vec::new())
    }

    /// Create table / 创建表
    ///
    /// # Arguments
    ///
    ///  * `db` -  database instance type / 数据库实例类型
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::sea_orm::*;
    /// use tardis::db::sea_orm::sea_query::*;
    /// fn create_table_statement(db_type: DbBackend) -> TableCreateStatement {
    ///     match db_type {
    ///         DbBackend::MySql => Table::create()
    ///             .table(tardis_db_config::Entity.table_ref())
    ///             .if_not_exists()
    ///             .col(ColumnDef::new(tardis_db_config::Column::Id).not_null().string().primary_key())
    ///             .col(ColumnDef::new(tardis_db_config::Column::K).not_null().string().unique_key())
    ///             .col(ColumnDef::new(tardis_db_config::Column::V).not_null().text())
    ///             .col(ColumnDef::new(tardis_db_config::Column::Creator).not_null().string())
    ///             .col(ColumnDef::new(tardis_db_config::Column::Updater).not_null().string())
    ///             .to_owned(),
    ///         DbBackend::Postgres => {
    ///             Table::create()
    ///                 .table(tardis_db_config::Entity.table_ref())
    ///                 .if_not_exists()
    ///                 .col(ColumnDef::new(tardis_db_config::Column::Id).not_null().string().primary_key())
    ///                 .col(ColumnDef::new(tardis_db_config::Column::K).not_null().string().unique_key())
    ///                 .col(ColumnDef::new(tardis_db_config::Column::V).not_null().text())
    ///                 .col(ColumnDef::new(tardis_db_config::Column::Creator).not_null().string())
    ///                 .col(ColumnDef::new(tardis_db_config::Column::Updater).not_null().string())
    ///                 .to_owned()
    ///         }
    ///         DbBackend::Sqlite =>{
    ///             Table::create()
    ///                 .table(tardis_db_config::Entity.table_ref())
    ///                 .if_not_exists()
    ///                 .col(ColumnDef::new(tardis_db_config::Column::Id).not_null().string().primary_key())
    ///                 .col(ColumnDef::new(tardis_db_config::Column::K).not_null().string().unique_key())
    ///                 .col(ColumnDef::new(tardis_db_config::Column::V).not_null().text())
    ///                 .col(ColumnDef::new(tardis_db_config::Column::Creator).not_null().string())
    ///                 .col(ColumnDef::new(tardis_db_config::Column::Updater).not_null().string())
    ///                 .to_owned()
    ///         }
    ///     }
    /// }
    /// ```
    fn create_table_statement(_: DbBackend) -> TableCreateStatement {
        TableCreateStatement::new()
    }

    /// Create index / 创建索引
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::sea_orm::*;
    /// use tardis::db::sea_orm::sea_query::*;
    /// fn create_index_statement() -> Vec<IndexCreateStatement> {
    ///         vec![
    ///             Index::create()
    ///                 .name(&format!("idx-{}-{}", tardis_db_config::Entity.table_name(),tardis_db_config::Column::K.to_string()))
    ///                 .table(tardis_db_config::Entity)
    ///                 .col(tardis_db_config::Column::K)
    ///                 .to_owned(),
    ///         ]
    ///     }
    /// ```
    fn create_index_statement() -> Vec<IndexCreateStatement> {
        vec![]
    }

    /// Create functions / 创建函数
    fn create_function_sqls(db: DbBackend, table_name: &str, update_time_field: Option<&str>, compatible_type: CompatibleType) -> Vec<String> {
        if db == DbBackend::Postgres {
            if let Some(update_time_field) = update_time_field {
                return Self::create_function_postgresql_auto_update_time(table_name, update_time_field, compatible_type);
            }
        }
        vec![]
    }

    fn create_function_postgresql_auto_update_time(table_name: &str, update_time_field: &str, compatible_type: CompatibleType) -> Vec<String> {
        match compatible_type {
            CompatibleType::None => {
                vec![
                    format!(
                        r###"CREATE OR REPLACE FUNCTION TARDIS_AUTO_UPDATE_TIME_{}()
        RETURNS TRIGGER AS $$
        BEGIN
            NEW.{} = now();
            RETURN NEW;
        END;
        $$ language 'plpgsql';"###,
                        update_time_field.replace('-', "_"),
                        update_time_field
                    ),
                    format!("DROP TRIGGER IF EXISTS TARDIS_AUTO_UPDATE_TIME_ON ON {table_name};"),
                    format!(
                        r###"CREATE TRIGGER TARDIS_AUTO_UPDATE_TIME_ON
            BEFORE UPDATE
            ON
                {table_name}
            FOR EACH ROW
        EXECUTE PROCEDURE TARDIS_AUTO_UPDATE_TIME_{}();"###,
                        update_time_field.replace('-', "_")
                    ),
                ]
            }
            CompatibleType::Oracle => vec![format!(
                r###"CREATE OR REPLACE TRIGGER TARDIS_AUTO_UPDATE_TIME_ON
            BEFORE UPDATE
            ON
                {}
            FOR EACH ROW
            BEGIN
                NEW.{}= now();
                RETURN NEW;
            END;"###,
                table_name, update_time_field
            )],
        }
    }
}

#[derive(Debug, FromQueryResult)]
struct CountResp {
    count: i64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DeleteEntity {
    pub entity_name: String,
    pub record_id: String,
    pub content: String,
}

#[derive(Debug, FromQueryResult)]
pub struct IdResp {
    pub id: String,
}

impl From<DbErr> for TardisError {
    fn from(error: DbErr) -> Self {
        error!("[Tardis.RelDBClient] DbErr: {}", error.to_string());
        TardisError::wrap(&format!("[Tardis.RelDBClient] {error:?}"), "-1-tardis-reldb-error")
    }
}

impl From<ParserError> for TardisError {
    fn from(error: ParserError) -> Self {
        error!("[Tardis.RelDBClient] ParserError: {}", error.to_string());
        TardisError::wrap(&format!("[Tardis.RelDBClient] {error:?}"), "406-tardis-reldb-sql-error")
    }
}
