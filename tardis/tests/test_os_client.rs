use std::env;

use s3::serde_types::LifecycleFilter;
use tracing::info;

use tardis::basic::result::TardisResult;
use tardis::config::config_dto::{FrameworkConfig, OSModuleConfig, TardisConfig};
use tardis::test::test_container::TardisTestContainer;
use tardis::TardisFuns;

#[tokio::test(flavor = "multi_thread")]
async fn test_os_client() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis=trace");
    TardisFuns::init_log();
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

        TardisFuns::os().object_copy("test/test.txt", "test/test_cp.txt", Some(bucket_name)).await?;
        let data = TardisFuns::os().object_get("test/test_cp.txt", Some(bucket_name)).await?;
        assert_eq!(String::from_utf8(data).unwrap(), "I want to go to S3 测试");

        info!("object_get_url = {:?}", TardisFuns::os().object_exist("test/test.txt", Some(bucket_name)).await?);

        info!("object_create_url = {:?}", TardisFuns::os().object_exist("test/test1.txt", Some(bucket_name)).await?);

        let put_config = s3::serde_types::BucketLifecycleConfiguration::new(vec![s3::serde_types::LifecycleRule::builder("Enabled")
            .expiration(s3::serde_types::Expiration::new(None, Some(30), None))
            .filter(LifecycleFilter::new(None, None, None, Some("test".to_string()), None))
            .build()]);
        TardisFuns::os().put_lifecycle(Some(bucket_name), put_config.clone()).await?;

        let get_config = TardisFuns::os().get_lifecycle(Some(bucket_name)).await?;
        info!("get_lifecycle_rule = {:?}", get_config);
        assert_eq!(serde_json::to_string(&put_config).unwrap(), serde_json::to_string(&get_config).unwrap());

        //info!("object_create_url = {}", TardisFuns::os().object_create_url("test/test2.txt", 1, Some(bucket_name.clone()))?);
        //
        //info!("object_delete_url = {}", TardisFuns::os().object_delete_url("test/test.txt", 60, Some(bucket_name.clone()))?);

        let data = TardisFuns::os().object_get("test/test.txt", Some(bucket_name)).await?;
        assert_eq!(String::from_utf8(data).unwrap(), "I want to go to S3 测试");

        TardisFuns::os().object_delete("test/test.txt", Some(bucket_name)).await?;
        TardisFuns::os().object_delete("test/test_cp.txt", Some(bucket_name)).await?;
        assert!(TardisFuns::os().object_get("test/test.txt", Some(bucket_name)).await.is_err());

        TardisFuns::os().bucket_delete(bucket_name).await?;

        Ok(())
    })
    .await
}
