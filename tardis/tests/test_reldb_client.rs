// https://github.com/SeaQL/sea-orm

use std::env;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tardis::config::config_dto::{CompatibleType, DBModuleConfig};
use tokio::time::sleep;

use tardis::basic::dto::TardisContext;
use tardis::basic::result::TardisResult;
use tardis::db::domain::{tardis_db_config, tardis_db_del_record};
use tardis::db::reldb_client::TardisSeaORMExtend;
use tardis::db::reldb_client::{TardisActiveModel, TardisRelDBClient};
use tardis::db::sea_orm::sea_query::*;
use tardis::db::sea_orm::*;
use tardis::test::test_container::TardisTestContainer;
use tardis::TardisFuns;
use tracing::info;

use crate::entities::RbumExampleResp;

#[tokio::test(flavor = "multi_thread")]
async fn test_reldb_client() -> TardisResult<()> {
    env::set_var("RUST_LOG", "debug,tardis=trace,sqlx=off,sqlparser::parser=off");
    TardisFuns::init_log();
    TardisTestContainer::mysql(None, |url| async move {
        let db_config = DBModuleConfig::builder().url(&url).max_connections(10).min_connections(5).build();
        let client = TardisRelDBClient::init(&db_config).await?;
        client.init_basic_tables().await?;

        test_basic(&client).await?;
        test_rel(&client).await?;
        test_transaction(&client).await?;
        test_advanced_query(&client).await?;
        test_raw_query(&client).await?;
        test_data_dict(&client).await?;
        test_timezone(&url).await?;
        test_field_type(&client).await?;
        Ok(())
    })
    .await?;
    TardisTestContainer::postgres(None, |url| async move {
        let db_config = DBModuleConfig::builder().url(&url).max_connections(10).min_connections(5).build();
        let client = TardisRelDBClient::init(&db_config).await?;
        client.init_basic_tables().await?;

        test_basic(&client).await?;
        test_rel(&client).await?;
        test_transaction(&client).await?;
        test_advanced_query(&client).await?;
        test_raw_query(&client).await?;
        test_data_dict(&client).await?;
        test_timezone(&url).await?;
        test_field_type(&client).await?;
        Ok(())
    })
    .await
}

async fn test_raw_query(client: &TardisRelDBClient) -> TardisResult<()> {
    let ctx = TardisContext {
        own_paths: "t1/a1".to_string(),
        ak: "ak1".to_string(),
        roles: vec![],
        groups: vec![],
        owner: "acc1".to_string(),
        ext: Default::default(),
        sync_task_fns: Default::default(),
        async_task_fns: Default::default(),
    };

    let db = client.conn();
    let conn = db.raw_conn();

    // Prepare data
    entities::app_account_rel::Entity::delete_many().exec(conn).await?;
    entities::account::Entity::delete_many().exec(conn).await?;
    entities::app::Entity::delete_many().exec(conn).await?;
    entities::tenant_conf::Entity::delete_many().exec(conn).await?;
    entities::tenant::Entity::delete_many().exec(conn).await?;

    db.insert_one(
        entities::tenant::ActiveModel {
            name: Set("tenant1".to_string()),
            ..Default::default()
        },
        &ctx,
    )
    .await?;

    db.insert_one(
        entities::tenant::ActiveModel {
            name: Set("tenant2".to_string()),
            ..Default::default()
        },
        &ctx,
    )
    .await?;

    db.insert_one(
        entities::tenant::ActiveModel {
            name: Set("tenant3".to_string()),
            ..Default::default()
        },
        &ctx,
    )
    .await?;

    #[derive(FromQueryResult)]
    struct TenantResp {
        #[allow(dead_code)]
        id: String,
        name: String,
    }

    let tenant_resp: Option<TenantResp> =
        db.get_dto(Query::select().columns(vec![entities::tenant::Column::Id, entities::tenant::Column::Name]).from(entities::tenant::Entity)).await?;
    assert!(tenant_resp.is_some());
    assert!(tenant_resp.unwrap().name.contains("tenant"));

    let tenant_resp: Vec<TenantResp> =
        db.find_dtos(Query::select().columns(vec![entities::tenant::Column::Id, entities::tenant::Column::Name]).from(entities::tenant::Entity)).await?;
    assert_eq!(tenant_resp.len(), 3);

    let tenant_resp: (Vec<TenantResp>, u64) = db
        .paginate_dtos(
            Query::select().columns(vec![entities::tenant::Column::Id, entities::tenant::Column::Name]).from(entities::tenant::Entity),
            1,
            2,
        )
        .await?;
    assert_eq!(tenant_resp.0.len(), 2);
    assert!(tenant_resp.0.first().unwrap().name.contains("tenant"));
    assert_eq!(tenant_resp.1, 3);

    Ok(())
}

