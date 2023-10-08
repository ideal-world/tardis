use std::env;

use tracing::info;

use tardis::basic::result::TardisResult;
use tardis::config::config_dto::{FrameworkConfig, OSModuleConfig, TardisConfig};
use tardis::test::test_container::TardisTestContainer;
use tardis::TardisFuns;

#[tokio::test]
async fn test_os_client() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis=trace");
    TardisFuns::init_log()?;

    TardisTestContainer::minio(|url| async move {
        let os_module_config = OSModuleConfig::builder().kind("s3").endpoint(url).ak("minioadmin").sk("minioadmin").region("us-east-1").build();
        TardisFuns::init_conf(TardisConfig::builder().fw(FrameworkConfig::builder().os(os_module_config).build()).build()).await?;
        let bucket_name = "test";

        TardisFuns::os().bucket_create_simple(bucket_name, true).await?;
        let resp = TardisFuns::os().bucket_create_simple(bucket_name, true).await;
        assert_eq!(resp.err().unwrap().code, "409");

        TardisFuns::os().object_create("test/test.txt", "I want to go to S3 测试".as_bytes(), None, Some(bucket_name)).await?;

        let data = TardisFuns::os().object_get("test/test.txt", Some(bucket_name)).await?;
        assert_eq!(String::from_utf8(data).unwrap(), "I want to go to S3 测试");

        info!("object_get_url = {}", TardisFuns::os().object_get_url("test/test.txt", 60, Some(bucket_name))?);

        //info!("object_create_url = {}", TardisFuns::os().object_create_url("test/test2.txt", 1, Some(bucket_name.clone()))?);
        //
        //info!("object_delete_url = {}", TardisFuns::os().object_delete_url("test/test.txt", 60, Some(bucket_name.clone()))?);

        let data = TardisFuns::os().object_get("test/test.txt", Some(bucket_name)).await?;
        assert_eq!(String::from_utf8(data).unwrap(), "I want to go to S3 测试");

        TardisFuns::os().object_delete("test/test.txt", Some(bucket_name)).await?;
        assert!(TardisFuns::os().object_get("test/test.txt", Some(bucket_name)).await.is_err());

        TardisFuns::os().bucket_delete(bucket_name).await?;

        Ok(())
    })
    .await
}
