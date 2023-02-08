use std::collections::HashMap;
use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use lazy_static::lazy_static;
use poem::web::websocket::{BoxWebSocketUpgraded, WebSocket};
use poem_openapi::param::Path;
use serde_json::json;
use tardis::basic::result::TardisResult;
use tardis::web::web_server::TardisWebServer;
use tardis::web::ws_processor::{ws_broadcast, ws_echo, TardisWebsocketReq, TardisWebsocketResp};
use tardis::TardisFuns;
use tokio::sync::broadcast::Sender;
use tokio::sync::RwLock;
use tokio::time::sleep;

lazy_static! {
    static ref SENDERS: Arc<RwLock<HashMap<String, Sender<String>>>> = Arc::new(RwLock::new(HashMap::new()));
}

static ERROR_COUNTER: AtomicUsize = AtomicUsize::new(0);
static SUB_COUNTER: AtomicUsize = AtomicUsize::new(0);
static NON_SUB_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[tokio::test]
async fn test_websocket() -> TardisResult<()> {
    tokio::spawn(async {
        env::set_var("RUST_LOG", "trace");
        let serv = TardisWebServer::init_simple("127.0.0.1", 8080).unwrap();
        serv.add_route_with_ws(Api, 100).await;
        serv.start().await
    });
    sleep(Duration::from_millis(500)).await;

    // message not illegal test
    let error_client_a = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/broadcast/gerror/a", move |_msg| async move {
        // error message not returned
        assert!(1 == 2);
        ERROR_COUNTER.fetch_add(1, Ordering::SeqCst);
        None
    })
    .await?;
    error_client_a.send_raw("hi".to_string()).await?;
    // not found test
    let error_client_b = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/broadcast/gxxx/a", move |msg| async move {
        println!("client_not_found recv:{}", msg);
        assert_eq!(msg, "Websocket connection error: group not found");
        ERROR_COUNTER.fetch_add(1, Ordering::SeqCst);
        None
    })
    .await?;
    error_client_b
        .send_obj(&TardisWebsocketReq {
            msg: json! {"hi"},
            from_avatar: "a".to_string(),
            ..Default::default()
        })
        .await?;

    // subscribe mode test
    let sub_client_a = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/broadcast/g1/a", move |msg| async move {
        println!("client_a recv:{}", msg);
        assert_eq!(msg, "\"service send:\\\"hi\\\"\"");
        SUB_COUNTER.fetch_add(1, Ordering::SeqCst);
        None
    })
    .await?;
    let sub_client_b1 = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/broadcast/g1/b", move |msg| async move {
        println!("client_b1 recv:{}", msg);
        assert_eq!(msg, "\"service send:\\\"hi\\\"\"");
        SUB_COUNTER.fetch_add(1, Ordering::SeqCst);
        Some(
            TardisFuns::json
                .obj_to_string(&TardisWebsocketReq {
                    msg: json! {"client_b send:hi again"},
                    from_avatar: "b".to_string(),
                    ..Default::default()
                })
                .unwrap(),
        )
    })
    .await?;
    let sub_client_b2 = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/broadcast/g1/b", move |msg| async move {
        println!("client_b2 recv:{}", msg);
        assert_eq!(msg, "\"service send:\\\"hi\\\"\"");
        SUB_COUNTER.fetch_add(1, Ordering::SeqCst);
        Some(
            TardisFuns::json
                .obj_to_string(&TardisWebsocketReq {
                    msg: json! {"client_b send:hi again"},
                    from_avatar: "b".to_string(),
                    ..Default::default()
                })
                .unwrap(),
        )
    })
    .await?;
    sub_client_a
        .send_obj(&TardisWebsocketReq {
            msg: json! {"hi"},
            from_avatar: "a".to_string(),
            ..Default::default()
        })
        .await?;
    sub_client_b1
        .send_obj(&TardisWebsocketReq {
            msg: json! {"hi"},
            from_avatar: "b".to_string(),
            ..Default::default()
        })
        .await?;
    sub_client_b2
        .send_obj(&TardisWebsocketReq {
            msg: json! {"hi"},
            from_avatar: "b".to_string(),
            ..Default::default()
        })
        .await?;

    // non-subscribe mode test
    let non_sub_client_a = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/broadcast/g2/a", move |msg| async move {
        println!("client_a recv:{}", msg);
        assert_eq!(msg, "\"service send:\\\"hi\\\"\"");
        NON_SUB_COUNTER.fetch_add(1, Ordering::SeqCst);
        None
    })
    .await?;
    let non_sub_client_b1 = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/broadcast/g2/b", move |msg| async move {
        println!("client_b1 recv:{}", msg);
        assert_eq!(msg, "\"service send:\\\"hi\\\"\"");
        NON_SUB_COUNTER.fetch_add(1, Ordering::SeqCst);
        Some(
            TardisFuns::json
                .obj_to_string(&TardisWebsocketReq {
                    msg: json! {"client_b send:hi again"},
                    from_avatar: "b".to_string(),
                    ..Default::default()
                })
                .unwrap(),
        )
    })
    .await?;
    let non_sub_client_b2 = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/broadcast/g2/b", move |msg| async move {
        println!("client_b2 recv:{}", msg);
        assert_eq!(msg, "\"service send:\\\"hi\\\"\"");
        NON_SUB_COUNTER.fetch_add(1, Ordering::SeqCst);
        Some(
            TardisFuns::json
                .obj_to_string(&TardisWebsocketReq {
                    msg: json! {"client_b send:hi again"},
                    from_avatar: "b".to_string(),
                    ..Default::default()
                })
                .unwrap(),
        )
    })
    .await?;
    non_sub_client_a
        .send_obj(&TardisWebsocketReq {
            msg: json! {"hi"},
            from_avatar: "a".to_string(),
            ..Default::default()
        })
        .await?;
    non_sub_client_b1
        .send_obj(&TardisWebsocketReq {
            msg: json! {"hi"},
            from_avatar: "b".to_string(),
            ..Default::default()
        })
        .await?;
    non_sub_client_b2
        .send_obj(&TardisWebsocketReq {
            msg: json! {"hi"},
            from_avatar: "b".to_string(),
            ..Default::default()
        })
        .await?;

    sleep(Duration::from_millis(500)).await;
    assert_eq!(ERROR_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(SUB_COUNTER.load(Ordering::SeqCst), 4);
    assert_eq!(NON_SUB_COUNTER.load(Ordering::SeqCst), 3);

    Ok(())
}