async fn test_advanced_query(client: &TardisRelDBClient) -> TardisResult<()> {
    let ctx = TardisContext {
        own_paths: "t1/a1".to_string(),
        ak: "ak1".to_string(),
        roles: vec![],
        groups: vec![],
        owner: "acc1".to_string(),
        ext: Default::default(),
        sync_task_fns: Default::default(),
        async_task_fns: Default::default(),
    };

    let db = client.conn();
    let conn = db.raw_conn();

    // Prepare data
    entities::app_account_rel::Entity::delete_many().exec(conn).await?;
    entities::account::Entity::delete_many().exec(conn).await?;
    entities::app::Entity::delete_many().exec(conn).await?;
    entities::tenant_conf::Entity::delete_many().exec(conn).await?;
    entities::tenant::Entity::delete_many().exec(conn).await?;

    let tenant_id: String = db
        .insert_one(
            entities::tenant::ActiveModel {
                name: Set("tenant1".to_string()),
                ..Default::default()
            },
            &ctx,
        )
        .await?
        .last_insert_id;

    db.insert_one(
        entities::app::ActiveModel {
            name: Set("app1".to_string()),
            tenant_id: Set(tenant_id.clone()),
            ..Default::default()
        },
        &ctx,
    )
    .await?;

    let app_id: String = db
        .insert_one(
            entities::app::ActiveModel {
                name: Set("app2".to_string()),
                tenant_id: Set(tenant_id.clone()),
                ..Default::default()
            },
            &ctx,
        )
        .await?
        .last_insert_id;

    let account_id: String = db
        .insert_one(
            entities::account::ActiveModel {
                name: Set("account1".to_string()),
                ..Default::default()
            },
            &ctx,
        )
        .await?
        .last_insert_id;

    entities::app_account_rel::ActiveModel {
        app_id: Set(app_id.clone()),
        account_id: Set(account_id.clone()),
    }
    .insert(conn)
    .await?;

    // Select to DTO
    #[derive(Debug, FromQueryResult)]
    struct SelectResult {
        name: String,
        aa_id: String,
    }

    let select_result = entities::tenant::Entity::find()
        .select_only()
        .column(entities::tenant::Column::Name)
        .column_as(entities::tenant::Column::Id, "aa_id")
        .into_model::<SelectResult>()
        .one(conn)
        .await?
        .unwrap();
    assert_eq!(select_result.aa_id, tenant_id);
    assert_eq!(select_result.name, "tenant1");

    // AND Condition
    let apps = entities::app::Entity::find().filter(Condition::all().add(entities::app::Column::Id.eq("__")).add(entities::app::Column::Name.like("%app%"))).all(conn).await?;
    assert_eq!(apps.len(), 0);

    // OR Condition
    let apps = entities::app::Entity::find().filter(Condition::any().add(entities::app::Column::Id.eq("__")).add(entities::app::Column::Name.like("%app%"))).all(conn).await?;
    assert_eq!(apps.len(), 2);

    // Group By
    let apps = entities::app::Entity::find()
        .select_only()
        .column(entities::app::Column::Name)
        .column_as(entities::app::Column::Id.count(), "count")
        .group_by(entities::app::Column::Name)
        .into_json()
        .all(conn)
        .await?;
    assert_eq!(apps[0]["count"], 1);

    // Join
    let tenants = entities::tenant::Entity::find()
        .select_only()
        .column(entities::tenant::Column::Name)
        .column_as(entities::tenant_conf::Column::Name, "conf_name")
        .left_join(entities::tenant_conf::Entity)
        .into_json()
        .all(conn)
        .await?;
    assert_eq!(tenants.len(), 1);
    let tenants = entities::tenant::Entity::find()
        .select_only()
        .column(entities::tenant::Column::Name)
        .column_as(entities::tenant_conf::Column::Name, "conf_name")
        .inner_join(entities::tenant_conf::Entity)
        .into_json()
        .all(conn)
        .await?;
    assert_eq!(tenants.len(), 0);

    let apps = entities::app::Entity::find()
        .select_only()
        .column(entities::app::Column::Name)
        .column_as(entities::tenant::Column::Name, "tenant_name")
        .left_join(entities::tenant::Entity)
        .filter(entities::tenant::Column::Name.contains("tenant"))
        .into_json()
        .all(conn)
        .await?;
    assert_eq!(apps.len(), 2);
    assert_eq!(apps[0]["tenant_name"], "tenant1");

    Ok(())
}

