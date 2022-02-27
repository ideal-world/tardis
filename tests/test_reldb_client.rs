// https://github.com/SeaQL/sea-orm

use std::time::Duration;

use tokio::time::sleep;

use tardis::basic::dto::TardisContext;
use tardis::basic::result::TardisResult;
use tardis::db::domain::{tardis_db_config, tardis_db_del_record};
use tardis::db::reldb_client::TardisSeaORMExtend;
use tardis::db::reldb_client::{TardisActiveModel, TardisRelDBClient};
use tardis::db::sea_orm::*;
use tardis::db::sea_query::*;
use tardis::log::info;
use tardis::test::test_container::TardisTestContainer;
use tardis::TardisFuns;

#[tokio::test]
async fn test_reldb_client() -> TardisResult<()> {
    TardisFuns::init_log()?;
    TardisTestContainer::mysql(None, |url| async move {
        let client = TardisRelDBClient::init(&url, 10, 5, None, None).await?;

        test_basic(&client).await?;
        test_rel(&client).await?;
        test_transaction(&client).await?;
        test_advanced_query(&client).await?;
        test_raw_query(&client).await?;
        Ok(())
    })
    .await
}

async fn test_raw_query(client: &TardisRelDBClient) -> TardisResult<()> {
    let cxt = TardisContext {
        app_id: "a1".to_string(),
        tenant_id: "t1".to_string(),
        ak: "ak1".to_string(),
        account_id: "acc1".to_string(),
        token: "token1".to_string(),
        token_kind: "default".to_string(),
        roles: vec![],
        groups: vec![],
    };

    // Prepare data
    entities::app_account_rel::Entity::delete_many().exec(client.conn()).await?;
    entities::account::Entity::delete_many().exec(client.conn()).await?;
    entities::app::Entity::delete_many().exec(client.conn()).await?;
    entities::tenant_conf::Entity::delete_many().exec(client.conn()).await?;
    entities::tenant::Entity::delete_many().exec(client.conn()).await?;

    entities::tenant::ActiveModel {
        name: Set("tenant1".to_string()),
        ..Default::default()
    }
    .insert_cust(client.conn(), &cxt)
    .await?;
    entities::tenant::ActiveModel {
        name: Set("tenant2".to_string()),
        ..Default::default()
    }
    .insert_cust(client.conn(), &cxt)
    .await?;
    entities::tenant::ActiveModel {
        name: Set("tenant3".to_string()),
        ..Default::default()
    }
    .insert_cust(client.conn(), &cxt)
    .await?;

    #[derive(Debug, FromQueryResult)]
    struct TenantResp {
        id: String,
        name: String,
    }

    let tenant_resp: Option<TenantResp> = client
        .get_dto(
            Query::select().columns(vec![entities::tenant::Column::Id, entities::tenant::Column::Name]).from(entities::tenant::Entity),
            client.conn(),
        )
        .await?;
    assert!(tenant_resp.is_some());
    assert!(tenant_resp.unwrap().name.contains("tenant"));

    let tenant_resp: Vec<TenantResp> = client
        .find_dtos(
            Query::select().columns(vec![entities::tenant::Column::Id, entities::tenant::Column::Name]).from(entities::tenant::Entity),
            client.conn(),
        )
        .await?;
    assert_eq!(tenant_resp.len(), 3);

    let tenant_resp: (Vec<TenantResp>, i64) = client
        .paginate_dtos(
            Query::select().columns(vec![entities::tenant::Column::Id, entities::tenant::Column::Name]).from(entities::tenant::Entity),
            1,
            2,
            client.conn(),
        )
        .await?;
    assert_eq!(tenant_resp.0.len(), 2);
    assert!(tenant_resp.0.get(0).unwrap().name.contains("tenant"));
    assert_eq!(tenant_resp.1, 3);

    Ok(())
}

