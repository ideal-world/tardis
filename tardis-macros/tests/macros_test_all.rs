#[test]
fn test_all() {
    let t = trybuild::TestCases::new();
    t.pass("tests/create_dto_test/*.rs");
}
