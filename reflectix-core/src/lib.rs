#![deny(missing_docs)]
#![allow(missing_docs)]

/// Information about type fields (if there is any)
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Fields {
    /// Type is structure-like and has fields with names
    Named(&'static [Field]),

    /// Type is tuple-like and has fields that can be referred by their index (just like you would be accessing plain tuple)
    ///
    Indexed(&'static [Field]),

    /// Type is unit and doesn't have any fields
    Unit,
}

/// Information about data contained within type
///
/// [`Data::Primitive`] is special case for fundamental rust types.
/// This crate assumes that your types will be built using those.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Data {
    /// Fundamental type, which doesn't have any fields. You **can't** define types with this kind of data
    Primitive,
    /// Struct-like, can be tuple struct or default struct
    Struct(Fields),

    /// Variants of this enum
    Enum(Variants),

    /// Unit type, which means that type doesn't have any fields.
    ///
    /// **Note**: that this differs from [`Data::Primitive`] semantic meaning: you can define types which hold this data
    Unit,
}

/// Discriminant of particular field
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FieldId {
    /// Index of field in tuple-like type
    Index(usize),
    /// Name of target field
    Named(&'static str),
}

/// Field of type
///
/// Single structure for fields of tuple-like types and fields of structures
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Field {
    /// Identifier of field inside related type
    ///
    /// Acts as path in filesystem
    pub id: FieldId,
    /// Associated info of field's type
    pub ty: &'static Type,
}
impl From<&'static str> for FieldId {
    fn from(s: &'static str) -> Self {
        FieldId::Named(s)
    }
}

impl From<usize> for FieldId {
    fn from(i: usize) -> Self {
        FieldId::Index(i)
    }
}


/// Variant of enum type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Variant {
    #[allow(missing_docs)]
    pub ident: &'static str,
    #[allow(missing_docs)]
    pub fields: Fields,
}
#[allow(missing_docs)]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Variants {
    #[allow(missing_docs)]
    pub variants: &'static [Variant],
}

/// Information about type
///
/// if [`TypeInfo`] is implemented, comes as associated constant
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Type {
    /// Type name, exactly as in code (case and underscores are preserved)
    pub ident: &'static str,
    /// Type of data that this type contains
    pub data: Data,
}

/// If attempt to borrow field was incorrect
#[derive(thiserror::Error, Debug)]
pub enum FieldAccessError {
    /// Requested type doesn't actually match that of the field
    #[error("Requested type doesn't match actual type")]
    UnmatchingType,

    /// If there were an attempt to access field in unit type
    #[error("Attempt to access field in unit type/variant")]
    Unit,

    /// If accessing field that is not present in type
    #[error("Field not found")]
    NotFound,
}

/// Failure of type construction
#[derive(thiserror::Error, Debug)]
pub enum RuntimeConstructError {
    /// Attempted to construct primitive type
    #[error("Can't construct primitive type")]
    Primitive,

    /// Invalid type was passed as argument to constructor
    #[error("Invalid type at {index} was passed to runtime constructor")]
    UnexpectedType {
        #[allow(missing_docs)]
        index: usize,
        /// Name of expected type
        expected: &'static str,
    },

    /// Related enum doesn't have requested variant
    #[error("Requested variant doesn't exist")]
    InvalidVariant,

    #[error("Some fields of this type are not public")]
    #[allow(missing_docs)]
    PrivateFields,

    #[error("Called `construct_struct` on enum type")]
    #[allow(missing_docs)]
    NotStruct,

    #[error("Called `construct_enum` on a struct type")]
    #[allow(missing_docs)]
    NotEnum,

    #[error("Not enough arguments were passed")]
    #[allow(missing_docs)]
    NotEnoughArgs,
}

/// Object-safe version of [`TypeInfo`]
///
/// Additionally provides ability to construct type (if it's not a enum without variants),
/// and ability to borrow (both immutably and mutably) fields
pub trait TypeInfoDynamic: std::any::Any {
    /// Get [`Type`] information for this type
    ///
    /// Because it accepts reference to self, it can be called on [`dyn`] trait-objects
    fn get_dynamic(&self) -> &'static Type;

    /// Constructs this type if it is a struct
    ///
    /// Attempts to downcast passed arguments to type of fields.
    /// Multiple fields can be of same type, just make sure that order is preserved or you might get unexpected results
    ///
    /// If called on enum type, [`RuntimeConstructError::NotStruct`] will be returned
    ///
    /// **Note**: Arguments must be passed in same order as definition order of fields inside struct
    fn construct_struct(
        &self,
        args: Vec<Box<dyn Any>>,
    ) -> Result<Box<dyn Any>, RuntimeConstructError>;

    /// Constructs `Self` if it is enum
    ///
    /// Attempts to downcast passed arguments as type of fields of requested variant, if there are any.
    /// List of required arguments is target variant-dependent, as well as their order
    ///
    /// If variant is unit, no arguments will be required aside from `variant`
    ///
    /// **Note**: Arguments must be passed in same order as definition order of fields inside of particular variant
    fn construct_enum(
        &self,
        variant: &'static str, // some sort of safety gate, because in fully reflective usage one wouldn't be able to construct &'static variant name
        args: Vec<Box<dyn Any>>,
    ) -> Result<Box<dyn Any>, RuntimeConstructError>;

