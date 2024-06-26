#[derive(reflectix::TypeInfo, Default)]
pub struct Foo {
    pub x: i32,
    pub y: i32,
}

pub fn modify_field_of_erased(obj: &mut dyn reflectix::TypeInfoDynamic) {
    let field = obj.field_mut("x".into()).unwrap();
    let ref_field = field.downcast_mut::<i32>().unwrap();
    *ref_field = 42;
}


#[test]
pub fn test_erased_mutation() {
    let mut foo = Foo::default();
    let erased = &mut foo;

    modify_field_of_erased(erased);

    assert_eq!(foo.x, 42);
}