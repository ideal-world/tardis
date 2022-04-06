use tardis::basic::result::TardisResult;
use tardis::serde::{self, Deserialize, Serialize};
use tardis::serde_json;
use tardis::tokio;
use tardis::TardisFuns;

#[tokio::main]
async fn main() -> TardisResult<()> {
    // Initial configuration
    TardisFuns::init("").await?;

    // Simple get request
    let response = TardisFuns::web_client().get_to_str("http://httpbin.org/get", None).await?;
    assert_eq!(response.code, 200);

    // Simple post request
    let request = serde_json::json!({
        "lang": "rust",
        "body": "json"
    });
    let response = TardisFuns::web_client().post_obj_to_str("https://httpbin.org/post", &request, None).await?;
    assert_eq!(response.code, 200);
    assert!(response.body.unwrap().contains(r#"data": "{\"body\":\"json\",\"lang\":\"rust\"}"#));

    // Simple post request
    let new_post = Post {
        id: None,
        title: "idealworld".into(),
        body: "http://idealworld.group/".into(),
        user_id: 1,
    };
    let response = TardisFuns::web_client().post::<Post, Post>("https://jsonplaceholder.typicode.com/posts", &new_post, None).await?;
    assert_eq!(response.code, 201);
    assert_eq!(response.body.unwrap().body, "http://idealworld.group/");

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "self::serde")]
struct Post {
    id: Option<i32>,
    title: String,
    body: String,
    #[serde(rename = "userId")]
    user_id: i32,
}