async fn test_transaction(client: &TardisRelDBClient) -> TardisResult<()> {
    // Normal transaction
    let mut db = client.conn();
    db.begin().await?;

    let config = tardis_db_config::ActiveModel {
        k: Set("kn".to_string()),
        v: Set("vn".to_string()),
        creator: Set("admin".to_string()),
        updater: Set("admin".to_string()),
        ..Default::default()
    }
    .insert(db.raw_tx().unwrap())
    .await?;

    let conf = tardis_db_config::Entity::find_by_id(config.k.clone()).one(db.raw_conn()).await?;
    assert_eq!(conf, None);
    let conf = tardis_db_config::Entity::find_by_id(config.k.clone()).one(db.raw_tx().unwrap()).await?.unwrap();
    assert_eq!(conf.k, "kn");

    db.commit().await?;

    let conf = tardis_db_config::Entity::find_by_id(config.k.clone()).one(client.conn().raw_conn()).await?.unwrap();
    assert_eq!(conf.k, "kn");

    // Rollback transaction

    let mut db = client.conn();
    db.begin().await?;

    let config = tardis_db_config::ActiveModel {
        k: Set("ke".to_string()),
        v: Set("ve".to_string()),
        creator: Set("admin".to_string()),
        updater: Set("admin".to_string()),
        ..Default::default()
    }
    .insert(db.raw_tx().unwrap())
    .await?;

    db.rollback().await?;

    let conf = tardis_db_config::Entity::find_by_id(config.k.clone()).one(client.conn().raw_conn()).await?;
    assert_eq!(conf, None);

    Ok(())
}

