// https://github.com/seanmonstar/reqwest

use reqwest::StatusCode;

use tardis::basic::config::{CacheConfig, DBConfig, FrameworkConfig, MQConfig, MailConfig, OSConfig, SearchConfig, TardisConfig};
use tardis::basic::result::TardisResult;
use tardis::serde::{Deserialize, Serialize};
use tardis::TardisFuns;

#[tokio::test]
async fn test_web_client() -> TardisResult<()> {
    TardisFuns::init_conf(TardisConfig {
        cs: Default::default(),
        fw: FrameworkConfig {
            app: Default::default(),
            web_server: Default::default(),
            web_client: Default::default(),
            cache: CacheConfig {
                enabled: false,
                ..Default::default()
            },
            db: DBConfig {
                enabled: false,
                ..Default::default()
            },
            mq: MQConfig {
                enabled: false,
                ..Default::default()
            },
            search: SearchConfig {
                enabled: false,
                ..Default::default()
            },
            mail: MailConfig {
                enabled: false,
                ..Default::default()
            },
            os: OSConfig {
                enabled: false,
                ..Default::default()
            },
            adv: Default::default(),
        },
    })
    .await?;

    let res = reqwest::get("http://httpbin.org/get").await?;
    assert_eq!(res.status(), StatusCode::OK);

    let response = TardisFuns::web_client().get_to_str("https://www.baidu.com", Some([("User-Agent".to_string(), "Tardis".to_string())].iter().cloned().collect())).await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());
    assert!(response.body.unwrap().contains("baidu"));

    let response = TardisFuns::web_client().get_to_str("http://httpbin.org/get", Some([("User-Agent".to_string(), "Tardis".to_string())].iter().cloned().collect())).await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());
    assert!(response.body.unwrap().contains("Tardis"));

    let response = TardisFuns::web_client()
        .delete_to_void(
            "https://httpbin.org/delete",
            Some([("User-Agent".to_string(), "Tardis".to_string())].iter().cloned().collect()),
        )
        .await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());

    let response = TardisFuns::web_client().post_str_to_str("https://httpbin.org/post", &"Raw body contents".to_string(), None).await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());
    assert!(response.body.unwrap().contains(r#"data": "Raw body contents"#));

    let response = TardisFuns::web_client().post_str_to_str("https://httpbin.org/post", &"Raw body contents".to_string(), None).await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());
    assert!(response.body.unwrap().contains(r#"data": "Raw body contents"#));

    let request = serde_json::json!({
        "lang": "rust",
        "body": "json"
    });
    let response = TardisFuns::web_client().post_obj_to_str("https://httpbin.org/post", &request, None).await?;
    assert_eq!(response.code, StatusCode::OK.as_u16());
    assert!(response.body.unwrap().contains(r#"data": "{\"body\":\"json\",\"lang\":\"rust\"}"#));

    let new_post = Post {
        id: None,
        title: "idealworld".into(),
        body: "http://idealworld.group/".into(),
        user_id: 1,
    };
    let response = TardisFuns::web_client().post::<Post, Post>("https://jsonplaceholder.typicode.com/posts", &new_post, None).await?;
    assert_eq!(response.code, StatusCode::CREATED.as_u16());
    assert_eq!(response.body.unwrap().body, "http://idealworld.group/");

    let response = TardisFuns::web_client().post_obj_to_str("https://jsonplaceholder.typicode.com/posts", &new_post, None).await?;
    assert_eq!(response.code, StatusCode::CREATED.as_u16());
    assert!(response.body.unwrap().contains("http://idealworld.group/"));

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct Post {
    id: Option<i32>,
    title: String,
    body: String,
    #[serde(rename = "userId")]
    user_id: i32,
}
