use log::info;

use tardis::basic::config::{CacheConfig, DBConfig, FrameworkConfig, MQConfig, MailConfig, OSConfig, SearchConfig, TardisConfig, WebServerConfig};
use tardis::basic::result::TardisResult;
use tardis::TardisFuns;

#[tokio::test]
async fn test_os_client() -> TardisResult<()> {
    TardisFuns::init_log()?;
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
                endpoint: "https://play.min.io:9000".to_string(),
                ak: "Q3AM3UQ867SPQQA43P2F".to_string(),
                sk: "zuf+tfteSlswRu7BJ86wekitnifILbZam1KYY3TG".to_string(),
                region: "us-east-1".to_string(),
                default_bucket: "".to_string(),
                modules: Default::default(),
            },
            adv: Default::default(),
        },
    })
    .await?;

    let bucket_name = format!("tardis-test-{}", rand::random::<u16>());

    TardisFuns::os().bucket_create_simple(&bucket_name, true).await?;

    TardisFuns::os().object_create("test/test.txt", "I want to go to S3 测试".as_bytes(), None, Some(&bucket_name)).await?;

    let data = TardisFuns::os().object_get("test/test.txt", Some(&bucket_name)).await?;
    assert_eq!(String::from_utf8(data).unwrap(), "I want to go to S3 测试");

    info!("object_get_url = {}", TardisFuns::os().object_get_url("test/test.txt", 60, Some(&bucket_name))?);

    //info!("object_create_url = {}", TardisFuns::os().object_create_url("test/test2.txt", 1, Some(&bucket_name))?);
    //
    //info!("object_delete_url = {}", TardisFuns::os().object_delete_url("test/test.txt", 60, Some(&bucket_name))?);

    let data = TardisFuns::os().object_get("test/test.txt", Some(&bucket_name)).await?;
    assert_eq!(String::from_utf8(data).unwrap(), "I want to go to S3 测试");

    TardisFuns::os().object_delete("test/test.txt", Some(&bucket_name)).await?;
    assert!(TardisFuns::os().object_get("test/test.txt", Some(&bucket_name)).await.is_err());

    TardisFuns::os().bucket_delete(&bucket_name).await?;

    Ok(())
}
