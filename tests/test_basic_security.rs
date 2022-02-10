use tardis::basic::result::TardisResult;
use tardis::basic::security::Algorithm::{HmacSha1, HmacSha265, HmacSha512, MD5, SHA1, SHA256, SHA512};
use tardis::TardisFuns;

#[tokio::test]
async fn test_basic_security() -> TardisResult<()> {
    let b64_str = TardisFuns::security.base64.encode("测试");
    let str = TardisFuns::security.base64.decode(&b64_str).unwrap();
    assert_eq!(str, "测试");

    assert_eq!(TardisFuns::security.digest("测试", None, MD5).unwrap(), "db06c78d1e24cf708a14ce81c9b617ec");
    assert_eq!(TardisFuns::security.digest("测试", None, SHA1).unwrap(), "0b5d7ed54bee16756a7579c6718ab01e3d1b75eb");
    assert_eq!(
        TardisFuns::security.digest("测试", None, SHA256).unwrap(),
        "6aa8f49cc992dfd75a114269ed26de0ad6d4e7d7a70d9c8afb3d7a57a88a73ed"
    );
    assert_eq!(
        TardisFuns::security.digest("测试", None, SHA512).unwrap(),
        "98fb26ea83ce0f08918c967392a26ab298740aff3c18d032983b88bcee2e16d152ef372778259ebd529ed01701ff01ac4c95ed94e3a1ab9272ab98daf11f076c"
    );
    assert_eq!(TardisFuns::security.digest("测试", Some("pwd"), HmacSha1).unwrap(), "0e+vxZN90mgzsju6KCbS2EJ8Us4=");
    assert_eq!(
        TardisFuns::security.digest("测试", Some("pwd"), HmacSha265).unwrap(),
        "4RnnEGA9fWaf/4mnWSQbJsdtsCXeXdUddSZUmXe6qn4="
    );
    assert_eq!(
        TardisFuns::security.digest("测试", Some("pwd"), HmacSha512).unwrap(),
        "wO2937bb3tY/zLxUped257He0QMWywTsyhf2ELB3YWJmCgN4rS5a6+yWS852MC1LZ5HRd3AQjlUSOUUYKk0p9w=="
    );

    Ok(())
}
