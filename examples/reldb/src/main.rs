use std::env;

use tardis::basic::dto::TardisContext;
use tardis::basic::result::TardisResult;
use tardis::db::sea_orm::*;
use tardis::log::info;
use tardis::test::test_container::TardisTestContainer;
use tardis::tokio;
use tardis::TardisFuns;

mod domain;

#[tokio::main]
async fn main() -> TardisResult<()> {
    // Here is a demonstration of using docker to start a mysql simulation scenario.
    let mysql_container = TardisTestContainer::mysql_custom(None).await?;
    let port = mysql_container.get_host_port_ipv4(3306).await?;
    let url = format!("mysql://root:123456@localhost:{port}/test");
    env::set_var("TARDIS_FW.DB.URL", url);

    env::set_var("RUST_LOG", "debug");
    env::set_var("PROFILE", "default");

    // Initial configuration
    TardisFuns::init(Some("config")).await?;

    let db = TardisFuns::reldb().conn();

    // --------------------------------------------------

    let ctx = TardisContext {
        own_paths: String::new(),
        ak: String::new(),
        owner: String::new(),
        roles: vec![],
        groups: vec![],
        ext: Default::default(),
        sync_task_fns: Default::default(),
        async_task_fns: Default::default(),
    };

    // Create table
    db.create_table_from_entity(domain::tenant::Entity).await?;
    db.create_table_from_entity(domain::tenant_conf::Entity).await?;
    db.create_table_from_entity(domain::app::Entity).await?;
    db.create_table_from_entity(domain::account::Entity).await?;
    db.create_table_from_entity(domain::app_account_rel::Entity).await?;

    // Insert some records
    db.insert_one(
        domain::tenant::ActiveModel {
            name: Set("tenant1".to_string()),
            ..Default::default()
        },
        &ctx,
    )
    .await?;

    let tenant = domain::tenant::Entity::find().one(db.raw_conn()).await?.unwrap();

    db.insert_one(
        domain::tenant_conf::ActiveModel {
            name: Set("conf1".to_string()),
            tenant_id: Set(tenant.id.clone()),
            ..Default::default()
        },
        &ctx,
    )
    .await?;

    db.insert_one(
        domain::app::ActiveModel {
            name: Set("app1".to_string()),
            tenant_id: Set(tenant.id.clone()),
            ..Default::default()
        },
        &ctx,
    )
    .await?;

    db.insert_one(
        domain::app::ActiveModel {
            name: Set("app2".to_string()),
            tenant_id: Set(tenant.id.clone()),
            ..Default::default()
        },
        &ctx,
    )
    .await?;

    let tenant = domain::tenant::Entity::find_by_id(tenant.id.clone()).one(db.raw_conn()).await?.unwrap();

    info!("----------------- One To One -----------------");
    let config = tenant.find_related(domain::tenant_conf::Entity).one(db.raw_conn()).await?.unwrap();
    assert_eq!(config.name, "conf1");
    let tenant = config.find_related(domain::tenant::Entity).one(db.raw_conn()).await?.unwrap();
    assert_eq!(tenant.name, "tenant1");

    info!("----------------- One To Many -----------------");
    let apps = tenant.find_related(domain::app::Entity).all(db.raw_conn()).await?;
    assert_eq!(apps.len(), 2);
    info!("----------------- Many To One -----------------");
    let tenant = apps[0].find_related(domain::tenant::Entity).one(db.raw_conn()).await?.unwrap();
    assert_eq!(tenant.name, "tenant1");

    info!("----------------- Many To Many -----------------");
    let accounts = apps[0].find_related(domain::account::Entity).all(db.raw_conn()).await?;
    assert_eq!(accounts.len(), 0);

    let account_id: String = db
        .insert_one(
            domain::account::ActiveModel {
                name: Set("account1".to_string()),
                ..Default::default()
            },
            &ctx,
        )
        .await?
        .last_insert_id;

    domain::app_account_rel::ActiveModel {
        app_id: Set(apps[0].id.to_string()),
        account_id: Set(account_id.to_string()),
    }
    .insert(db.raw_conn())
    .await?;

    let accounts = apps[0].find_related(domain::account::Entity).all(db.raw_conn()).await?;
    assert_eq!(accounts.len(), 1);

    Ok(())
}
