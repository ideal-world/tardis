use tardis::basic::result::TardisResult;
use tardis::TardisFuns;

#[tokio::test]
async fn test_basic_uri() -> TardisResult<()> {
    assert_eq!(TardisFuns::uri.format("http://idealwrold.group").unwrap().to_string(), "http://idealwrold.group");
    assert_eq!(TardisFuns::uri.format("jdbc:h2:men:iam").unwrap().to_string(), "jdbc:h2:men:iam");
    assert_eq!(
        TardisFuns::uri.format("api://a1.t1/e1?q2=2&q1=1&q3=3").unwrap().to_string(),
        "api://a1.t1/e1?q1=1&q2=2&q3=3"
    );
    Ok(())
}
