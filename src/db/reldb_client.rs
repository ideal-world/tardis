use std::collections::HashMap;
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
use crate::serde::{Deserialize, Serialize};
use crate::{FrameworkConfig, TardisFuns};

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
///     use tardis::db::sea_query::*;
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
///         fn fill_cxt(&mut self, cxt: &TardisContext, is_insert: bool) {}
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
/// 3. Create TardisContext / 创建TardisContext [TardisContext](crate::basic::dto::TardisContext)
///
/// 4. Use `TardisRelDBClient` to operate database / 使用 `TardisRelDBClient` 操作数据库, E.g:
/// ```ignore
/// use std::process::id;
/// use tardis::basic::dto::TardisContext;
/// use tardis::db::domain::tardis_db_config;
/// use tardis::db::reldb_client::IdResp;
/// use tardis::db::sea_query::*;
/// use tardis::db::sea_orm::*;
/// use tardis::TardisFuns;
/// let cxt = TardisContext{
/// // Define Context
///   ..Default::default()
/// };
/// let conn = TardisFuns::reldb().conn();
/// conn.insert_one(tardis_db_config::ActiveModel {
///     k: Set("ke".to_string()),
///     v: Set("ve".to_string()),
///     ..Default::default()
/// },&cxt).await.unwrap();
///
/// conn.paginate_dtos::<IdResp>(&Query::select()
///     .column(tardis_db_config::Column::Id)
///     .from(tardis_db_config::Entity),
///     1,10
/// ).await.unwrap();
/// ```
pub struct TardisRelDBClient {
    con: DatabaseConnection,
}

impl TardisRelDBClient {
    /// Initialize configuration from the database configuration object / 从数据库配置对象中初始化配置
    pub async fn init_by_conf(conf: &FrameworkConfig) -> TardisResult<HashMap<String, TardisRelDBClient>> {
        let mut clients = HashMap::new();
        clients.insert(
            "".to_string(),
            TardisRelDBClient::init(
                &conf.db.url,
                conf.db.max_connections,
                conf.db.min_connections,
                conf.db.connect_timeout_sec,
                conf.db.idle_timeout_sec,
            )
            .await?,
        );
        for (k, v) in &conf.db.modules {
            clients.insert(
                k.to_string(),
                TardisRelDBClient::init(&v.url, v.max_connections, v.min_connections, v.connect_timeout_sec, v.idle_timeout_sec).await?,
            );
        }
        Ok(clients)
    }

