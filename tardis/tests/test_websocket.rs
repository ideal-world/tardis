use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use poem::web::websocket::{BoxWebSocketUpgraded, WebSocket};
use poem::web::Data;
use poem_openapi::param::Path;
use tardis::basic::result::TardisResult;
use tardis::web::web_server::TardisWebServer;
use tardis::web::ws_processor::{ws_broadcast, TardisWebsocketResp};
use tardis::TardisFuns;
use tokio::sync::broadcast::Sender;
use tokio::time::sleep;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

#[tokio::test]
async fn test_websocket() -> TardisResult<()> {
    tokio::spawn(async {
        let serv = TardisWebServer::init_simple("127.0.0.1", 8080).unwrap();
        serv.add_route_with_ws(Api, 100).await;
        serv.start().await
    });
    sleep(Duration::from_millis(500)).await;

    let mut client_a = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/broadcast/a", move |msg| async move {
        println!("client_a recv:{}", msg);
        assert_eq!(msg, "service send:hi");
        COUNTER.fetch_add(1, Ordering::SeqCst);
        None
    })
    .await?;
    let mut client_b = TardisFuns::ws_client("ws://127.0.0.1:8080/ws/broadcast/b", move |msg| async move {
        println!("client_b recv:{}", msg);
        assert_eq!(msg, "service send:hi");
        COUNTER.fetch_add(1, Ordering::SeqCst);
        Some(format!("client_b send:hi again"))
    })
    .await?;
    client_a.send("hi".to_string()).await?;
    client_b.send("hi".to_string()).await?;

    sleep(Duration::from_millis(500)).await;
    assert_eq!(COUNTER.load(Ordering::SeqCst), 6);

    Ok(())
}

struct Api;

#[poem_openapi::OpenApi]
impl Api {
    #[oai(path = "/ws/broadcast/:name", method = "get")]
    async fn ws_broadcast(&self, name: Path<String>, websocket: WebSocket, sender: Data<&Sender<String>>) -> BoxWebSocketUpgraded {
        ws_broadcast(
            "default".to_string(),
            websocket,
            sender.clone(),
            name.0,
            true,
            HashMap::new(),
            |req_session, msg, _ext| async move {
                println!("service recv:{}:{}", req_session, msg);
                if msg == "client_b send:hi again" {
                    COUNTER.fetch_add(1, Ordering::SeqCst);
                    return None;
                }
                Some(TardisWebsocketResp {
                    msg: format!("service send:{msg}"),
                    from_seesion: req_session,
                    to_seesions: vec![],
                    ignore_self: false,
                })
            },
            |_, _| async move {},
        )
    }
}
