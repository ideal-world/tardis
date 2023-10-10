use std::{collections::HashMap, env, sync::Arc, time::Duration};

use tardis::basic::error::TardisError;
use tardis::basic::result::TardisResult;
use tardis::{basic::dto::TardisContext, TardisFuns};
use tokio::{
    sync::{Mutex, RwLock},
    time::sleep,
};

#[tokio::test(flavor = "multi_thread")]
async fn test_basic_dto() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis=trace");
    let ctx = TardisContext {
        own_paths: "".to_string(),
        ak: "".to_string(),
        owner: "".to_string(),
        roles: vec![],
        groups: vec![],
        ext: Arc::new(RwLock::new(HashMap::new())),
        sync_task_fns: Arc::new(Mutex::new(Vec::new())),
        async_task_fns: Arc::new(Mutex::new(Vec::new())),
    };
    let _ = ctx
        .add_sync_task(Box::new(|| {
            Box::pin(async move {
                println!("Starting background task");
                Ok(())
            })
        }))
        .await;
    let ctx_json = TardisFuns::json.obj_to_string(&ctx)?;
    println!("ctx_json: {}", ctx_json);
    let ctx: TardisContext = TardisFuns::json.str_to_obj(&ctx_json)?;
    println!("ctx: {:?}", ctx);
    let _ = ctx
        .add_async_task(Box::new(|| {
            Box::pin(async move {
                println!("Starting async background task box 1");
                sleep(Duration::from_secs(1)).await;
                println!("Finished async background task box 1");
                Ok(())
            })
        }))
        .await;
    let _ = ctx
        .add_async_task(Box::new(|| {
            Box::pin(async move {
                println!("Starting async background task box 2");
                println!("Finished async background task box 2");
                Ok(())
            })
        }))
        .await;
    let _ = ctx
        .add_async_task(Box::new(|| {
            Box::pin(async move {
                println!("Starting async background task box 3");
                sleep(Duration::from_secs(1)).await;
                println!("Finished async background task box 3");
                Ok(())
            })
        }))
        .await;
    let _ = ctx.add_async_task(Box::new(|| Box::pin(async move { async_test("2").await }))).await;
    let _ = ctx.add_sync_task(Box::new(|| Box::pin(async move { sync_test("3").await }))).await;

    println!("sleep 1 second before task scheduling");
    sleep(Duration::from_secs(1)).await;
    println!("sleep 1 second after task scheduling");
    let _ = ctx.execute_task().await;
    sleep(Duration::from_secs(4)).await;
    Ok(())
}

pub async fn sync_test(t: &str) -> TardisResult<()> {
    println!("Starting sync background task {}", t);
    sleep(Duration::from_secs(1)).await;
    println!("Finished sync background task {}", t);
    Ok(())
}
pub async fn async_test(t: &str) -> TardisResult<()> {
    println!("Starting async background task {}", t);
    sleep(Duration::from_secs(1)).await;
    println!("Finished async background task {}", t);
    Err(TardisError::bad_request("error", "error"))
}
