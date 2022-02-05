// https://github.com/CleverCloud/lapin

use std::collections::HashMap;
use std::time::Duration;

use tokio::time::sleep;

use tardis::basic::config::{CacheConfig, DBConfig, FrameworkConfig, MQConfig, NoneConfig, TardisConfig, WebServerConfig};
use tardis::basic::result::TardisResult;
use tardis::test::test_container::TardisTestContainer;
use tardis::TardisFuns;

#[tokio::test]
async fn test_mq_client() -> TardisResult<()> {
    TardisFuns::init_log()?;
    TardisTestContainer::rabbit(|url| async move {
        // Default test
        TardisFuns::init_conf(TardisConfig {
            ws: NoneConfig {},
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
                mq: MQConfig { enabled: true, url },
                adv: Default::default(),
            },
        })
        .await?;

        let client = TardisFuns::mq();

        let mut header = HashMap::new();
        header.insert("k1".to_string(), "v1".to_string());

        /*let latch_req = CountDownLatch::new(4);
        let latch_cp = latch_req.clone();*/
        client
            .response("test-addr", |(header, msg)| async move {
                println!("response1 {}", msg);
                assert_eq!(header.get("k1").unwrap(), "v1");
                assert_eq!(msg, "测试!");
                // move occurs because ..., which does not implement the `Copy` trait
                //latch_cp.countdown();
                Ok(())
            })
            .await?;

        client
            .response("test-addr", |(header, msg)| async move {
                println!("response2 {}", msg);
                assert_eq!(header.get("k1").unwrap(), "v1");
                assert_eq!(msg, "测试!");
                Ok(())
            })
            .await?;

        client.request("test-addr", "测试!".to_string(), &header).await?;
        client.request("test-addr", "测试!".to_string(), &header).await?;
        client.request("test-addr", "测试!".to_string(), &header).await?;
        client.request("test-addr", "测试!".to_string(), &header).await?;

        client
            .subscribe("test-topic", |(header, msg)| async move {
                println!("subscribe1 {}", msg);
                assert_eq!(header.get("k1").unwrap(), "v1");
                assert_eq!(msg, "测试!");
                Ok(())
            })
            .await?;

        client
            .subscribe("test-topic", |(header, msg)| async move {
                println!("subscribe2 {}", msg);
                assert_eq!(header.get("k1").unwrap(), "v1");
                assert_eq!(msg, "测试!");
                Ok(())
            })
            .await?;

        client.publish("test-topic", "测试!".to_string(), &header).await?;
        client.publish("test-topic", "测试!".to_string(), &header).await?;
        client.publish("test-topic", "测试!".to_string(), &header).await?;
        client.publish("test-topic", "测试!".to_string(), &header).await?;

        sleep(Duration::from_millis(1000)).await;

        TardisFuns::shutdown().await?;
        Ok(())
    })
    .await
}