async fn test_advanced_query(client: &TardisRelDBClient) -> TardisResult<()> {
    let cxt = TardisContext {
        app_id: "a1".to_string(),
        tenant_id: "t1".to_string(),
        ak: "ak1".to_string(),
        account_id: "acc1".to_string(),
        token: "token1".to_string(),
        token_kind: "default".to_string(),
        roles: vec![],
        groups: vec![],
    };

    // Prepare data
    entities::app_account_rel::Entity::delete_many().exec(client.conn()).await?;
    entities::account::Entity::delete_many().exec(client.conn()).await?;
    entities::app::Entity::delete_many().exec(client.conn()).await?;
    entities::tenant_conf::Entity::delete_many().exec(client.conn()).await?;
    entities::tenant::Entity::delete_many().exec(client.conn()).await?;

    let tenant = entities::tenant::ActiveModel {
        name: Set("tenant1".to_string()),
        ..Default::default()
    }
    .insert_cust(client.conn(), &cxt)
    .await?;

    entities::app::ActiveModel {
        name: Set("app1".to_string()),
        tenant_id: Set(tenant.id.clone()),
        ..Default::default()
    }
    .insert_cust(client.conn(), &cxt)
    .await?;

    let app = entities::app::ActiveModel {
        name: Set("app2".to_string()),
        tenant_id: Set(tenant.id.clone()),
        ..Default::default()
    }
    .insert_cust(client.conn(), &cxt)
    .await?;

    let account = entities::account::ActiveModel {
        name: Set("account1".to_string()),
        ..Default::default()
    }
    .insert_cust(client.conn(), &cxt)
    .await?;
    entities::app_account_rel::ActiveModel {
        app_id: Set(app.id.to_string()),
        account_id: Set(account.id.to_string()),
    }
    .insert(client.conn())
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
        .one(client.conn())
        .await?
        .unwrap();
    assert_eq!(select_result.aa_id, tenant.id);
    assert_eq!(select_result.name, "tenant1");

    // AND Condition
    let apps =
        entities::app::Entity::find().filter(Condition::all().add(entities::app::Column::Id.eq("__")).add(entities::app::Column::Name.like("%app%"))).all(client.conn()).await?;
    assert_eq!(apps.len(), 0);

    // OR Condition
    let apps =
        entities::app::Entity::find().filter(Condition::any().add(entities::app::Column::Id.eq("__")).add(entities::app::Column::Name.like("%app%"))).all(client.conn()).await?;
    assert_eq!(apps.len(), 2);

    // Group By
    let apps = entities::app::Entity::find()
        .select_only()
        .column(entities::app::Column::Name)
        .column_as(entities::app::Column::Id.count(), "count")
        .group_by(entities::app::Column::Name)
        .into_json()
        .all(client.conn())
        .await?;
    assert_eq!(apps[0]["count"], 1);

    // Join
    let tenants = entities::tenant::Entity::find()
        .select_only()
        .column(entities::tenant::Column::Name)
        .column_as(entities::tenant_conf::Column::Name, "conf_name")
        .left_join(entities::tenant_conf::Entity)
        .into_json()
        .all(client.conn())
        .await?;
    assert_eq!(tenants.len(), 1);
    let tenants = entities::tenant::Entity::find()
        .select_only()
        .column(entities::tenant::Column::Name)
        .column_as(entities::tenant_conf::Column::Name, "conf_name")
        .inner_join(entities::tenant_conf::Entity)
        .into_json()
        .all(client.conn())
        .await?;
    assert_eq!(tenants.len(), 0);

    let apps = entities::app::Entity::find()
        .select_only()
        .column(entities::app::Column::Name)
        .column_as(entities::tenant::Column::Name, "tenant_name")
        .left_join(entities::tenant::Entity)
        .filter(entities::tenant::Column::Name.contains("tenant"))
        .into_json()
        .all(client.conn())
        .await?;
    assert_eq!(apps.len(), 2);
    assert_eq!(apps[0]["tenant_name"], "tenant1");

    let apps = entities::app::Entity::find()
        .select_only()
        .column(entities::app::Column::Name)
        .column_as(entities::tenant::Column::Name, "tenant_name")
        .join(
            JoinType::LeftJoin,
            // construct `RelationDef` on the fly
            entities::app::Entity::belongs_to(entities::tenant::Entity).from(entities::app::Column::TenantId).to(entities::tenant::Column::Id).into(),
        )
        .filter(entities::tenant::Column::Name.contains("tenant"))
        .into_json()
        .all(client.conn())
        .await?;
    assert_eq!(apps.len(), 2);
    assert_eq!(apps[0]["tenant_name"], "tenant1");

    Ok(())
}

