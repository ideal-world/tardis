use std::env;
use std::sync::Arc;
use std::time::Duration;
use tardis::basic::result::TardisResult;
use tardis::test::test_container::TardisTestContainer;
use tardis::tokio;
use tardis::tokio::sync::Mutex;
use tardis::tokio::time::sleep;
use tardis::TardisFuns;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> TardisResult<()> {
    // Here is a demonstration of using docker to start a mysql simulation scenario.
    let redis_container = TardisTestContainer::redis_custom().await?;
    let port = redis_container.get_host_port_ipv4(6379).await?;
    let url = format!("redis://127.0.0.1:{port}/0");
    env::set_var("TARDIS_FW.CACHE.URL", url.clone());
    env::set_var("TARDIS_FW.CACHE.MODULES.M1.URL", url.clone());

    env::set_var("RUST_LOG", "debug");
    env::set_var("PROFILE", "default");

    // Initial configuration
    TardisFuns::init(Some("config")).await?;

    let client = TardisFuns::cache();
    let client_m1 = TardisFuns::cache_by_module("m1");

    // --------------------------------------------------

    // basic operations

    let mut opt_value = client.get("test_key").await?;
    assert_eq!(opt_value, None);

    client.set("test_key", "测试").await?;
    let mut str_value = client.get("test_key").await?.unwrap();
    assert_eq!(str_value, "测试");

    let mut set_result = client.set_nx("test_key", "测试2").await?;
    assert!(!set_result);
    client.get("test_key").await?;
    assert_eq!(str_value, "测试");
    set_result = client.set_nx("test_key_nx", "测试2").await?;
    assert!(set_result);
    str_value = client.get("test_key_nx").await?.unwrap();
    assert_eq!(str_value, "测试2");

    client.expire("test_key_nx", 1).await?;
    client.set_ex("test_key_ex", "测试3", 1).await?;
    str_value = client.get("test_key_ex").await?.unwrap();
    assert_eq!(str_value, "测试3");
    let mut bool_value = client.exists("test_key_ex").await?;
    assert!(bool_value);
    sleep(Duration::from_millis(1200)).await;
    opt_value = client.get("test_key_ex").await?;
    assert_eq!(opt_value, None);
    bool_value = client.exists("test_key_ex").await?;
    assert!(!bool_value);
    bool_value = client.exists("test_key_nx").await?;
    assert!(!bool_value);

    opt_value = client.getset("test_key_none", "孤岛旭日").await?;
    assert_eq!(opt_value, None);
    opt_value = client.getset("test_key_none", "idealworld").await?;
    assert_eq!(opt_value.unwrap(), "孤岛旭日");

    client.del("test_key_none1").await?;
    client.del("test_key_none").await?;
    bool_value = client.exists("test_key_none").await?;
    assert!(!bool_value);

    let mut num_value = client.incr("incr", 1).await?;
    assert_eq!(num_value, 1);
    num_value = client.incr("incr", 1).await?;
    assert_eq!(num_value, 2);
    num_value = client.incr("incr", -1).await?;
    assert_eq!(num_value, 1);

    client.expire_at("test_key_xp", 1893430861).await?;
    let num_value = client.ttl("test_key_xp").await?;
    println!("Expire AT : {num_value}");
    assert!(num_value > 0);

    // hash operations

    client.hset("h", "f1", "v1").await?;
    client.hset("h", "f2", "v2").await?;
    assert_eq!(client.hget("h", "f0").await?, None);
    assert_eq!(client.hget("h", "f1").await?.unwrap(), "v1");

    assert!(client.hexists("h", "f1").await?);
    client.hdel("h", "f1").await?;
    assert!(!client.hexists("h", "f1").await?);

    assert!(client.hset_nx("h", "f0", "v0").await?);
    assert!(!client.hset_nx("h", "f0", "v0").await?);

    assert_eq!(client.hincr("h", "f3", 1).await?, 1);
    assert_eq!(client.hincr("h", "f3", 1).await?, 2);
    assert_eq!(client.hincr("h", "f3", -1).await?, 1);

    assert_eq!(client.hkeys("h").await?, vec!("f2", "f0", "f3"));
    assert_eq!(client.hvals("h").await?, vec!("v2", "v0", "1"));

    assert_eq!(client.hlen("h").await?, 3);

    let map_result = client.hgetall("h").await?;
    assert_eq!(map_result.len(), 3);
    assert_eq!(map_result.get("f2").unwrap(), "v2");
    assert_eq!(map_result.get("f0").unwrap(), "v0");
    assert_eq!(map_result.get("f3").unwrap(), "1");

    // module 1 operations
    client_m1.set("test_key_m1", "测试").await?;
    assert_eq!(client_m1.get("test_key_m1").await?.unwrap(), "测试");

    // --------------------------------------------------
    // Pub/Sub operations
    // --------------------------------------------------

    println!("\n=== Redis Pub/Sub Examples ===\n");

    // Example 1: Basic publish/subscribe
    println!("Example 1: Basic publish/subscribe");
    let mut pubsub = client.pubsub().await?;
    pubsub.subscribe("test_channel").await?;

    let received_messages = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received_messages.clone();

    // Spawn listener task
    let listener = tokio::spawn(async move {
        let mut stream = pubsub.on_message();
        for _ in 0..3 {
            if let Some(msg) = stream.next().await {
                let payload: String = msg.get_payload().unwrap();
                println!("  Received: {}", payload);
                received_clone.lock().await.push(payload);
            }
        }
    });

    sleep(Duration::from_millis(100)).await;

    // Publish messages
    client.publish("test_channel", "Hello Redis!").await?;
    client.publish("test_channel", "Pub/Sub is working").await?;
    client.publish("test_channel", "Third message").await?;

    listener.await.unwrap();
    println!("  Total messages received: {}\n", received_messages.lock().await.len());

    // Example 2: Pattern subscription
    println!("Example 2: Pattern subscription (wildcard)");
    let mut pubsub2 = client.pubsub().await?;
    pubsub2.psubscribe("user:*").await?;

    let pattern_messages = Arc::new(Mutex::new(Vec::new()));
    let pattern_clone = pattern_messages.clone();

    let listener2 = tokio::spawn(async move {
        let mut stream = pubsub2.on_message();
        for _ in 0..3 {
            if let Some(msg) = stream.next().await {
                let channel: String = msg.get_channel_name().to_string();
                let payload: String = msg.get_payload().unwrap();
                println!("  Channel: {}, Message: {}", channel, payload);
                pattern_clone.lock().await.push((channel, payload));
            }
        }
    });

    sleep(Duration::from_millis(100)).await;

    // These will match the pattern
    client.publish("user:login", "User logged in").await?;
    client.publish("user:logout", "User logged out").await?;
    client.publish("user:update", "User profile updated").await?;
    // This won't match
    client.publish("admin:action", "Should not receive").await?;

    listener2.await.unwrap();
    println!("  Pattern-matched messages: {}\n", pattern_messages.lock().await.len());

    // Example 3: Multiple channels
    println!("Example 3: Multiple channels subscription");
    let mut pubsub3 = client.pubsub().await?;
    pubsub3.subscribe(&["events", "logs", "alerts"]).await?;

    let listener3 = tokio::spawn(async move {
        let mut stream = pubsub3.on_message();
        for _ in 0..3 {
            if let Some(msg) = stream.next().await {
                let channel: String = msg.get_channel_name().to_string();
                let payload: String = msg.get_payload().unwrap();
                println!("  [{}] {}", channel, payload);
            }
        }
    });

    sleep(Duration::from_millis(100)).await;

    client.publish("events", "System started").await?;
    client.publish("logs", "Debug info").await?;
    client.publish("alerts", "High CPU usage").await?;

    listener3.await.unwrap();

    println!("\n=== All examples completed successfully! ===");

    Ok(())
}
