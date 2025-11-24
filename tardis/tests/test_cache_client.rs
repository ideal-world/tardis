// https://github.com/mitsuhiko/redis-rs

use std::env;
use std::sync::Arc;

use futures_util::StreamExt;
use tardis::cache::AsyncCommands;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};
use tracing::info;

use tardis::basic::result::TardisResult;
use tardis::cache::cache_client::TardisCacheClient;
use tardis::config::config_dto::{CacheConfig, CacheModuleConfig, FrameworkConfig, TardisConfig};
use tardis::test::test_container::TardisTestContainer;
use tardis::TardisFuns;
use url::Url;

#[tokio::test(flavor = "multi_thread")]
async fn test_cache_client() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis=trace");
    TardisFuns::init_log(); // let url = "redis://:123456@127.0.0.1:6379/1".to_lowercase();
    TardisTestContainer::redis(|url| async move {
        let url = url.parse::<Url>().expect("invalid url");
        let cache_module_config = CacheModuleConfig::builder().url(url).build();
        let client = TardisCacheClient::init(&cache_module_config).await?;
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
        num_value = client.incr("incr", -3).await?;
        assert_eq!(num_value, -2);

        client.expire_at("test_key_xp", 1893430861).await?;
        let num_value = client.ttl("test_key_xp").await?;
        println!("Expire AT : {}", num_value);
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

        // list operations
        client.lpush("l", "v1").await?;
        client.lpush("l", "v2").await?;
        client.lpushmulti("l", vec!["v3", "v4"]).await?;
        assert_eq!(client.llen("l").await?, 4);
        let list_result = client.lrangeall("l").await?;
        assert_eq!(list_result.len(), 4);
        assert_eq!(list_result.first().unwrap(), "v4");
        assert_eq!(list_result.get(1).unwrap(), "v3");
        let lset_result = client.lset("l", 0, "v0").await?;
        assert!(lset_result);
        let list_result = client.lrangeall("l").await?;
        assert_eq!(list_result.len(), 4);
        assert_eq!(list_result.first().unwrap(), "v0");
        let lrem_result = client.lrem("l", 1, "v0").await?;
        assert_eq!(lrem_result, 1);
        let list_result = client.lrangeall("l").await?;
        assert_eq!(list_result.len(), 3);
        assert_eq!(list_result.first().unwrap(), "v3");

        // bitmap operations
        assert!(!client.setbit("bit", 1024, true).await?);
        assert!(client.setbit("bit", 1024, true).await?);
        assert!(!client.setbit("bit", 2048, true).await?);
        assert!(client.getbit("bit", 1024).await?);
        assert!(client.getbit("bit", 2048).await?);
        assert!(!client.getbit("bit", 3333).await?);
        assert_eq!(client.bitcount("bit").await?, 2);
        assert_eq!(client.bitcount_range_by_byte("bit", 1, 1023 / 8).await?, 0);
        assert_eq!(client.bitcount_range_by_byte("bit", 1, 1024 / 8).await?, 1);
        assert_eq!(client.bitcount_range_by_byte("bit", 1024 / 8, 2048 / 8).await?, 2);
        assert_eq!(client.bitcount_range_by_byte("bit", 2048 / 8, 6666 / 8).await?, 1);

        let max: i64 = u32::MAX.into();
        assert!(!client.setbit("bit", max.try_into()?, true).await?);

        // custom

        let mut _s: bool = client.cmd().await?.sadd("s1", "m1").await?;
        _s = client.cmd().await?.sadd("s1", "m2").await?;
        let mem: Vec<String> = client.cmd().await?.smembers("s1").await?;
        assert!(mem.contains(&"m1".to_string()));
        assert!(mem.contains(&"m2".to_string()));
        assert!(!mem.contains(&"m3".to_string()));

        // Default test
        TardisFuns::init_conf(
            TardisConfig::builder()
                .fw(FrameworkConfig::builder()
                    .cache(CacheConfig::builder().default(cache_module_config.clone()).modules([("m1".to_string(), cache_module_config.clone())]).build())
                    .build())
                .build(),
        )
        .await?;

        let map_result = TardisFuns::cache().hgetall("h").await?;
        assert_eq!(map_result.len(), 3);
        assert_eq!(map_result.get("f2").unwrap(), "v2");
        assert_eq!(map_result.get("f0").unwrap(), "v0");
        assert_eq!(map_result.get("f3").unwrap(), "1");

        let map_result = TardisFuns::cache_by_module("m1").hgetall("h").await?;
        assert_eq!(map_result.len(), 3);
        assert_eq!(map_result.get("f2").unwrap(), "v2");
        assert_eq!(map_result.get("f0").unwrap(), "v0");
        assert_eq!(map_result.get("f3").unwrap(), "1");

        tokio::spawn(async {
            let map_result = TardisFuns::cache_by_module("m1").hgetall("h").await.unwrap();
            assert_eq!(map_result.len(), 3);
            assert_eq!(map_result.get("f2").unwrap(), "v2");
            assert_eq!(map_result.get("f0").unwrap(), "v0");
            assert_eq!(map_result.get("f3").unwrap(), "1");
            info!("cache_by_module m1 hgetall done");
        })
        .await
        .unwrap();

        // flush

        client.set("flush_test", "测试").await?;
        assert!(client.exists("flush_test").await?);
        client.flushdb().await?;
        assert!(!client.exists("flush_test").await?);

        // script
        let k1_in = "key1";
        let k2_in = "key2";
        let a1_in = 1;
        let a2_in = 2;
        let (k1_out, k2_out, a1_out, a2_out): (String, String, u32, u32) =
            client.script(r#"return {KEYS[1],KEYS[2],ARGV[1],ARGV[2]}"#).arg(&[a1_in, a2_in]).key(k1_in).key(k2_in).invoke().await?;
        assert_eq!(k1_in, k1_out);
        assert_eq!(k2_in, k2_out);
        assert_eq!(a1_in, a1_out);
        assert_eq!(a2_in, a2_out);
        // _test_concurrent().await?;
        Ok(())
    })
    .await
}