async fn test_transaction(client: &TardisRelDBClient) -> TardisResult<()> {
    // Normal transaction
    let tx = client.conn().begin().await?;

    let config = tardis_db_config::ActiveModel {
        k: Set("kn".to_string()),
        v: Set("vn".to_string()),
        creator: Set("admin".to_string()),
        updater: Set("admin".to_string()),
        ..Default::default()
    }
    .insert(&tx)
    .await?;

    let conf = tardis_db_config::Entity::find_by_id(config.id.clone()).one(client.conn()).await?;
    assert_eq!(conf, None);
    let conf = tardis_db_config::Entity::find_by_id(config.id.clone()).one(&tx).await?.unwrap();
    assert_eq!(conf.k, "kn");

    tx.commit().await?;

    let conf = tardis_db_config::Entity::find_by_id(config.id.clone()).one(client.conn()).await?.unwrap();
    assert_eq!(conf.k, "kn");

    // Rollback transaction

    let tx = client.conn().begin().await?;

    let config = tardis_db_config::ActiveModel {
        k: Set("ke".to_string()),
        v: Set("ve".to_string()),
        creator: Set("admin".to_string()),
        updater: Set("admin".to_string()),
        ..Default::default()
    }
    .insert(&tx)
    .await?;

    tx.rollback().await?;

    let conf = tardis_db_config::Entity::find_by_id(config.id.clone()).one(client.conn()).await?;
    assert_eq!(conf, None);

    Ok(())
}

async fn test_rel(client: &TardisRelDBClient) -> TardisResult<()> {
    client.create_table_from_entity(entities::tenant::Entity).await?;
    client.create_table_from_entity(entities::tenant_conf::Entity).await?;
    client.create_table_from_entity(entities::app::Entity).await?;
    client.create_table_from_entity(entities::account::Entity).await?;
    client.create_table_from_entity(entities::app_account_rel::Entity).await?;

    let cxt = TardisContext {
        app_id: "a1".to_string(),
        tenant_id: "t1".to_string(),
        ak: "ak1".to_string(),
        account_id: "acc1".to_string(),
        token: "token1".to_string(),
        token_kind: "default".to_string(),
        roles: vec![],
        groups: vec![],
    };

    entities::tenant::ActiveModel {
        name: Set("tenant1".to_string()),
        ..Default::default()
    }
    .insert_cust(client.conn(), &cxt)
    .await?;

    let tenant = entities::tenant::Entity::find().one(client.conn()).await?.unwrap();
    let config = tenant.find_related(entities::tenant_conf::Entity).one(client.conn()).await?;
    // Not Exists
    assert_eq!(config, None);

    entities::tenant_conf::ActiveModel {
        name: Set("conf1".to_string()),
        tenant_id: Set(tenant.id.clone()),
        ..Default::default()
    }
    .insert_cust(client.conn(), &cxt)
    .await?;

    entities::app::ActiveModel {
        name: Set("app1".to_string()),
        tenant_id: Set(tenant.id.clone()),
        ..Default::default()
    }
    .insert_cust(client.conn(), &cxt)
    .await?;

    entities::app::ActiveModel {
        name: Set("app2".to_string()),
        tenant_id: Set(tenant.id.clone()),
        ..Default::default()
    }
    .insert_cust(client.conn(), &cxt)
    .await?;

    let tenant = entities::tenant::Entity::find_by_id(tenant.id.clone()).one(client.conn()).await?.unwrap();

    info!("----------------- One To One -----------------");
    let config = tenant.find_related(entities::tenant_conf::Entity).one(client.conn()).await?.unwrap();
    assert_eq!(config.name, "conf1");
    let tenant = config.find_related(entities::tenant::Entity).one(client.conn()).await?.unwrap();
    assert_eq!(tenant.name, "tenant1");

    info!("----------------- One To Many -----------------");
    let apps = tenant.find_related(entities::app::Entity).all(client.conn()).await?;
    assert_eq!(apps.len(), 2);
    info!("----------------- Many To One -----------------");
    let tenant = apps[0].find_related(entities::tenant::Entity).one(client.conn()).await?.unwrap();
    assert_eq!(tenant.name, "tenant1");

    info!("----------------- Many To Many -----------------");
    let accounts = apps[0].find_related(entities::account::Entity).all(client.conn()).await?;
    assert_eq!(accounts.len(), 0);

    let account = entities::account::ActiveModel {
        name: Set("account1".to_string()),
        ..Default::default()
    }
    .insert_cust(client.conn(), &cxt)
    .await?;
    entities::app_account_rel::ActiveModel {
        app_id: Set(apps[0].id.to_string()),
        account_id: Set(account.id.to_string()),
    }
    .insert(client.conn())
    .await?;

    let accounts = apps[0].find_related(entities::account::Entity).all(client.conn()).await?;
    assert_eq!(accounts.len(), 1);

    Ok(())
}

