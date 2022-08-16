use tardis::basic::error::TardisError;
use tardis::basic::locale::TardisLocale;
use tardis::basic::result::TardisResult;
use tardis::TardisFuns;

#[tokio::test]
async fn test_basic_locale() -> TardisResult<()> {
    TardisFuns::init("tests/config").await?;

    assert_eq!(TardisLocale::get_message("404", "", "zh-cn")?, "找不到资源");
    assert_eq!(TardisLocale::get_message("404", "", "en")?, "Not found resource");
    assert_eq!(TardisLocale::get_message("400xx", "default message", "en")?, "default message");
    // Un-matched
    assert_eq!(
        TardisLocale::get_message("404-m1-res-add", "Not found ## resource in m1", "zh-cn")?,
        "Not found ## resource in m1"
    );
    assert_eq!(
        TardisLocale::get_message("404-m1-res-add", "Not found RES1 resource in m1", "en")?,
        "Not found RES1 resource in m1"
    );
    assert_eq!(
        TardisLocale::get_message("404-m1-res-add", "Not found RES1 resource in m1", "zh-cn")?,
        "在m1中找不到[RES1]资源"
    );

    assert_eq!(TardisLocale::env_message("404-m1-res-add", "Not found RES1 resource in m1"), "在m1中找不到[RES1]资源");

    assert_eq!(
        TardisError::bad_request("Not found RES1 resource in m1", "404-m1-res-add").message,
        "在m1中找不到[RES1]资源"
    );

    let inst = TardisFuns::inst("m1".to_string(), Some("zh-CN".to_string()));

    assert_eq!(inst.err().not_found("res", "add", "Not found RES1 resource in m1", "").message, "在m1中找不到[RES1]资源");

    assert_eq!(
        inst.err().not_found("xx", "add", "Not found RES1 resource in m1", "").message,
        "Not found RES1 resource in m1"
    );

    assert_eq!(
        inst.err().not_found("xx", "add", "Not found RES1 resource in m1", "404-m1-res-add").message,
        "在m1中找不到[RES1]资源"
    );

    Ok(())
}