async fn test_rel(client: &TardisRelDBClient) -> TardisResult<()> {
    let mut db = client.conn();
    db.begin().await?;
    db.create_table_from_entity(entities::tenant::Entity).await?;
    db.create_table_from_entity(entities::tenant_conf::Entity).await?;
    db.create_table_from_entity(entities::app::Entity).await?;
    db.create_table_from_entity(entities::account::Entity).await?;
    db.create_table_from_entity(entities::app_account_rel::Entity).await?;
    db.commit().await?;

    let db = client.conn();

    let ctx = TardisContext {
        own_paths: "t1/a1".to_string(),
        ak: "ak1".to_string(),
        roles: vec![],
        groups: vec![],
        owner: "acc1".to_string(),
        ext: Default::default(),
        sync_task_fns: Default::default(),
        async_task_fns: Default::default(),
    };

    db.insert_one(
        entities::tenant::ActiveModel {
            name: Set("tenant1".to_string()),
            ..Default::default()
        },
        &ctx,
    )
    .await?;

    let tenant = entities::tenant::Entity::find().one(db.raw_conn()).await?.unwrap();
    let config = tenant.find_related(entities::tenant_conf::Entity).one(db.raw_conn()).await?;
    // Not Exists
    assert_eq!(config, None);

    db.insert_one(
        entities::tenant_conf::ActiveModel {
            name: Set("conf1".to_string()),
            tenant_id: Set(tenant.id.clone()),
            ..Default::default()
        },
        &ctx,
    )
    .await?;

    db.insert_many(
        vec![
            entities::app::ActiveModel {
                name: Set("app1".to_string()),
                tenant_id: Set(tenant.id.clone()),
                ..Default::default()
            },
            entities::app::ActiveModel {
                name: Set("app2".to_string()),
                tenant_id: Set(tenant.id.clone()),
                ..Default::default()
            },
        ],
        &ctx,
    )
    .await?;

    let tenant = entities::tenant::Entity::find_by_id(tenant.id.clone()).one(db.raw_conn()).await?.unwrap();

    info!("----------------- One To One -----------------");
    let config = tenant.find_related(entities::tenant_conf::Entity).one(db.raw_conn()).await?.unwrap();
    assert_eq!(config.name, "conf1");
    let tenant = config.find_related(entities::tenant::Entity).one(db.raw_conn()).await?.unwrap();
    assert_eq!(tenant.name, "tenant1");

    info!("----------------- One To Many -----------------");
    let apps = tenant.find_related(entities::app::Entity).all(db.raw_conn()).await?;
    assert_eq!(apps.len(), 2);
    info!("----------------- Many To One -----------------");
    let tenant = apps[0].find_related(entities::tenant::Entity).one(db.raw_conn()).await?.unwrap();
    assert_eq!(tenant.name, "tenant1");

    info!("----------------- Many To Many -----------------");
    let accounts = apps[0].find_related(entities::account::Entity).all(db.raw_conn()).await?;
    assert_eq!(accounts.len(), 0);

    let account_id: String = db
        .insert_one(
            entities::account::ActiveModel {
                name: Set("account1".to_string()),
                ..Default::default()
            },
            &ctx,
        )
        .await?
        .last_insert_id;

    entities::app_account_rel::ActiveModel {
        app_id: Set(apps[0].id.to_string()),
        account_id: Set(account_id.clone()),
    }
    .insert(db.raw_conn())
    .await?;

    let accounts = apps[0].find_related(entities::account::Entity).all(db.raw_conn()).await?;
    assert_eq!(accounts.len(), 1);

    Ok(())
}