async fn test_basic(client: &TardisRelDBClient) -> TardisResult<()> {
    // Insert
    tardis_db_config::ActiveModel {
        k: Set("k1".to_string()),
        v: Set("v1".to_string()),
        creator: Set("admin".to_string()),
        updater: Set("admin".to_string()),
        ..Default::default()
    }
    .insert(client.conn())
    .await?;

    let conf2 = tardis_db_config::ActiveModel {
        k: Set("k2".to_string()),
        v: Set("v2".to_string()),
        creator: Set("admin".to_string()),
        updater: Set("admin".to_string()),
        ..Default::default()
    };
    let insert_result = tardis_db_config::Entity::insert(conf2).exec(client.conn()).await?;

    // Find One
    let conf2 = tardis_db_config::Entity::find_by_id(insert_result.last_insert_id.clone()).one(client.conn()).await?.unwrap();
    assert_eq!(conf2.k, "k2");
    assert_eq!(conf2.create_time, conf2.update_time);

    // Update One
    sleep(Duration::from_millis(1100)).await;
    let mut conf2: tardis_db_config::ActiveModel = conf2.into();
    conf2.v = Set("v2更新".to_string());
    conf2.update(client.conn()).await?;
    let conf2 = tardis_db_config::Entity::find_by_id(insert_result.last_insert_id.clone()).one(client.conn()).await?.unwrap();
    assert_eq!(conf2.v, "v2更新");
    assert_ne!(conf2.create_time, conf2.update_time);

    // Update Many
    tardis_db_config::Entity::update_many()
        .col_expr(tardis_db_config::Column::V, Expr::value("v1更新"))
        .filter(tardis_db_config::Column::Id.ne(insert_result.last_insert_id))
        .exec(client.conn())
        .await?;

    // Find Many
    let confs = tardis_db_config::Entity::find().filter(tardis_db_config::Column::K.contains("k")).order_by_desc(tardis_db_config::Column::K).all(client.conn()).await?;
    assert_eq!(confs.len(), 2);
    assert_eq!(confs[0].k, "k2");
    assert_eq!(confs[1].k, "k1");
    assert_eq!(confs[0].v, "v2更新");
    assert_eq!(confs[1].v, "v1更新");

    // Page
    let conf_page = tardis_db_config::Entity::find().filter(tardis_db_config::Column::K.contains("k1")).order_by_desc(tardis_db_config::Column::K).paginate(client.conn(), 1);
    assert_eq!(conf_page.num_pages().await.unwrap(), 1);
    assert_eq!(conf_page.cur_page(), 0);
    let confs = conf_page.fetch_page(0).await?;
    assert_eq!(confs.len(), 1);
    assert_eq!(confs[0].k, "k1");
    assert_eq!(confs[0].v, "v1更新");

    // Exists TODO https://github.com/SeaQL/sea-orm/issues/408

    // Soft Delete
    tardis_db_config::Entity::find().soft_delete(client.conn(), "admin").await?;
    let dels = tardis_db_del_record::Entity::find().all(client.conn()).await?;
    assert_eq!(dels.len(), 2);
    assert_eq!(dels[0].entity_name, "tardis_config");

    // Delete
    let delete_result = tardis_db_del_record::Entity::delete_many().filter(tardis_db_del_record::Column::Id.eq(dels[0].id.clone())).exec(client.conn()).await?;
    assert_eq!(delete_result.rows_affected, 1);

    // Count
    let count = tardis_db_del_record::Entity::find().count(client.conn()).await?;
    assert_eq!(count, 1);

    Ok(())
}

pub mod entities {

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
            type Entity = Entity;

            fn fill_cxt(&mut self, _: &TardisContext, is_insert: bool) {
                if is_insert {
                    self.id = Set(TardisFuns::field.uuid_str());
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
            type Entity = Entity;

            fn fill_cxt(&mut self, _: &TardisContext, is_insert: bool) {
                if is_insert {
                    self.id = Set(TardisFuns::field.uuid_str());
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
            type Entity = Entity;

            fn fill_cxt(&mut self, _: &TardisContext, is_insert: bool) {
                if is_insert {
                    self.id = Set(TardisFuns::field.uuid_str());
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
            type Entity = Entity;

            fn fill_cxt(&mut self, _: &TardisContext, is_insert: bool) {
                if is_insert {
                    self.id = Set(TardisFuns::field.uuid_str());
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
}
