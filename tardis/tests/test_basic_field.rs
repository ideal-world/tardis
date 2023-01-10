use tardis::basic::field::TrimString;
use tardis::basic::result::TardisResult;
use tardis::TardisFuns;

#[tokio::test]
async fn test_basic_field() -> TardisResult<()> {
    assert!(TardisFuns::field.is_phone("18657120202"));

    assert_eq!(TardisFuns::field.incr_by_base36("abcd1").unwrap(), "abcd2");
    assert_eq!(TardisFuns::field.incr_by_base36("abcd12").unwrap(), "abcd13");
    assert_eq!(TardisFuns::field.incr_by_base36("abcd9").unwrap(), "abcda");
    assert_eq!(TardisFuns::field.incr_by_base36("0000").unwrap(), "0001");
    assert_eq!(TardisFuns::field.incr_by_base36("000z").unwrap(), "0010");
    assert_eq!(TardisFuns::field.incr_by_base36("azzzy").unwrap(), "azzzz");
    assert_eq!(TardisFuns::field.incr_by_base36("azzzz").unwrap(), "b0000");
    assert!(TardisFuns::field.incr_by_base36("zzz").is_none());

    assert_eq!(TardisFuns::field.incr_by_base62("abcd1").unwrap(), "abcd2");
    assert_eq!(TardisFuns::field.incr_by_base62("abcd12").unwrap(), "abcd13");
    assert_eq!(TardisFuns::field.incr_by_base62("abcd9").unwrap(), "abcdA");
    assert_eq!(TardisFuns::field.incr_by_base62("abcdz").unwrap(), "abce0");
    assert_eq!(TardisFuns::field.incr_by_base62("azZzz").unwrap(), "aza00");
    assert_eq!(TardisFuns::field.incr_by_base62("azzzz").unwrap(), "b0000");
    assert!(TardisFuns::field.incr_by_base62("zzz").is_none());

    assert!(TardisFuns::field.is_code_cs("Adw834_dfds"));
    assert!(!TardisFuns::field.is_code_cs(" Adw834_dfds"));
    assert!(!TardisFuns::field.is_code_cs("Adw834_d-fds"));
    assert!(TardisFuns::field.is_code_ncs("adon2_43323tr"));
    assert!(!TardisFuns::field.is_code_ncs("adon2_43323tr "));
    assert!(!TardisFuns::field.is_code_ncs("Adw834_dfds"));
    assert_eq!(TardisFuns::field.nanoid().len(), 21);
    assert_eq!(TardisFuns::field.nanoid_len(4).len(), 4);

    let ts = TrimString(" a ".to_string());
    assert_eq!(ts.0, " a ");
    let s: &str = &ts;
    assert_eq!(s, "a");
    let ts: TrimString = " a ".into();
    let s: &str = &ts;
    assert_eq!(s, "a");
    let ts: TrimString = " a ".to_string().into();
    let s: &str = &ts;
    assert_eq!(s, "a");

    Ok(())
}
