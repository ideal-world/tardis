use std::collections::HashMap;
use std::env;
use std::time::Duration;

use tokio::time::sleep;

use tardis::basic::result::TardisResult;
use tardis::test::test_container::TardisTestContainer;
use tardis::tokio;
use tardis::TardisFuns;

#[tokio::main]
async fn main() -> TardisResult<()> {
    // Here is a demonstration of using docker to start a mysql simulation scenario.
    let rabbit_container = TardisTestContainer::rabbit_custom().await?;
    let port = rabbit_container.get_host_port_ipv4(5672).await?;
    let url = format!("amqp://guest:guest@127.0.0.1:{port}/%2f");
    env::set_var("TARDIS_FW.MQ.URL", url);

    env::set_var("RUST_LOG", "debug");
    env::set_var("PROFILE", "default");

    // Initial configuration
    TardisFuns::init(Some("config")).await?;

    let client = TardisFuns::mq();

    // --------------------------------------------------

    let mut header = HashMap::new();
    header.insert("k1".to_string(), "v1".to_string());

    /*let latch_req = CountDownLatch::new(4);
    let latch_cp = latch_req.clone();*/
    client
        .response("test-addr", |(header, msg)| async move {
            println!("response1");
            assert_eq!(header.get("k1").unwrap(), "v1");
            assert_eq!(msg, "测试!");
            // move occurs because ..., which does not implement the `Copy` trait
            //latch_cp.countdown();
            Ok(())
        })
        .await?;

    client
        .response("test-addr", |(header, msg)| async move {
            println!("response2");
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
            println!("subscribe1");
            assert_eq!(header.get("k1").unwrap(), "v1");
            assert_eq!(msg, "测试!");
            Ok(())
        })
        .await?;

    client
        .subscribe("test-topic", |(header, msg)| async move {
            println!("subscribe2");
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
}
