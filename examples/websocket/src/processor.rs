use serde::{Deserialize, Serialize};
use tardis::basic::result::TardisResult;
use tardis::tokio::sync::broadcast::Sender;
use tardis::web::poem::web::websocket::BoxWebSocketUpgraded;
use tardis::web::poem::web::{websocket::WebSocket, Data, Path};
use tardis::web::poem_openapi::payload::Html;
use tardis::web::poem_openapi::{self};
use tardis::web::websocket::{ws_broadcast, ws_echo, TardisWebsocketResp};
use tardis::TardisFuns;

pub struct Page;

#[poem_openapi::OpenApi]
impl Page {
    #[oai(path = "/echo", method = "get")]
    async fn echo(&self) -> Html<&'static str> {
        Html(
            r###"
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
            ws = new WebSocket("ws://" + location.host + "/ws/echo/" + nameInput.value);
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
    "###,
        )
    }

    #[oai(path = "/broadcast", method = "get")]
    async fn broadcast(&self) -> Html<&'static str> {
        Html(
            r###"
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
            ws = new WebSocket("ws://" + location.host + "/ws/broadcast/" + nameInput.value);
            ws.onmessage = function(event) {
                msgsArea.value += event.data + "\r\n";
            }
        });
        
        sendForm.addEventListener("submit", function(event) {
            event.preventDefault();
            ws.send(JSON.stringify({"to": recNameInput.value, "msg": msgInput.value}));
            recNameInput.value = "";
            msgInput.value = "";
        });
    </script>
    "###,
        )
    }

    #[oai(path = "/ws/echo/:name", method = "get")]
    async fn ws_echo(&self, name: Path<String>, websocket: WebSocket) -> BoxWebSocketUpgraded {
        ws_echo(
            websocket,
            name.0,
            |req_session, msg| async move {
                let resp = format!("echo:{msg} by {req_session}");
                Some(resp)
            },
            |_| async move {},
        )
    }

    #[oai(path = "/ws/broadcast/:name", method = "get")]
    async fn ws_broadcast(&self, name: Path<String>, websocket: WebSocket, sender: Data<&Sender<String>>) -> BoxWebSocketUpgraded {
        ws_broadcast(
            websocket,
            sender,
            name.0,
            |req_session, msg| async move {
                let exmaple_msg = TardisFuns::json.str_to_obj::<WebsocketExample>(&msg).unwrap();
                Some(TardisWebsocketResp {
                    msg: TardisFuns::json.obj_to_string(&TardisResult::Ok(format!("echo:{}", exmaple_msg.msg))).unwrap(),
                    from_seesion: req_session,
                    to_seesions: if exmaple_msg.to.is_empty() { vec![] } else { vec![exmaple_msg.to] },
                    ignore_self: true,
                })
            },
            |_| async move {},
        )
    }
}

#[derive(Deserialize, Serialize)]
pub struct WebsocketExample {
    pub msg: String,
    pub to: String,
}
