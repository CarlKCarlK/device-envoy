#![allow(missing_docs)]

#[test]
fn ui_compile_fail() {
    let test_cases = trybuild::TestCases::new();
    test_cases.compile_fail("tests/ui/*.rs");
}