async fn _test_concurrent() -> TardisResult<()> {
    let threads: Vec<i32> = (0..100).collect();

    let _ = threads
        .into_iter()
        .map(|_| {
            tokio::task::spawn(async {
                let client = TardisFuns::cache_by_module("m1");
                let id = TardisFuns::field.nanoid();
                loop {
                    info!("--------##{}", id);
                    client.set_ex("con_str", &TardisFuns::field.nanoid(), 10).await.unwrap();
                    client.get("con_str").await.unwrap();
                    client.get("con_str_none").await.unwrap();
                    client.hset("con_map", &TardisFuns::field.nanoid(), r#"{"user":{"id":1,"name":"张三","open":false}}"#).await.unwrap();
                    client.hgetall("con_map").await.unwrap();
                    sleep(Duration::from_millis(100)).await;
                }
            })
        })
        .collect::<Vec<JoinHandle<()>>>();
    sleep(Duration::from_secs(10000)).await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cache_pubsub() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis=trace");
    TardisFuns::init_log();

    TardisTestContainer::redis(|url| async move {
        let url = url.parse::<Url>().expect("invalid url");
        let cache_module_config = CacheModuleConfig::builder().url(url).build();
        let client = TardisCacheClient::init(&cache_module_config).await?;

        info!("=== Test 1: Basic publish/subscribe ===");

        // Create a pub/sub connection and subscribe
        let mut pubsub = client.pubsub().await?;
        pubsub.subscribe("test_channel").await?;

        let received_messages = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received_messages.clone();

        // Spawn listener task
        let listener_handle = tokio::spawn(async move {
            let mut stream = pubsub.on_message();
            for _ in 0..3 {
                if let Some(msg) = stream.next().await {
                    let payload: String = msg.get_payload().unwrap();
                    info!("Received: {}", payload);
                    received_clone.lock().await.push(payload);
                }
            }
        });

        // Give subscription time to establish
        sleep(Duration::from_millis(100)).await;

        // Publish messages
        client.publish("test_channel", "Message 1").await?;
        client.publish("test_channel", "Message 2").await?;
        client.publish("test_channel", "Message 3").await?;

        // Wait for listener to receive all messages
        listener_handle.await.unwrap();

        // Verify
        let messages = received_messages.lock().await;
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0], "Message 1");
        assert_eq!(messages[1], "Message 2");
        assert_eq!(messages[2], "Message 3");

        info!("=== Test 2: Pattern subscriptions ===");

        let mut pubsub2 = client.pubsub().await?;
        pubsub2.psubscribe("user:*").await?;

        let pattern_messages = Arc::new(Mutex::new(Vec::new()));
        let pattern_clone = pattern_messages.clone();

        let listener_handle2 = tokio::spawn(async move {
            let mut stream = pubsub2.on_message();
            for _ in 0..3 {
                if let Some(msg) = stream.next().await {
                    let payload: String = msg.get_payload().unwrap();
                    let channel: String = msg.get_channel_name().to_string();
                    info!("Pattern matched: channel={}, payload={}", channel, payload);
                    pattern_clone.lock().await.push((channel, payload));
                }
            }
        });

        sleep(Duration::from_millis(100)).await;

        // Publish to matching channels
        client.publish("user:login", "Login event").await?;
        client.publish("user:logout", "Logout event").await?;
        client.publish("user:update", "Update event").await?;
        client.publish("system:error", "Should not match").await?;

        listener_handle2.await.unwrap();

        let pattern_msgs = pattern_messages.lock().await;
        assert_eq!(pattern_msgs.len(), 3);
        assert!(pattern_msgs.iter().any(|(ch, msg)| ch == "user:login" && msg == "Login event"));
        assert!(pattern_msgs.iter().any(|(ch, msg)| ch == "user:logout" && msg == "Logout event"));
        assert!(pattern_msgs.iter().any(|(ch, msg)| ch == "user:update" && msg == "Update event"));

        info!("=== Test 3: Multiple channels subscription ===");

        let mut pubsub3 = client.pubsub().await?;
        pubsub3.subscribe(&["channel1", "channel2", "channel3"]).await?;

        let multi_messages = Arc::new(Mutex::new(Vec::new()));
        let multi_clone = multi_messages.clone();

        let listener_handle3 = tokio::spawn(async move {
            let mut stream = pubsub3.on_message();
            for _ in 0..3 {
                if let Some(msg) = stream.next().await {
                    let channel: String = msg.get_channel_name().to_string();
                    let payload: String = msg.get_payload().unwrap();
                    multi_clone.lock().await.push((channel, payload));
                }
            }
        });

        sleep(Duration::from_millis(100)).await;

        client.publish("channel1", "From channel 1").await?;
        client.publish("channel2", "From channel 2").await?;
        client.publish("channel3", "From channel 3").await?;

        listener_handle3.await.unwrap();

        let multi_msgs = multi_messages.lock().await;
        assert_eq!(multi_msgs.len(), 3);
        assert!(multi_msgs.iter().any(|(ch, msg)| ch == "channel1" && msg == "From channel 1"));
        assert!(multi_msgs.iter().any(|(ch, msg)| ch == "channel2" && msg == "From channel 2"));
        assert!(multi_msgs.iter().any(|(ch, msg)| ch == "channel3" && msg == "From channel 3"));

        info!("All pub/sub tests passed!");
        Ok(())
    })
    .await
}
