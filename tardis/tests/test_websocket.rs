use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use poem::web::websocket::{BoxWebSocketUpgraded, WebSocket};
use poem_openapi::param::Path;
use serde_json::json;
use tardis::basic::result::TardisResult;
use tardis::web::web_server::{TardisWebServer, WebServerModule};
use tardis::web::ws_client::TardisWebSocketMessageExt;
use tardis::web::ws_processor::{
    ws_broadcast, ws_echo, TardisWebsocketInstInfo, TardisWebsocketMessage, TardisWebsocketMgrMessage, TardisWebsocketReq, TardisWebsocketResp, WS_SYSTEM_EVENT_AVATAR_ADD,
    WS_SYSTEM_EVENT_AVATAR_DEL, WS_SYSTEM_EVENT_INFO,
};
use tardis::TardisFuns;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn test_websocket() -> TardisResult<()> {
    let serv = TardisWebServer::init_simple("127.0.0.1", 8080).unwrap();
    serv.add_route(WebServerModule::from(Api).with_ws(100)).await;
    serv.start().await?;
    sleep(Duration::from_millis(500)).await;

    test_normal().await?;
    test_dyn_avatar().await?;

    Ok(())
}

async fn test_normal() -> TardisResult<()> {
    static ERROR_COUNTER: AtomicUsize = AtomicUsize::new(0);
    static SUB_COUNTER: AtomicUsize = AtomicUsize::new(0);
    static NON_SUB_COUNTER: AtomicUsize = AtomicUsize::new(0);

    // message not illegal test
    let error_client_a = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/broadcast/gerror/a", move |msg| async move {
        if let Message::Text(msg) = msg {
            println!("client_not_found recv:{}", msg);
            assert_eq!(msg, r#"{"msg":"message not illegal","event":"__sys_error__"}"#);
            ERROR_COUNTER.fetch_add(1, Ordering::SeqCst);
        }
        None
    })
    .await?;
    error_client_a.send_text("hi".to_string()).await?;
    // not found test
    let error_client_b = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/broadcast/gxxx/a", move |msg| async move {
        if let Message::Text(msg) = msg {
            println!("client_not_found recv:{}", msg);
            assert_eq!(msg, "Websocket connection error: group not found");
            ERROR_COUNTER.fetch_add(1, Ordering::SeqCst);
        }
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
        if let Message::Text(msg) = msg {
            println!("client_a recv:{}", msg);
            assert_eq!(msg, r#"{"msg":"service send:\"hi\"","event":null}"#);
            SUB_COUNTER.fetch_add(1, Ordering::SeqCst);
        }
        None
    })
    .await?;
    let sub_client_b1 = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/broadcast/g1/b", move |msg| async move {
        if let Message::Text(msg) = msg {
            println!("client_b1 recv:{}", msg);
            assert_eq!(msg, r#"{"msg":"service send:\"hi\"","event":null}"#);
            SUB_COUNTER.fetch_add(1, Ordering::SeqCst);
            Some(Message::Text(
                TardisFuns::json
                    .obj_to_string(&TardisWebsocketReq {
                        msg: json! {"client_b send:hi again"},
                        from_avatar: "b".to_string(),
                        ..Default::default()
                    })
                    .unwrap(),
            ))
        } else {
            None
        }
    })
    .await?;
    let sub_client_b2 = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/broadcast/g1/b", move |msg| async move {
        if let Message::Text(msg) = msg {
            println!("client_b2 recv:{}", msg);
            assert_eq!(msg, r#"{"msg":"service send:\"hi\"","event":null}"#);
            SUB_COUNTER.fetch_add(1, Ordering::SeqCst);
            Some(Message::Text(
                TardisFuns::json
                    .obj_to_string(&TardisWebsocketReq {
                        msg: json! {"client_b send:hi again"},
                        from_avatar: "b".to_string(),
                        ..Default::default()
                    })
                    .unwrap(),
            ))
        } else {
            None
        }
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
        if let Message::Text(msg) = msg {
            println!("client_a recv:{}", msg);
            assert_eq!(msg, r#"{"msg":"service send:\"hi\"","event":null}"#);
            NON_SUB_COUNTER.fetch_add(1, Ordering::SeqCst);
        }
        None
    })
    .await?;
    let non_sub_client_b1 = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/broadcast/g2/b", move |msg| async move {
        if let Message::Text(msg) = msg {
            println!("client_b1 recv:{}", msg);
            assert_eq!(msg, r#"{"msg":"service send:\"hi\"","event":null}"#);
            NON_SUB_COUNTER.fetch_add(1, Ordering::SeqCst);
            Some(Message::Text(
                TardisFuns::json
                    .obj_to_string(&TardisWebsocketReq {
                        msg: json! {"client_b send:hi again"},
                        from_avatar: "b".to_string(),
                        ..Default::default()
                    })
                    .unwrap(),
            ))
        } else {
            None
        }
    })
    .await?;
    let non_sub_client_b2 = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/broadcast/g2/b", move |msg| async move {
        if let Message::Text(msg) = msg {
            println!("client_b2 recv:{}", msg);
            assert_eq!(msg, r#"{"msg":"service send:\"hi\"","event":null}"#);
            NON_SUB_COUNTER.fetch_add(1, Ordering::SeqCst);
            Some(Message::Text(
                TardisFuns::json
                    .obj_to_string(&TardisWebsocketReq {
                        msg: json! {"client_b send:hi again"},
                        from_avatar: "b".to_string(),
                        ..Default::default()
                    })
                    .unwrap(),
            ))
        } else {
            None
        }
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
    assert_eq!(ERROR_COUNTER.load(Ordering::SeqCst), 2);
    assert_eq!(SUB_COUNTER.load(Ordering::SeqCst), 6);
    assert_eq!(NON_SUB_COUNTER.load(Ordering::SeqCst), 5);

    Ok(())
}

