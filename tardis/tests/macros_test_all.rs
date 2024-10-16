#[test]
fn test_all() {
    let t = trybuild::TestCases::new();
    t.pass("tests/macros_tests/pass_cases/*.rs");
    // t.compile_fail("tests/macros_tests/fail_cases/*.rs");
}
