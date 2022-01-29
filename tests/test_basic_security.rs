use tardis::basic::result::TardisResult;
use tardis::TardisFuns;

#[tokio::test]
async fn test_basic_security() -> TardisResult<()> {
    let b64_str = TardisFuns::security.base64.encode("测试");
    let str = TardisFuns::security.base64.decode(&b64_str).unwrap();
    assert_eq!(str, "测试");
    Ok(())
}
