/*!
Crate for primitive run-time type reflection

Information is generated entirely in compile-time using derive macro [`TypeInfo`]

Moreover, this crate mostly itends to operate with trait objects and [`std::any::Any`].

That's why there are two separate "main" traits: [`TypeInfo`] and [`TypeInfoDynamic`].

The former indents to provide info about type when the type can be named and is known and the latter
is for case if type is erased

# Examples
```
use reflectix::TypeInfoDynamic;

#[derive(reflectix::TypeInfo, Default)]
struct Foo{
    bar: i32
}

# fn main() {
# let foo = Foo::default();
 let foo_erased = &foo;
 assert_eq!(foo.get_dynamic().ident, "Foo");
# }

```
*/

pub use ::reflectix_core::*;

/// Derive-able implementation of [`TypeInfo`] and [`TypeInfoDynamic`]
///
/// Accepts both enum's and struct's
///
/// *Note*: That if any field type is compound (non-primitive), then you
/// must derive  [`TypeInfo`] for those types too
pub use reflectix_macros::TypeInfo;