    /// Initialize configuration / 初始化配置
    pub async fn init(
        str_url: &str,
        max_connections: u32,
        min_connections: u32,
        connect_timeout_sec: Option<u64>,
        idle_timeout_sec: Option<u64>,
    ) -> TardisResult<TardisRelDBClient> {
        let url = Url::parse(str_url).map_err(|_| TardisError::BadRequest(format!("[Tardis.RelDBClient] Invalid url {}", str_url)))?;
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

    /// Get database instance implementation / 获取数据库实例的实现
    pub fn backend(&self) -> DbBackend {
        self.con.get_database_backend()
    }

    /// Get database connection
    ///
    /// 获取数据库操作连接
    pub fn conn(&self) -> TardisRelDBlConnection {
        TardisRelDBlConnection { conn: &self.con, tx: None }
    }

    /// Initialize basic tables / 初始化基础表
    async fn init_basic_tables(&self) -> TardisResult<()> {
        let tx = self.con.begin().await?;
        let config_create_table_statements = tardis_db_config::ActiveModel::create_table_and_index_statement(self.con.get_database_backend());
        TardisRelDBClient::create_table_and_index_inner(&config_create_table_statements, &tx).await?;
        let del_record_create_statements = tardis_db_del_record::ActiveModel::create_table_and_index_statement(self.con.get_database_backend());
        TardisRelDBClient::create_table_and_index_inner(&del_record_create_statements, &tx).await?;
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

    pub(self) async fn create_table_and_index_inner<C>(statements: &(TableCreateStatement, Vec<IndexCreateStatement>), db: &C) -> TardisResult<()>
    where
        C: ConnectionTrait,
    {
        Self::create_table_inner(&statements.0, db).await?;
        Self::create_index_inner(&statements.1, db).await
    }

    pub(self) async fn create_table_inner<C>(statement: &TableCreateStatement, db: &C) -> TardisResult<()>
    where
        C: ConnectionTrait,
    {
        let statement = db.get_database_backend().build(statement);
        match TardisRelDBClient::execute_inner(statement, db).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub(self) async fn create_index_inner<C>(statements: &[IndexCreateStatement], db: &C) -> TardisResult<()>
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

    pub(self) async fn get_dto_inner<C, D>(select_statement: &SelectStatement, db: &C) -> TardisResult<Option<D>>
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

    pub(self) async fn find_dtos_inner<C, D>(select_statement: &SelectStatement, db: &C) -> TardisResult<Vec<D>>
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

    pub(self) async fn paginate_dtos_inner<C, D>(select_statement: &SelectStatement, page_number: u64, page_size: u64, db: &C) -> TardisResult<(Vec<D>, u64)>
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

    pub(self) async fn count_inner<C>(select_statement: &SelectStatement, db: &C) -> TardisResult<u64>
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

    pub(self) async fn insert_one_inner<T, C>(mut model: T, db: &C, cxt: &TardisContext) -> TardisResult<InsertResult<T>>
    where
        C: ConnectionTrait,
        T: TardisActiveModel,
    {
        model.fill_cxt(cxt, true);
        let result = EntityTrait::insert(model).exec(db).await?;
        Ok(result)
    }

    pub(self) async fn insert_many_inner<T, C>(mut models: Vec<T>, db: &C, cxt: &TardisContext) -> TardisResult<()>
    where
        C: ConnectionTrait,
        T: TardisActiveModel,
    {
        models.iter_mut().for_each(|m| m.fill_cxt(cxt, true));
        EntityTrait::insert_many(models).exec(db).await?;
        Ok(())
    }

    pub(self) async fn update_one_inner<T, C>(mut model: T, db: &C, cxt: &TardisContext) -> TardisResult<()>
    where
        C: ConnectionTrait,
        T: TardisActiveModel,
    {
        model.fill_cxt(cxt, false);
        let update = EntityTrait::update(model);
        TardisRelDBClient::execute_inner(db.get_database_backend().build(update.as_query()), db).await?;
        Ok(())
    }

    pub(self) async fn update_many_inner<C>(update_statement: &UpdateStatement, db: &C) -> TardisResult<()>
    where
        C: ConnectionTrait,
    {
        TardisRelDBClient::execute_inner(db.get_database_backend().build(update_statement), db).await?;
        Ok(())
    }

    pub(self) async fn soft_delete_inner<E, C>(select: Select<E>, delete_user: &str, db: &C) -> TardisResult<u64>
    where
        C: ConnectionTrait,
        E: EntityTrait,
    {
        select.soft_delete(delete_user, db).await
    }

    pub(self) async fn soft_delete_custom_inner<'a, E, C>(select: Select<E>, custom_pk_field: &str, db: &'a C) -> TardisResult<Vec<DeleteEntity>>
    where
        C: ConnectionTrait,
        E: EntityTrait,
    {
        select.soft_delete_custom(custom_pk_field, db).await
    }
}

/// Database operation connection object / 数据库操作连接对象
pub struct TardisRelDBlConnection<'a> {
    conn: &'a DatabaseConnection,
    tx: Option<DatabaseTransaction>,
}

impl<'a> TardisRelDBlConnection<'a> {
    /// Get original connection (generally not recommended) / 获取原始连接(一般不推荐使用)
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// let raw_conn = TardisFuns::reldb().conn().raw_conn();
    /// ```
    pub fn raw_conn(&self) -> &DatabaseConnection {
        self.conn
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
            Err(TardisError::NotFound("[Tardis.RelDBClient] The current connection  has no transactions".to_string()))
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
            TardisRelDBClient::create_table_from_entity_inner(entity, self.conn).await
        }
    }

