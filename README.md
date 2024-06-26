# Reflectix

[![Crates.io](https://img.shields.io/crates/v/reflectix.svg)](https://crates.io/crates/reflectix)
[![Documentation](https://docs.rs/reflectix/badge.svg)](https://docs.rs/reflectix)
[![License](https://img.shields.io/crates/l/reflectix.svg)](https://github.com/abusers-inc/reflectix/blob/main/LICENSE)

## Overview

Reflectix is a Rust crate that provides primitive runtime type reflection. It allows you to inspect and manipulate types at runtime, enabling dynamic behavior in your Rust programs.

## Example

```rust
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


pub fn main() {
    let mut foo = Foo::default();
    let erased = &mut foo;

    modify_field_of_erased(erased);

    assert_eq!(foo.x, 42);
}

```

## Features

- **Type Reflection**: Reflectix allows you to reflect on types at runtime, providing information about their structure, fields, and more.

- **Type Manipulation**: Reflectix provides utilities for manipulating types at runtime, such as creating new instances, modifying existing objects, and inspecting type hierarchies.

## Installation

To use Reflectix in your Rust project, add the following line to your `Cargo.toml` file:

```toml
[dependencies]
Reflectix = "0.1"
```

