use tardis::basic::result::TardisResult;
use tardis::TardisFuns;

#[tokio::test]
async fn test_basic_field() -> TardisResult<()> {
    assert!(TardisFuns::field.is_phone("18657120202"));

    assert_eq!(TardisFuns::field.incr_by_base62("abcd1").unwrap(), "abcd2");
    assert_eq!(TardisFuns::field.incr_by_base62("abcd12").unwrap(), "abcd13");
    assert_eq!(TardisFuns::field.incr_by_base62("abcd9").unwrap(), "abceA");
    assert_eq!(TardisFuns::field.incr_by_base62("azzz9").unwrap(), "azz0A");
    assert_eq!(TardisFuns::field.incr_by_base62("a9999").unwrap(), "bAAAA");
    assert!(TardisFuns::field.incr_by_base62("999").is_none());

    assert_eq!(TardisFuns::field.incr_by_base36("abcd1").unwrap(), "abcd2");
    assert_eq!(TardisFuns::field.incr_by_base36("abcd12").unwrap(), "abcd13");
    assert_eq!(TardisFuns::field.incr_by_base36("abcd9").unwrap(), "abcea");
    assert_eq!(TardisFuns::field.incr_by_base36("azzz9").unwrap(), "azz0a");
    assert_eq!(TardisFuns::field.incr_by_base36("a9999").unwrap(), "baaaa");
    assert!(TardisFuns::field.incr_by_base36("999").is_none());

    assert!(TardisFuns::field.is_code_cs("Adw834_dfds"));
    assert!(!TardisFuns::field.is_code_cs(" Adw834_dfds"));
    assert!(!TardisFuns::field.is_code_cs("Adw834_d-fds"));
    assert!(TardisFuns::field.is_code_ncs("adon2_43323tr"));
    assert!(!TardisFuns::field.is_code_ncs("adon2_43323tr "));
    assert!(!TardisFuns::field.is_code_ncs("Adw834_dfds"));
    assert_eq!(TardisFuns::field.nanoid().len(), 21);
    assert_eq!(TardisFuns::field.nanoid_len(4).len(), 4);

    Ok(())
}