struct Api;

#[poem_openapi::OpenApi]
impl Api {
    #[oai(path = "/ws/broadcast/:group/:name", method = "get")]
    async fn ws_broadcast(&self, group: Path<String>, name: Path<String>, websocket: WebSocket) -> BoxWebSocketUpgraded {
        if !SENDERS.read().await.contains_key(&group.0) {
            SENDERS.write().await.insert(group.0.clone(), tokio::sync::broadcast::channel::<String>(100).0);
        }
        let sender = SENDERS.read().await.get(&group.0).unwrap().clone();
        if group.0 == "g1" {
            ws_broadcast(
                vec![name.0],
                true,
                HashMap::new(),
                websocket,
                sender,
                |req_msg, _ext| async move {
                    println!("service g1 recv:{}:{}", req_msg.from_avatar, req_msg.msg);
                    if req_msg.msg == json! {"client_b send:hi again"} {
                        return None;
                    }
                    Some(TardisWebsocketResp {
                        msg: json! { format!("service send:{}", TardisFuns::json.json_to_string(req_msg.msg).unwrap())},
                        to_avatars: vec![],
                        ignore_avatars: vec![],
                    })
                },
                |_, _| async move {},
            )
        } else if group.0 == "g2" {
            ws_broadcast(
                vec![name.0],
                false,
                HashMap::new(),
                websocket,
                sender,
                |req_msg, _ext| async move {
                    println!("service g2 recv:{}:{}", req_msg.from_avatar, req_msg.msg);
                    if req_msg.msg == json! {"client_b send:hi again"} {
                        return None;
                    }
                    Some(TardisWebsocketResp {
                        msg: json! { format!("service send:{}", TardisFuns::json.json_to_string(req_msg.msg).unwrap())},
                        to_avatars: vec![],
                        ignore_avatars: vec![],
                    })
                },
                |_, _| async move {},
            )
        } else if group.0 == "gerror" {
            ws_broadcast(
                vec![name.0],
                false,
                HashMap::new(),
                websocket,
                sender,
                |req_msg, _ext| async move {
                    println!("service gerror recv:{}:{}", req_msg.from_avatar, req_msg.msg);
                    return None;
                },
                |_, _| async move {},
            )
        } else {
            ws_echo(
                name.0,
                HashMap::new(),
                websocket,
                |_, _, _| async move { Some(format!("Websocket connection error: group not found")) },
                |_, _| async move {},
            )
        }
    }
}