async fn test_basic(client: &TardisRelDBClient) -> TardisResult<()> {
    let db = client.conn();
    // Insert
    tardis_db_config::ActiveModel {
        k: Set("k1".to_string()),
        v: Set("v1".to_string()),
        creator: Set("admin".to_string()),
        updater: Set("admin".to_string()),
        ..Default::default()
    }
    .insert(db.raw_conn())
    .await?;

    let conf2 = tardis_db_config::ActiveModel {
        k: Set("k2".to_string()),
        v: Set("v2".to_string()),
        creator: Set("admin".to_string()),
        updater: Set("admin".to_string()),
        ..Default::default()
    };
    let insert_result = tardis_db_config::Entity::insert(conf2).exec(db.raw_conn()).await?;

    // Find One
    let conf2 = tardis_db_config::Entity::find_by_id(insert_result.last_insert_id.clone()).one(db.raw_conn()).await?.unwrap();
    assert_eq!(conf2.k, "k2");
    assert_eq!(conf2.create_time, conf2.update_time);

    // Update One
    sleep(Duration::from_millis(1100)).await;
    let mut conf2: tardis_db_config::ActiveModel = conf2.into();
    conf2.v = Set("v2更新".to_string());
    conf2.update(db.raw_conn()).await?;
    let conf2 = tardis_db_config::Entity::find_by_id(insert_result.last_insert_id.clone()).one(db.raw_conn()).await?.unwrap();
    assert_eq!(conf2.v, "v2更新");
    assert_ne!(conf2.create_time, conf2.update_time);

    // Update Many
    tardis_db_config::Entity::update_many()
        .col_expr(tardis_db_config::Column::V, Expr::value("v1更新"))
        .filter(tardis_db_config::Column::K.ne(insert_result.last_insert_id))
        .exec(db.raw_conn())
        .await?;

    // Find Many
    let confs = tardis_db_config::Entity::find().filter(tardis_db_config::Column::K.contains("k")).order_by_desc(tardis_db_config::Column::K).all(db.raw_conn()).await?;
    assert_eq!(confs.len(), 2);
    assert_eq!(confs[0].k, "k2");
    assert_eq!(confs[1].k, "k1");
    assert_eq!(confs[0].v, "v2更新");
    assert_eq!(confs[1].v, "v1更新");

    // Page
    let conf_page = tardis_db_config::Entity::find().filter(tardis_db_config::Column::K.contains("k1")).order_by_desc(tardis_db_config::Column::K).paginate(db.raw_conn(), 1);
    assert_eq!(conf_page.num_pages().await.unwrap(), 1);
    assert_eq!(conf_page.cur_page(), 0);
    let confs = conf_page.fetch_page(0).await?;
    assert_eq!(confs.len(), 1);
    assert_eq!(confs[0].k, "k1");
    assert_eq!(confs[0].v, "v1更新");

    // Exists TODO https://github.com/SeaQL/sea-orm/issues/408

    // Soft Delete
    tardis_db_config::Entity::find().soft_delete_with_pk("k", "admin", db.raw_conn()).await?;
    let dels = tardis_db_del_record::Entity::find().all(db.raw_conn()).await?;
    assert_eq!(dels.len(), 2);
    assert_eq!(dels[0].entity_name, "tardis_config");
    let vals = TardisFuns::dict.find_all(&db).await?;
    assert_eq!(vals.len(), 0);

    // Delete
    let delete_result = tardis_db_del_record::Entity::delete_many().filter(tardis_db_del_record::Column::Id.eq(dels[0].id.clone())).exec(db.raw_conn()).await?;
    assert_eq!(delete_result.rows_affected, 1);

    // Count
    let count = tardis_db_del_record::Entity::find().count(db.raw_conn()).await?;
    assert_eq!(count, 1);

    Ok(())
}

async fn test_data_dict(client: &TardisRelDBClient) -> TardisResult<()> {
    let db = client.conn();
    assert!(TardisFuns::dict.get("xxx", &db).await?.is_none());

    TardisFuns::dict.add("xxx", "yyyy", "admin", &db).await?;
    assert_eq!(TardisFuns::dict.get("xxx", &db).await?.unwrap().v, "yyyy");

    TardisFuns::dict.update("xxx", "zzzz", "admin", &db).await?;
    assert_eq!(TardisFuns::dict.get("xxx", &db).await?.unwrap().v, "zzzz");

    TardisFuns::dict.delete("xxx", &db).await?;
    assert!(TardisFuns::dict.get("xxx", &db).await?.is_none());

    assert!(TardisFuns::dict.update("xxx111", "zzzz", "admin", &db).await.is_err());

    TardisFuns::dict.add("t1:xx", "1", "", &db).await?;
    TardisFuns::dict.add("t1:yy", "2", "", &db).await?;
    TardisFuns::dict.add("t2:zz", "3", "", &db).await?;
    let vals = TardisFuns::dict.find_all(&db).await?;
    assert_eq!(vals.len(), 4);
    let vals = TardisFuns::dict.find_like("t1", &db).await?;
    assert_eq!(vals.len(), 2);
    assert_eq!(vals[0].k, "t1:xx");
    assert_eq!(vals[0].v, "1");
    assert_eq!(vals[1].k, "t1:yy");
    assert_eq!(vals[1].v, "2");

    Ok(())
}