    /// Create table and index / 创建表和索引
    ///
    /// # Arguments
    ///
    ///  * `statements` -  Statement for creating table and creating index / 创建表和创建索引的Statement
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::reldb_client::TardisActiveModel;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// conn.create_table_and_index(&tardis_db_config::ActiveModel::create_table_and_index_statement(TardisFuns::reldb().backend())).await.unwrap();
    /// ```
    pub async fn create_table_and_index(&self, statements: &(TableCreateStatement, Vec<IndexCreateStatement>)) -> TardisResult<()> {
        self.create_table(&statements.0).await?;
        self.create_index(&statements.1).await
    }

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
            TardisRelDBClient::create_table_inner(statement, self.conn).await
        }
    }

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
            TardisRelDBClient::create_index_inner(statements, self.conn).await
        }
    }

    /// Get a record, return a custom structure / 获取一条记录，返回自定义结构体
    ///
    /// # Arguments
    ///
    ///  * `select_statement` - Statement of the query / 查询的Statement
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_query::*;
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
            TardisRelDBClient::get_dto_inner(select_statement, self.conn).await
        }
    }

    /// Get multiple rows and return a custom structure / 获取多条记录，返回自定义结构体
    ///
    /// # Arguments
    ///
    ///  * `select_statement` - Statement of the query / 查询的Statement
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_query::*;
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
            TardisRelDBClient::find_dtos_inner(select_statement, self.conn).await
        }
    }

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
    /// use tardis::db::sea_query::*;
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
            TardisRelDBClient::paginate_dtos_inner(select_statement, page_number, page_size, self.conn).await
        }
    }

    /// Get number of records / 获取记录数量
    ///
    /// # Arguments
    ///
    ///  * `select_statement` - Statement of the query / 查询的Statement
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_query::*;
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
            TardisRelDBClient::count_inner(select_statement, self.conn).await
        }
    }

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
            TardisRelDBClient::execute_inner(statement, self.conn).await
        }
    }

    /// Insert a record and return primary key value / 插入一条记录，返回主键值
    ///
    /// # Arguments
    ///
    ///  * `model` -  Record to be inserted / 要插入的记录
    ///  * `cxt` -  TardisContext
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_orm::*;
    /// use tardis::db::sea_query::*;
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::reldb_client::TardisActiveModel;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// let resp = conn.insert_one(tardis_db_config::ActiveModel {
    ///     k: Set("ke".to_string()),
    ///     v: Set("ve".to_string()),
    ///     ..Default::default()
    /// },&cxt).await.unwrap();
    /// ```
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

    /// Insert multiple records / 插入多条记录
    ///
    /// # Arguments
    ///
    ///  * `models` -  Set of records to be inserted / 要插入的记录集
    ///  * `cxt` -  TardisContext
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_orm::*;
    /// use tardis::db::sea_query::*;
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
    ///  ],&cxt).await.unwrap();
    /// ```
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

    /// Update a record / 更新一条记录
    ///
    /// # Arguments
    ///
    ///  * `model` -  Records to be inserted / 要插入的记录
    ///  * `cxt` -  TardisContext
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_orm::*;
    /// use tardis::db::sea_query::*;
    /// use tardis::db::domain::tardis_db_config;
    /// use tardis::db::reldb_client::TardisActiveModel;
    /// use tardis::TardisFuns;
    /// let mut conn = TardisFuns::reldb().conn();
    /// let resp = conn.update_one(tardis_db_config::ActiveModel {
    ///     id: Set("111".to_string()),
    ///     k: Set("ke".to_string()),
    ///     v: Set("ve".to_string()),
    ///     ..Default::default()
    /// },&cxt).await.unwrap();
    /// ```
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

    /// Update multiple records / 更新多条记录
    ///
    /// # Arguments
    ///
    ///  * `update_statement` -  Statement to be updated / 要更新的Statement
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_orm::*;
    /// use tardis::db::sea_query::*;
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
    pub async fn update_many<T>(&self, update_statement: &UpdateStatement) -> TardisResult<()> {
        if let Some(tx) = &self.tx {
            TardisRelDBClient::update_many_inner(update_statement, tx).await
        } else {
            TardisRelDBClient::update_many_inner(update_statement, self.conn).await
        }
    }

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
    /// use tardis::db::sea_query::*;
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
            TardisRelDBClient::soft_delete_inner(select, delete_user, self.conn).await
        }
    }

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
    /// use tardis::db::sea_query::*;
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
            TardisRelDBClient::soft_delete_custom_inner(select, custom_pk_field, self.conn).await
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
                "sql parsing error, the name of the table to be soft deleted was not found".to_string(),
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
                    id.as_str()
                        .as_ref()
                        .ok_or_else(|| TardisError::InternalError(format!("[Tardis.RelDBClient] The primary key [{}] in a soft delete operation is not a character type", id)))?
                        .to_string()
                        .into(),
                );
            } else {
                ids.push(
                    id.as_u64()
                        .ok_or_else(|| TardisError::InternalError(format!("[Tardis.RelDBClient] The primary key [{}] in a soft delete operation is not a number type", id)))?
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
                DbBackend::Postgres => format!("DELETE FROM {} WHERE {} in ($1)", table_name, custom_pk_field),
                _ => format!("DELETE FROM {} WHERE {} in (?)", table_name, custom_pk_field),
            }
            .as_str(),
            ids,
        );
        let result = db.execute(statement).await;
        match result {
            Ok(_) => Ok(delete_entities),
            Err(err) => Err(TardisError::from(err)),
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
    ///  * `cxt` -  TardisContext
    ///  * `is_insert` -  whether to insert the operation / 是否插入操作
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::basic::dto::TardisContext;
    /// use tardis::db::sea_orm::*;
    ///
    /// fn fill_cxt(&mut self, cxt: &TardisContext, is_insert: bool) {
    ///     if is_insert {
    ///         self.rel_app_code = Set(cxt.app_code.to_string());
    ///     }
    ///     self.updater_code = Set(cxt.account_code.to_string());
    /// }
    /// ```
    fn fill_cxt(&mut self, cxt: &TardisContext, is_insert: bool);

    /// Create table and index / 创建表和索引
    ///
    /// # Arguments
    ///
    ///  * `db` -  database instance type / 数据库实例类型
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::db::sea_orm::*;
    /// use tardis::db::sea_query::*;
    /// use tardis::basic::dto::TardisContext;
    /// fn create_table_and_index_statement(db: DbBackend) -> (TableCreateStatement, Vec<IndexCreateStatement>) {
    ///     (Self::create_table_statement(db), Self::create_index_statement())
    /// }
    /// ```
    fn create_table_and_index_statement(db: DbBackend) -> (TableCreateStatement, Vec<IndexCreateStatement>) {
        (Self::create_table_statement(db), Self::create_index_statement())
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
    /// use tardis::db::sea_query::*;
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
    /// use tardis::db::sea_query::*;
    /// fn create_index_statement() -> Vec<IndexCreateStatement> {
    ///         vec![
    ///             Index::create()
    ///                 .name(&format!("idx-{}-{}", tardis_db_config::Entity.table_name(),tardis_db_config::Column::K.to_string()))
    ///                 .table(tardis_db_config::Entity)
    ///                 .col(tardis_db_config::Column::K)
    ///                 .to_owned(),
    ///         ]
    ///     }
    fn create_index_statement() -> Vec<IndexCreateStatement> {
        vec![]
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
        TardisError::Box(Box::new(error))
    }
}

impl From<ParserError> for TardisError {
    fn from(error: ParserError) -> Self {
        TardisError::Box(Box::new(error))
    }
}