async fn test_dyn_avatar() -> TardisResult<()> {
    static INFO_COUNTER: AtomicUsize = AtomicUsize::new(0);
    static ADD_COUNTER: AtomicUsize = AtomicUsize::new(0);
    static DEL_COUNTER: AtomicUsize = AtomicUsize::new(0);

    TardisFuns::ws_client("ws://127.0.0.1:8080/ws/dyn/_/true", move |msg| async move {
        let receive_msg = msg.str_to_obj::<TardisWebsocketMgrMessage>().unwrap();
        if receive_msg.event == Some(WS_SYSTEM_EVENT_AVATAR_ADD.to_string()) && receive_msg.msg.as_str().unwrap() == "c" {
            ADD_COUNTER.fetch_add(1, Ordering::SeqCst);
            let from_avatar = receive_msg.from_avatar.clone();
            return Some(Message::Text(
                TardisFuns::json.obj_to_string(&receive_msg.into_req(json! {"c"}, from_avatar.clone(), Some(vec![from_avatar]))).unwrap(),
            ));
        }
        if receive_msg.event == Some(WS_SYSTEM_EVENT_AVATAR_DEL.to_string()) && receive_msg.msg.as_str().unwrap() == "c" {
            assert!(1 == 2);
            DEL_COUNTER.fetch_add(1, Ordering::SeqCst);
        }
        None
    })
    .await?;

    TardisFuns::ws_client("ws://127.0.0.1:8080/ws/dyn/a/false", move |msg| async move {
        let receive_msg = TardisFuns::json.str_to_obj::<TardisWebsocketMessage>(msg.to_text().unwrap()).unwrap();
        if receive_msg.event == Some(WS_SYSTEM_EVENT_AVATAR_ADD.to_string()) && receive_msg.msg.as_str().unwrap() == "c" {
            assert!(1 == 2);
            ADD_COUNTER.fetch_add(1, Ordering::SeqCst);
        }
        if receive_msg.event == Some(WS_SYSTEM_EVENT_AVATAR_DEL.to_string()) && receive_msg.msg.as_str().unwrap() == "c" {
            assert!(1 == 2);
            DEL_COUNTER.fetch_add(1, Ordering::SeqCst);
        }
        None
    })
    .await?;

    let a_client = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/dyn/a/false", move |msg| async move {
        let receive_msg = TardisFuns::json.str_to_obj::<TardisWebsocketMessage>(msg.to_text().unwrap()).unwrap();
        if receive_msg.event == Some(WS_SYSTEM_EVENT_AVATAR_ADD.to_string()) && receive_msg.msg.as_str().unwrap() == "c" {
            assert!(1 == 2);
            ADD_COUNTER.fetch_add(1, Ordering::SeqCst);
        }
        if receive_msg.event == Some(WS_SYSTEM_EVENT_AVATAR_DEL.to_string()) && receive_msg.msg.as_str().unwrap() == "c" {
            assert!(1 == 2);
            DEL_COUNTER.fetch_add(1, Ordering::SeqCst);
        }
        if receive_msg.event == Some(WS_SYSTEM_EVENT_INFO.to_string()) {
            let info_msg = TardisFuns::json.json_to_obj::<TardisWebsocketInstInfo>(receive_msg.msg).unwrap();
            assert_eq!(info_msg.avatars, vec!["c"]);
            INFO_COUNTER.fetch_add(1, Ordering::SeqCst);
        }
        None
    })
    .await?;

    TardisFuns::ws_client("ws://127.0.0.1:8080/ws/dyn/a/false", move |msg| async move {
        let receive_msg = TardisFuns::json.str_to_obj::<TardisWebsocketMessage>(msg.to_text().unwrap()).unwrap();
        if receive_msg.msg.as_str().unwrap() == "a" {
            ADD_COUNTER.fetch_add(1, Ordering::SeqCst);
            assert!(1 == 2);
        }
        None
    })
    .await?;

    // add avatar
    a_client
        .send_obj(&TardisWebsocketReq {
            msg: json! {"c"},
            from_avatar: "a".to_string(),
            to_avatars: Some(vec!["_".to_string()]),
            event: Some(WS_SYSTEM_EVENT_AVATAR_ADD.to_string()),
            ..Default::default()
        })
        .await?;

    // del avatar
    a_client
        .send_obj(&TardisWebsocketReq {
            msg: json! {"a"},
            from_avatar: "a".to_string(),
            event: Some(WS_SYSTEM_EVENT_AVATAR_DEL.to_string()),
            ..Default::default()
        })
        .await?;

    // fetch info
    a_client
        .send_obj(&TardisWebsocketReq {
            msg: json! {""},
            from_avatar: "c".to_string(),
            event: Some(WS_SYSTEM_EVENT_INFO.to_string()),
            ..Default::default()
        })
        .await?;

    sleep(Duration::from_millis(500)).await;
    assert_eq!(ADD_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(INFO_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(DEL_COUNTER.load(Ordering::SeqCst), 0);

    Ok(())
}

#[derive(Debug, Clone)]
struct Api;

#[poem_openapi::OpenApi]
impl Api {
    #[oai(path = "/ws/broadcast/:group/:name", method = "get")]
    async fn ws_broadcast(&self, group: Path<String>, name: Path<String>, websocket: WebSocket) -> BoxWebSocketUpgraded {
        if group.0 == "g1" {
            ws_broadcast(
                &group.0,
                vec![name.0],
                false,
                true,
                HashMap::new(),
                websocket,
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
            .await
        } else if group.0 == "g2" {
            ws_broadcast(
                &group.0,
                vec![name.0],
                false,
                false,
                HashMap::new(),
                websocket,
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
            .await
        } else if group.0 == "gerror" {
            ws_broadcast(
                &group.0,
                vec![name.0],
                false,
                false,
                HashMap::new(),
                websocket,
                |req_msg, _ext| async move {
                    println!("service gerror recv:{}:{}", req_msg.from_avatar, req_msg.msg);
                    None
                },
                |_, _| async move {},
            )
            .await
        } else {
            ws_echo(
                name.0,
                HashMap::new(),
                websocket,
                |_, _, _| async move { Some("Websocket connection error: group not found".to_string()) },
                |_, _| async move {},
            )
        }
    }

    #[oai(path = "/ws/dyn/:name/:mgr", method = "get")]
    async fn ws_dyn_broadcast(&self, name: Path<String>, mgr: Path<bool>, websocket: WebSocket) -> BoxWebSocketUpgraded {
        ws_broadcast(
            "dyn",
            vec![name.0],
            mgr.0,
            true,
            HashMap::new(),
            websocket,
            |req_msg, _ext| async move {
                Some(TardisWebsocketResp {
                    msg: req_msg.msg,
                    to_avatars: req_msg.to_avatars.unwrap_or(vec![]),
                    ignore_avatars: vec![],
                })
            },
            |_, _| async move {},
        )
        .await
    }
}
