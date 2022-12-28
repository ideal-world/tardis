use std::env;

use log::info;

use tardis::basic::result::TardisResult;
use tardis::config::config_dto::{CacheConfig, DBConfig, FrameworkConfig, MQConfig, MailConfig, OSConfig, SearchConfig, TardisConfig, WebServerConfig};
use tardis::test::test_container::TardisTestContainer;
use tardis::TardisFuns;

#[tokio::test]
async fn test_os_client() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis=trace");
    TardisFuns::init_log()?;

    TardisTestContainer::minio(|url| async move {
        TardisFuns::init_conf(TardisConfig {
            cs: Default::default(),
            fw: FrameworkConfig {
                app: Default::default(),
                web_server: WebServerConfig {
                    enabled: false,
                    ..Default::default()
                },
                web_client: Default::default(),
                cache: CacheConfig {
                    enabled: false,
                    ..Default::default()
                },
                db: DBConfig {
                    enabled: false,
                    ..Default::default()
                },
                mq: MQConfig {
                    enabled: false,
                    ..Default::default()
                },
                search: SearchConfig {
                    enabled: false,
                    ..Default::default()
                },
                mail: MailConfig {
                    enabled: false,
                    ..Default::default()
                },
                os: OSConfig {
                    enabled: true,
                    kind: "s3".to_string(),
                    endpoint: url.to_string(),
                    ak: "minioadmin".to_string(),
                    sk: "minioadmin".to_string(),
                    region: "us-east-1".to_string(),
                    default_bucket: "".to_string(),
                    modules: Default::default(),
                },
                ..Default::default()
            },
        })
        .await?;
        let bucket_name = "test".to_string();

        TardisFuns::os().bucket_create_simple(&bucket_name, true).await?;
        let resp = TardisFuns::os().bucket_create_simple(&bucket_name, true).await;
        assert_eq!(resp.err().unwrap().code, "409");

        TardisFuns::os().object_create("test/test.txt", "I want to go to S3 测试".as_bytes(), None, Some(bucket_name.clone())).await?;

        let data = TardisFuns::os().object_get("test/test.txt", Some(bucket_name.clone())).await?;
        assert_eq!(String::from_utf8(data).unwrap(), "I want to go to S3 测试");

        info!("object_get_url = {}", TardisFuns::os().object_get_url("test/test.txt", 60, Some(bucket_name.clone()))?);

        //info!("object_create_url = {}", TardisFuns::os().object_create_url("test/test2.txt", 1, Some(bucket_name.clone()))?);
        //
        //info!("object_delete_url = {}", TardisFuns::os().object_delete_url("test/test.txt", 60, Some(bucket_name.clone()))?);

        let data = TardisFuns::os().object_get("test/test.txt", Some(bucket_name.clone())).await?;
        assert_eq!(String::from_utf8(data).unwrap(), "I want to go to S3 测试");

        TardisFuns::os().object_delete("test/test.txt", Some(bucket_name.clone())).await?;
        assert!(TardisFuns::os().object_get("test/test.txt", Some(bucket_name.clone())).await.is_err());

        TardisFuns::os().bucket_delete(&bucket_name).await?;

        Ok(())
    })
    .await
}
