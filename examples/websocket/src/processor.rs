use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tardis::basic::result::TardisResult;
use tardis::tokio::sync::broadcast::Sender;
use tardis::web::poem::web::websocket::BoxWebSocketUpgraded;
use tardis::web::poem::web::{websocket::WebSocket, Data, Path};
use tardis::web::poem_openapi::payload::Html;
use tardis::web::poem_openapi::{self};
use tardis::web::ws_processor::{ws_broadcast, ws_echo, TardisWebsocketMgrMessage, TardisWebsocketResp, WsHooks};
use tardis::TardisFuns;
#[derive(Debug, Clone)]
pub struct Page;

#[poem_openapi::OpenApi]
impl Page {
    #[oai(path = "/echo", method = "get")]
    async fn echo(&self) -> Html<&'static str> {
        Html(
            r##"
    <body>
        <form id="loginForm">
            Name: <input id="nameInput" type="text" />
            <button type="submit">Login</button>
        </form>
        
        <form id="sendForm" hidden>
            Text: <input id="msgInput" type="text" />
            <button type="submit">Send</button>
        </form>
        
        <textarea id="msgsArea" cols="50" rows="30" hidden></textarea>
    </body>
    <script>
        let ws;
        const loginForm = document.querySelector("#loginForm");
        const sendForm = document.querySelector("#sendForm");
        const nameInput = document.querySelector("#nameInput");
        const msgInput = document.querySelector("#msgInput");
        const msgsArea = document.querySelector("#msgsArea");
        
        nameInput.focus();
        loginForm.addEventListener("submit", function(event) {
            event.preventDefault();
            loginForm.hidden = true;
            sendForm.hidden = false;
            msgsArea.hidden = false;
            msgInput.focus();
            ws = new WebSocket("wss://" + location.host + "/ws/echo/" + nameInput.value);
            ws.onmessage = function(event) {
                msgsArea.value += event.data + "\r\n";
            }
        });
        
        sendForm.addEventListener("submit", function(event) {
            event.preventDefault();
            ws.send(msgInput.value);
            msgInput.value = "";
        });
    </script>
    "##,
        )
    }

    #[oai(path = "/broadcast", method = "get")]
    async fn broadcast(&self) -> Html<&'static str> {
        Html(
            r##"
    <body>
        <form id="loginForm">
            Name: <input id="nameInput" type="text" />
            <button type="submit">Login</button>
        </form>
        
        <form id="sendForm" hidden>
            Text: <input id="msgInput" type="text" /> Receiver name: <input id="recNameInput" type="text" />
            <button type="submit">Send</button>
        </form>
        
        <textarea id="msgsArea" cols="50" rows="30" hidden></textarea>
    </body>
    <script>
        let ws;
        const loginForm = document.querySelector("#loginForm");
        const sendForm = document.querySelector("#sendForm");
        const nameInput = document.querySelector("#nameInput");
        const msgInput = document.querySelector("#msgInput");
        const recNameInput = document.querySelector("#recNameInput");
        const msgsArea = document.querySelector("#msgsArea");
        
        nameInput.focus();
        loginForm.addEventListener("submit", function(event) {
            event.preventDefault();
            loginForm.hidden = true;
            sendForm.hidden = false;
            msgsArea.hidden = false;
            msgInput.focus();
            ws = new WebSocket("wss://" + location.host + "/ws/broadcast/" + nameInput.value);
            ws.onmessage = function(event) {
                msgsArea.value += event.data + "\r\n";
            }
        });
        
        sendForm.addEventListener("submit", function(event) {
            event.preventDefault();
            ws.send(JSON.stringify({"from_avatar": nameInput.value, "msg": {"to": recNameInput.value, "msg": msgInput.value}}));
            recNameInput.value = "";
            msgInput.value = "";
        });
    </script>
    "##,
        )
    }

    #[oai(path = "/ws/echo/:name", method = "get")]
    async fn ws_echo(&self, name: Path<String>, websocket: WebSocket) -> BoxWebSocketUpgraded {
        ws_echo(
            name.0,
            HashMap::new(),
            websocket,
            |req_session, msg, _| async move {
                let resp = format!("echo:{msg} by {req_session}");
                Some(resp)
            },
            |_, _| async move {},
        )
    }

    #[oai(path = "/ws/broadcast/:name", method = "get")]
    async fn ws_broadcast(&self, name: Path<String>, websocket: WebSocket, sender: Data<&Sender<TardisWebsocketMgrMessage>>) -> BoxWebSocketUpgraded {
        pub struct Hooks {
            ext: HashMap<String, String>,
        }
        impl WsHooks for Hooks {
            async fn on_process(&self, req: tardis::web::ws_processor::TardisWebsocketReq) -> Option<TardisWebsocketResp> {
                let example_msg = TardisFuns::json.json_to_obj::<WebsocketExample>(req.msg).unwrap();
                Some(TardisWebsocketResp {
                    msg: TardisFuns::json.obj_to_json(&TardisResult::Ok(format!("echo:{}, ext info:{}", example_msg.msg, self.ext.get("some_key").unwrap()))).unwrap(),
                    to_avatars: if example_msg.to.is_empty() { vec![] } else { vec![example_msg.to] },
                    ignore_avatars: vec![],
                })
            }
        }
        let hooks = Hooks {
            ext: HashMap::from([("some_key".to_string(), "ext_value".to_string())]),
        };
        ws_broadcast(vec![name.0], false, true, websocket, sender.clone(), hooks).await
    }
}

#[derive(Deserialize, Serialize)]
pub struct WebsocketExample {
    pub msg: String,
    pub to: String,
}
