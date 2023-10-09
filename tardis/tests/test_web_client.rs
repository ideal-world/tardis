// https://github.com/seanmonstar/reqwest

use std::env;

use reqwest::StatusCode;

use tardis::basic::result::TardisResult;
use tardis::config::config_dto::{FrameworkConfig, TardisConfig, WebClientConfig};
use tardis::serde::{Deserialize, Serialize};
use tardis::TardisFuns;

#[tokio::test(flavor = "multi_thread")]
async fn test_web_client() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis=trace");
    TardisFuns::init_conf(TardisConfig {
        cs: Default::default(),
        fw: FrameworkConfig::builder().web_client(WebClientConfig::default()).build(),
    })
    .await?;

    let res = reqwest::get("https://postman-echo.com/get").await?;
    assert_eq!(res.status(), StatusCode::OK);

    let response = TardisFuns::web_client().get_to_str("https://www.baidu.com", [("User-Agent", "Tardis")]).await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());
    assert!(response.body.unwrap().contains("baidu"));

    let response = TardisFuns::web_client().get_to_str("https://postman-echo.com/get", [("User-Agent", "Tardis")]).await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());
    assert!(response.body.unwrap().contains("Tardis"));

    let response = TardisFuns::web_client().delete_to_void("https://postman-echo.com/delete", [("User-Agent", "Tardis")]).await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());

    let response = TardisFuns::web_client().post_str_to_str("https://postman-echo.com/post", "Raw body contents", None).await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());
    assert!(response.body.unwrap().contains(r#"data": "Raw body contents"#));

    let response = TardisFuns::web_client().post_str_to_str("https://postman-echo.com/post", "Raw body contents", None).await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());
    assert!(response.body.unwrap().contains(r#"data": "Raw body contents"#));

    let request = serde_json::json!({
        "lang": "rust",
        "body": "json"
    });
    let response = TardisFuns::web_client().post::<_, EchoPostResponse<serde_json::Value>>("https://postman-echo.com/post", &request, None).await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());
    assert_eq!(response.body.unwrap().data, request);

    let new_post = Post {
        id: None,
        title: "idealworld".into(),
        body: "http://idealworld.group/".into(),
        user_id: 1,
    };
    let response = TardisFuns::web_client().post::<Post, EchoPostResponse<Post>>("https://postman-echo.com/post", &new_post, None).await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());
    assert_eq!(response.body.unwrap().data.body, "http://idealworld.group/");

    let response = TardisFuns::web_client().post_obj_to_str("https://postman-echo.com/post", &new_post, None).await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());
    assert!(response.body.unwrap().contains("http://idealworld.group/"));

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct EchoPostResponse<T> {
    data: T,
}

#[derive(Debug, Serialize, Deserialize)]
struct Post {
    id: Option<i32>,
    title: String,
    body: String,
    #[serde(rename = "userId")]
    user_id: i32,
}