async fn test_timezone(url: &str) -> TardisResult<()> {
    let mut db_config = DBModuleConfig::builder().url(url).max_connections(10).min_connections(5).build();

    let client_with_out_time_zone = TardisRelDBClient::init(&db_config).await?;

    match client_with_out_time_zone.backend() {
        DatabaseBackend::Postgres => {
            db_config.url = format!("{url}?timezone=Asia/Shanghai");
            let client_with_time_zone = TardisRelDBClient::init(&db_config).await?;

            let tz = client_with_out_time_zone.conn().query_one("SHOW timezone", Vec::new()).await?.unwrap().try_get::<String>("", "TimeZone")?;
            assert_eq!(tz, "UTC");

            let tz = client_with_time_zone.conn().query_one("SHOW timezone", Vec::new()).await?.unwrap().try_get::<String>("", "TimeZone")?;
            assert_eq!(tz, "Asia/Shanghai");

            let now1 = client_with_out_time_zone.conn().query_one("SELECT CURRENT_TIMESTAMP AS now", Vec::new()).await?.unwrap().try_get::<DateTime<Utc>>("", "now")?;
            let now2 = client_with_time_zone.conn().query_one("SELECT CURRENT_TIMESTAMP AS now", Vec::new()).await?.unwrap().try_get::<DateTime<Utc>>("", "now")?;

            println!("client_with_out_time_zone：{},client_with_time_zone：{},", now1, now2);
        }
        _ => {
            db_config.url = format!("{url}?timezone=%2B08:00");
            let client_with_time_zone = TardisRelDBClient::init(&db_config).await?;

            let tz = client_with_out_time_zone.conn().query_one("SELECT @@global.time_zone z1, @@session.time_zone z2", Vec::new()).await?.unwrap().try_get::<String>("", "z2")?;
            assert_eq!(tz, "+00:00");

            let tz = client_with_time_zone.conn().query_one("SELECT @@global.time_zone z1, @@session.time_zone z2", Vec::new()).await?.unwrap().try_get::<String>("", "z2")?;
            assert_eq!(tz, "+08:00");

            let now1 = client_with_out_time_zone.conn().query_one("SELECT CURRENT_TIMESTAMP() AS now", Vec::new()).await?.unwrap().try_get::<DateTime<Utc>>("", "now")?;
            let now2 = client_with_time_zone.conn().query_one("SELECT CURRENT_TIMESTAMP() AS now", Vec::new()).await?.unwrap().try_get::<DateTime<Utc>>("", "now")?;

            // Mysql's timestamp does not store the time zone, so there is an error when converting to utc time zone
            println!("client_with_out_time_zone：{},client_with_time_zone：{},", now1, now2);
        }
    }

    Ok(())
}

async fn test_field_type(client: &TardisRelDBClient) -> TardisResult<()> {
    // https://www.sea-ql.org/SeaORM/docs/generate-entity/entity-structure/#column-type
    let ctx = TardisContext {
        own_paths: "t1/a1".to_string(),
        ak: "ak1".to_string(),
        roles: vec![],
        groups: vec![],
        owner: "acc1".to_string(),
        ext: Default::default(),
        sync_task_fns: Default::default(),
        async_task_fns: Default::default(),
    };

    let mut conn = client.conn();
    conn.init(entities::rbum_example::ActiveModel::init(client.backend(), Some("update_time"), CompatibleType::None)).await?;

    conn.begin().await?;
    let insert_result = conn
        .insert_one(
            entities::rbum_example::ActiveModel {
                name: Set("sunisle".to_string()),
                sort: Set(100),
                status: Set(1),
                scope_level: Set(0),
                ..Default::default()
            },
            &ctx,
        )
        .await?;

    let dto = conn
        .get_dto::<RbumExampleResp>(
            Query::select()
                .columns(vec![
                    entities::rbum_example::Column::Id,
                    entities::rbum_example::Column::Name,
                    entities::rbum_example::Column::Sort,
                    entities::rbum_example::Column::Status,
                    entities::rbum_example::Column::OwnPaths,
                    entities::rbum_example::Column::ScopeLevel,
                    entities::rbum_example::Column::CreateTime,
                    entities::rbum_example::Column::UpdateTime,
                ])
                .from(entities::rbum_example::Entity)
                .and_where(Expr::col(entities::rbum_example::Column::Id).eq(insert_result.last_insert_id)),
        )
        .await?
        .unwrap();

    assert_eq!(dto.name, "sunisle");
    assert_eq!(dto.sort, 100);
    assert_eq!(dto.status, 1);
    assert_eq!(dto.own_paths, "t1/a1");
    assert_eq!(dto.scope_level, 0);
    conn.commit().await?;

    Ok(())
}