    /// Borrow immutably field inside this type
    ///
    /// Type must not be a unit and `id` must be valid in terms of this type (present)
    ///
    /// Returns [`Unsizeable`], which can further be downcast to "nameable" type
    fn field<'s>(&'s self, id: FieldId) -> Result<Unsizeable<'s>, FieldAccessError>;

    /// Borrow mutably field inside this type
    ///
    /// Same as [`TypeInfo::field`], except that returned "reference" is mutable
    fn field_mut<'s>(&'s mut self, id: FieldId) -> Result<UnsizeableMut<'s>, FieldAccessError>;
}

/// Static-type version of [`TypeInfoDynamic`]
pub trait TypeInfo: TypeInfoDynamic + Sized {
    #[allow(missing_docs)]
    const INFO: &'static Type;
}

/// Immutable reference holder, returned by [`TypeInfoDynamic::field`] method
///
/// Can be downcasted to underlying type if underlying type is "nameable"
pub struct Unsizeable<'a> {
    ptr: *const (),
    target_id: std::any::TypeId,
    _lt: std::marker::PhantomData<&'a ()>,
}

impl<'a> Unsizeable<'a> {
    #[doc(hidden)]
    pub fn new(ptr: *const (), target_id: std::any::TypeId) -> Self {
        Self {
            ptr,
            target_id,
            _lt: std::marker::PhantomData,
        }
    }

    /// Attempts to downcast field to immutable reference of particular type
    ///
    /// You need to be able to name this type in compile-time to succesfully downcast
    ///
    /// If `T` doesn't match actual type, [`Option::None`] will be returned
    pub fn downcast_ref<T>(&self) -> Option<&'a T>
    where
        T: 'static,
    {
        if std::any::TypeId::of::<T>() != self.target_id {
            return None;
        }

        unsafe {
            let target_ptr = self.ptr as *const T;
            target_ptr.as_ref()
        }
    }
}

/// Mutable reference holder, returned by [`TypeInfoDynamic::field_mut`] method
///
/// Can be downcasted to underlying type if underlying type is "nameable"

pub struct UnsizeableMut<'a> {
    ptr: *mut (),
    target_id: std::any::TypeId,
    _lt: std::marker::PhantomData<&'a ()>,
}
impl<'a> UnsizeableMut<'a> {
    #[doc(hidden)]
    pub fn new(ptr: *mut (), target_id: std::any::TypeId) -> Self {
        Self {
            ptr,
            target_id,
            _lt: std::marker::PhantomData,
        }
    }

    /// Attempts to downcast field to mutable reference of particular type
    ///
    /// You need to be able to name this type in compile-time
    /// If `T` doesn't match actual type, [`Option::None`] will be returned
    pub fn downcast_mut<T>(&self) -> Option<&'a mut T>
    where
        T: 'static,
    {
        if std::any::TypeId::of::<T>() != self.target_id {
            return None;
        }

        unsafe {
            let target_ptr = self.ptr as *mut T;
            target_ptr.as_mut()
        }
    }
}

use std::any::Any;

use paste::paste;
macro_rules! impl_primitive {
    ($name:ty ) => {
        paste! {
            #[allow(unused)]
            const  [<$name:upper _INFO>]: Type = Type {
              ident: std::stringify!($name),
              data: Data::Primitive,
              // size: std::mem::size_of::<$name>(),
              // alignment: std::mem::align_of::<$name>()
            };

            #[automatically_derived]
            impl TypeInfoDynamic for $name {
                fn get_dynamic(&self) ->  &'static Type {
                    &[<$name:upper _INFO>]

                }
                fn construct_struct(&self, _args: Vec<Box<dyn Any>>) -> Result<Box<dyn Any>, RuntimeConstructError> {
                     Err(RuntimeConstructError::Primitive)
                }

                fn construct_enum(
                    &self,
                    _variant: &'static str,
                    _args: Vec<Box<dyn Any>>,
                ) -> Result<Box<dyn Any>, RuntimeConstructError> {
                         Err(RuntimeConstructError::Primitive)

                }

                fn field<'s>(&'s self, id: FieldId) -> Result<Unsizeable<'s>, FieldAccessError> {
                    Err(FieldAccessError::Unit)
                }
                fn field_mut<'s>(&'s mut self, id: FieldId) -> Result<UnsizeableMut<'s>, FieldAccessError> {
                    Err(FieldAccessError::Unit)

                }

            }
            #[automatically_derived]
            impl TypeInfo for $name {
                const INFO: &'static Type = &[<$name:upper _INFO>];
            }
        }
    };
}

impl_primitive!(u8);
impl_primitive!(u16);
impl_primitive!(u32);
impl_primitive!(u64);
impl_primitive!(u128);

impl_primitive!(i8);
impl_primitive!(i16);
impl_primitive!(i32);
impl_primitive!(i64);
impl_primitive!(i128);

impl_primitive!(usize);
impl_primitive!(isize);

impl_primitive!(String);

impl_primitive!(f32);
impl_primitive!(f64);

mod __object_safety_check {
    use super::TypeInfoDynamic;

    fn __check_is_object_safe() -> Box<dyn TypeInfoDynamic> {
        Box::new(100u32)
    }
}
