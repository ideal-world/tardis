#[test]
fn test_all() {
    let t = trybuild::TestCases::new();
    t.pass("tests/create_dto_test/create_entity_test.rs");
    t.compile_fail("tests/create_dto_test/fail_cases/*.rs");
}