pub mod entities {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};

    pub mod tenant {
        use sea_orm::entity::prelude::*;
        use sea_orm::ActiveModelBehavior;
        use sea_orm::ActiveValue::Set;

        use tardis::basic::dto::TardisContext;
        use tardis::db::reldb_client::TardisActiveModel;
        use tardis::TardisFuns;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "test_tenant")]
        pub struct Model {
            #[sea_orm(primary_key, auto_increment = false)]
            pub id: String,
            pub name: String,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {
            #[sea_orm(has_one = "super::tenant_conf::Entity")]
            TenantConfig,
            #[sea_orm(has_many = "super::app::Entity")]
            App,
        }

        impl Related<super::tenant_conf::Entity> for super::tenant::Entity {
            fn to() -> RelationDef {
                Relation::TenantConfig.def()
            }
        }

        impl Related<super::app::Entity> for super::tenant::Entity {
            fn to() -> RelationDef {
                Relation::App.def()
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
    }

    pub mod tenant_conf {
        use sea_orm::entity::prelude::*;
        use sea_orm::ActiveModelBehavior;
        use sea_orm::ActiveValue::Set;

        use tardis::basic::dto::TardisContext;
        use tardis::db::reldb_client::TardisActiveModel;
        use tardis::TardisFuns;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "test_tenant_conf")]
        pub struct Model {
            #[sea_orm(primary_key, auto_increment = false)]
            pub id: String,
            pub name: String,
            pub tenant_id: String,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {
            #[sea_orm(belongs_to = "super::tenant::Entity", from = "Column::TenantId", to = "super::tenant::Column::Id")]
            Tenant,
        }

        impl Related<super::tenant::Entity> for super::tenant_conf::Entity {
            fn to() -> RelationDef {
                Relation::Tenant.def()
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
    }

    pub mod app {
        use sea_orm::entity::prelude::*;
        use sea_orm::ActiveModelBehavior;
        use sea_orm::ActiveValue::Set;

        use tardis::basic::dto::TardisContext;
        use tardis::db::reldb_client::TardisActiveModel;
        use tardis::TardisFuns;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "test_app")]
        pub struct Model {
            #[sea_orm(primary_key, auto_increment = false)]
            pub id: String,
            pub name: String,
            pub tenant_id: String,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {
            #[sea_orm(belongs_to = "super::tenant::Entity", from = "Column::TenantId", to = "super::tenant::Column::Id")]
            Tenant,
        }

        impl Related<super::tenant::Entity> for super::app::Entity {
            fn to() -> RelationDef {
                Relation::Tenant.def()
            }
        }

        impl Related<super::account::Entity> for super::app::Entity {
            fn to() -> RelationDef {
                super::app_account_rel::Relation::Account.def()
            }

            fn via() -> Option<RelationDef> {
                Some(super::app_account_rel::Relation::App.def().rev())
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
    }

    pub mod account {
        use sea_orm::entity::prelude::*;
        use sea_orm::ActiveModelBehavior;
        use sea_orm::ActiveValue::Set;

        use tardis::basic::dto::TardisContext;
        use tardis::db::reldb_client::TardisActiveModel;
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

        impl TardisActiveModel for ActiveModel {
            fn fill_ctx(&mut self, _: &TardisContext, is_insert: bool) {
                if is_insert {
                    self.id = Set(TardisFuns::field.nanoid());
                }
            }
        }

        impl ActiveModelBehavior for ActiveModel {}
    }

    pub mod app_account_rel {
        use sea_orm::entity::prelude::*;
        use sea_orm::ActiveModelBehavior;

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
    }

    pub mod rbum_example {
        use tardis::basic::dto::TardisContext;
        use tardis::chrono::{self, Utc};
        use tardis::db::reldb_client::TardisActiveModel;
        use tardis::db::sea_orm;
        use tardis::db::sea_orm::prelude::*;
        use tardis::db::sea_orm::sea_query::{ColumnDef, Index, IndexCreateStatement, Table, TableCreateStatement};
        use tardis::db::sea_orm::*;
        use tardis::TardisFuns;

        #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
        #[sea_orm(table_name = "rbum_example")]
        pub struct Model {
            #[sea_orm(primary_key, auto_increment = false)]
            pub id: String,
            pub name: String,
            pub sort: i64,
            pub own_paths: String,
            pub scope_level: i16,
            pub status: i16,
            pub create_time: chrono::DateTime<Utc>,
            pub update_time: chrono::DateTime<Utc>,
        }

        impl TardisActiveModel for ActiveModel {
            fn fill_ctx(&mut self, ctx: &TardisContext, is_insert: bool) {
                if is_insert {
                    self.id = Set(TardisFuns::field.nanoid());
                    self.own_paths = Set(ctx.own_paths.to_string());
                }
            }

            fn create_table_statement(db: DbBackend) -> TableCreateStatement {
                let mut builder = Table::create();
                builder
                    .table(Entity.table_ref())
                    .if_not_exists()
                    .col(ColumnDef::new(Column::Id).not_null().string().primary_key())
                    .col(ColumnDef::new(Column::Name).not_null().string())
                    .col(ColumnDef::new(Column::Sort).not_null().big_integer())
                    .col(ColumnDef::new(Column::OwnPaths).not_null().string())
                    .col(ColumnDef::new(Column::ScopeLevel).not_null().small_integer())
                    .col(ColumnDef::new(Column::Status).not_null().small_integer());
                if db == DatabaseBackend::Postgres {
                    builder
                        .col(ColumnDef::new(Column::CreateTime).extra("DEFAULT CURRENT_TIMESTAMP".to_string()).timestamp_with_time_zone())
                        .col(ColumnDef::new(Column::UpdateTime).extra("DEFAULT CURRENT_TIMESTAMP".to_string()).timestamp_with_time_zone());
                } else {
                    builder
                        .engine("InnoDB")
                        .character_set("utf8mb4")
                        .collate("utf8mb4_0900_as_cs")
                        .col(ColumnDef::new(Column::CreateTime).extra("DEFAULT CURRENT_TIMESTAMP".to_string()).timestamp())
                        .col(ColumnDef::new(Column::UpdateTime).extra("DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP".to_string()).timestamp());
                }
                builder.to_owned()
            }

            fn create_index_statement() -> Vec<IndexCreateStatement> {
                vec![Index::create().name(format!("idx-{}-{}", Entity.table_name(), Column::OwnPaths.to_string())).table(Entity).col(Column::OwnPaths).to_owned()]
            }
        }

        impl ActiveModelBehavior for ActiveModel {}

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}
    }

    #[derive(Serialize, Deserialize, sea_orm::FromQueryResult, Debug)]
    pub struct RbumExampleResp {
        pub id: String,
        pub name: String,
        pub sort: i64,
        pub status: i16,
        pub own_paths: String,
        pub scope_level: i16,
        pub create_time: DateTime<Utc>,
        pub update_time: DateTime<Utc>,
    }
}
