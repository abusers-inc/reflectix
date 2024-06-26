use reflectix::*;

#[derive(reflectix::TypeInfo)]
pub struct Test {
    a: i32,
    b: u32,
}

#[test]
pub fn test_name() {
    assert_eq!(Test::INFO.ident, "Test")
}
