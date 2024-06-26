use proc_macro::TokenStream;
use quote::{format_ident, quote};

#[derive(std::hash::Hash, Clone, PartialEq, Eq)]
enum FieldId {
    Named(syn::Ident),
    Index(syn::LitInt),
}

impl FieldId {
    pub fn as_named(&self) -> &syn::Ident {
        match &self {
            Self::Named(name) => name,
            _ => panic!("Not a named ID"),
        }
    }
    pub fn as_indexed(&self) -> &syn::LitInt {
        match &self {
            Self::Index(name) => name,
            _ => panic!("Not a indexed ID"),
        }
    }
}

struct Field {
    id: FieldId,
    ty_ident: syn::Ident,
}

enum Fields {
    Named(Vec<Field>),
    Indexed(Vec<Field>),
    Unit,
}

struct FieldsIter<'a> {
    iter: std::option::Option<std::slice::Iter<'a, Field>>,
}

impl<'a> Iterator for FieldsIter<'a> {
    type Item = &'a Field;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter {
            Some(ref mut iter) => iter.next(),
            None => None,
        }
    }
}

impl<'a> DoubleEndedIterator for FieldsIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.iter {
            Some(ref mut iter) => iter.next_back(),
            None => None,
        }
    }
}

impl<'a> ExactSizeIterator for FieldsIter<'a> {
    fn len(&self) -> usize {
        match &self.iter {
            Some(iter) => iter.len(),
            None => 0usize,
        }
    }
}

impl Fields {
    fn iter<'a>(&'a self) -> FieldsIter<'a> {
        match self {
            Fields::Named(named) => FieldsIter {
                iter: Some(named.iter()),
            },
            Fields::Indexed(indexed) => FieldsIter {
                iter: Some(indexed.iter()),
            },
            Fields::Unit => FieldsIter { iter: None },
        }
    }
}

struct Variant {
    name: syn::Ident,
    discriminator: syn::LitInt,
    fields: Fields,
}

struct Variants {
    variants: Vec<Variant>,
}

enum Data {
    Struct(Fields),
    Enum(Variants),
}

fn create_meta_fields<'a, I: Iterator<Item = &'a syn::Field>>(fields: I) -> Fields {
    let mut new_fields = Vec::new();
    for (index, field) in fields.enumerate() {
        let field_id = match field.ident.as_ref() {
            Some(str_id) => FieldId::Named(str_id.clone()),
            None => FieldId::Index(syn::LitInt::new(
                &index.to_string(),
                proc_macro2::Span::call_site(),
            )),
        };

        let syn::Type::Path(ref type_ident) = field.ty else {
            panic!("Unsupported field type used in ",)
        };

        let Some(type_ident) = type_ident.path.get_ident() else {
            todo!();
        };

        new_fields.push(Field {
            id: field_id,
            ty_ident: type_ident.clone(),
        });
    }

    match new_fields.first() {
        Some(field) => match field.id {
            FieldId::Named(_) => Fields::Named(new_fields),
            FieldId::Index(_) => Fields::Indexed(new_fields),
        },
        None => Fields::Unit,
    }
}

fn create_meta_variants<'a, I: Iterator<Item = &'a syn::Variant>>(variants: I) -> Variants {
    let mut new_variants = Vec::new();

    for (index, variant) in variants.enumerate() {
        let variant_name = variant.ident.clone();
        let fields = create_meta_fields(variant.fields.iter());

        new_variants.push(Variant {
            discriminator: syn::LitInt::new(&index.to_string(), variant_name.span()),
            name: variant_name,
            fields,
        })
    }

    Variants {
        variants: new_variants,
    }
}

struct MetaType {
    ident: syn::Ident,
    info_ident: syn::Ident,
    data: Data,
}

impl MetaType {
    pub fn new(input: &syn::DeriveInput) -> Self {
        let ident = input.ident.clone();

        let meta_data = match &input.data {
            syn::Data::Struct(syn::DataStruct { fields, .. }) => {
                let fields_iter = match fields {
                    syn::Fields::Named(named) => create_meta_fields(named.named.iter()),
                    syn::Fields::Unnamed(unnamed) => create_meta_fields(unnamed.unnamed.iter()),
                    syn::Fields::Unit => Fields::Unit,
                };
                Data::Struct(fields_iter)
            }
            syn::Data::Enum(enum_data) => {
                Data::Enum(create_meta_variants(enum_data.variants.iter()))
            }
            syn::Data::Union(_) => panic!("Unions are not supported"),
        };

        let info_ident = format_ident!("{}_TYPE_INFO", ident.to_string().to_ascii_uppercase());

        Self {
            ident,
            data: meta_data,
            info_ident,
        }
    }
}

mod gen {

    use quote::format_ident;
    use quote::quote;
    use quote::quote_spanned;
    use quote::ToTokens;

    use super::FieldId;
    use crate::Variants;

    use super::Fields;
    use super::MetaType;

    use std::collections::HashMap;

    fn collect_fields(fields: &Fields) -> proc_macro2::TokenStream {
        match fields {
            Fields::Named(named) => {
                let mut fields_definition = Vec::new();
                for field in named.iter() {
                    let crate::FieldId::Named(ref ident) = field.id else {
                        unreachable!()
                    };
                    let name = ident.to_string();
                    let type_ident = field.ty_ident.clone();

                    fields_definition.push(quote! {
                        reflectix_core::Field {
                            id: reflectix_core::FieldId::Named(#name),
                            ty: <#type_ident as reflectix_core::TypeInfo>::INFO,
                        }
                    });
                }

                quote! {
                    reflectix_core::Fields::Named(&[#(#fields_definition),*])

                }
            }
            Fields::Indexed(unnamed) => {
                let mut fields_definition = Vec::new();
                for field in unnamed.iter() {
                    let FieldId::Index(ref ident) = field.id else {
                        unreachable!()
                    };
                    let type_ident = field.ty_ident.clone();

                    fields_definition.push(quote! {
                        reflectix_core::Field {
                            id: reflectix_core::FieldId::Indexed(#ident),
                            ty: <#type_ident as reflectix_core::TypeInfo>::INFO,
                        }
                    });
                }

                quote! {
                    reflectix_core::Fields::Indexed(&[#(#fields_definition),*])

                }
            }
            Fields::Unit => quote! {reflectix_core::Fields::Unit},
        }
    }

    fn collect_variants(variants: &Variants) -> proc_macro2::TokenStream {
        let mut variants_list = Vec::new();

        for (discriminator, variant) in variants.variants.iter().enumerate() {
            let variant_name = variant.name.to_string();
            let fields_stmt = collect_fields(&variant.fields);

            variants_list.push(quote! {
                reflectix_core::Variant {
                    ident: #variant_name,
                    fields: #fields_stmt,
                    discriminator: #discriminator
                }
            });
        }

        quote! {
            reflectix_core::Variants{variants: &[#(#variants_list),*]}
        }
    }

    pub fn create_const_definition(meta: &MetaType) -> proc_macro2::TokenStream {
        let data_definition = match &meta.data {
            crate::Data::Struct(fields) => {
                let fields = collect_fields(&fields);
                quote! {
                    reflectix_core::Data::Struct(#fields)
                }
            }
            crate::Data::Enum(variants) => {
                let variants = collect_variants(&variants);
                quote! {
                    reflectix_core::Data::Enum(#variants)
                }
            }
        };

        let const_ident = &meta.info_ident;
        let ty_ident = meta.ident.to_string();

        let const_type_info_stmt = quote_spanned! {proc_macro2::Span::mixed_site()=>
          const #const_ident: reflectix_core::Type = reflectix_core::Type {
              ident: #ty_ident,
              data: #data_definition,
          };
        };
        const_type_info_stmt
    }

    fn field_id_to_tokens(id: &FieldId) -> proc_macro2::TokenStream {
        match id {
            FieldId::Named(ident) => {
                let as_str = ident.to_string();
                quote! {
                    reflectix_core::FieldId::Named(#as_str)
                }
            }
            FieldId::Index(index) => quote! {
                reflectix_core::FieldId::Index(#index)
            },
        }
    }

    fn create_dyn_field_access_match(
        self_ident: Option<&syn::Ident>,
        input_id_ident: &syn::Ident,
        fields: &Fields,
        is_mut_ref: bool,
        is_accessing_tuple_enum_variant: bool,
    ) -> proc_macro2::TokenStream {
        let ref_producer = |ident: &syn::Ident| {
            let field_ident = match self_ident {
                Some(self_ident) => quote! {#self_ident.#ident},
                None => ident.to_token_stream(),
            };

            // match is_mut_ref {
            //     true => {
            //         quote! {&mut #field_ident}
            //     }
            //     false => {
            //         quote! {& #field_ident}
            //     }
            // }
            field_ident
        };

        let mut patterns = Vec::new();
        let mut arms = Vec::new();

        for field in fields.iter() {
            let field_id_as_tokens = field_id_to_tokens(&field.id);

            let attr_access_name = match &field.id {
                FieldId::Named(ident) => ident.clone(),
                FieldId::Index(index) => {
                    // tuple-emum field names are prefixed with _ to make them valid idents
                    let prefix = if is_accessing_tuple_enum_variant {
                        "_"
                    } else {
                        ""
                    }
                    .to_string();
                    syn::Ident::new(&(prefix + &index.clone().to_string()), index.span())
                }
            };

            let pattern = quote! {
                #field_id_as_tokens
            };
            let mut field_ref = ref_producer(&attr_access_name);

            let field_ty_ident = &field.ty_ident;

            // need to extend lifetime
            //
            // SAFETY: this field is part of enum and we are accessing it in correct variant
            // therefore, it is safe to extend this reference live to &self lifetime
            // as this field will live for as long, as enum is living

            if !is_accessing_tuple_enum_variant {
                let ref_type = match is_mut_ref {
                    true => quote! {&mut },
                    false => quote!(&),
                };
                field_ref = quote! {#ref_type #field_ref};
            }

            let caster_block = match is_mut_ref {
                true => quote! {
                    let field_ref = (#field_ref as *mut #field_ty_ident) as *mut ();
                    let target_id = std::any::TypeId::of::<#field_ty_ident>();

                    return Ok(reflectix_core::UnsizeableMut::new(field_ref, target_id));
                },
                false => quote! {
                    let field_ref = (#field_ref as *const #field_ty_ident) as *const ();
                    let target_id = std::any::TypeId::of::<#field_ty_ident>();

                    return Ok(reflectix_core::Unsizeable::new(field_ref, target_id));
                },
            };

            patterns.push(pattern);
            arms.push(caster_block);
        }

        // can avoid handling `Fields::Unit` variant because iterator will be empty
        // and wildcard arm will be triggered
        quote! {
            match #input_id_ident {
                #(#patterns => {#arms})*
                _ => {
                    return Err(reflectix_core::FieldAccessError::NotFound);
                }
            }
        }
    }

    fn create_dyn_variant_access_match(
        self_ident: &syn::Ident,
        input_id_ident: &syn::Ident,
        variants: &Variants,
        is_mut_ref: bool,
    ) -> proc_macro2::TokenStream {
        let mut patterns = Vec::new();
        let mut arms = Vec::new();

        let inplace_ref_type = match is_mut_ref {
            true => quote! {ref mut },
            false => quote! {ref},
        };

        for variant in variants.variants.iter() {
            let variant_name = &variant.name;

            let pattern = match &variant.fields {
                Fields::Named(named) => {
                    let all_fields_idents = named
                        .iter()
                        .map(|x| x.id.as_named().clone())
                        .collect::<Vec<_>>();

                    quote! {
                        Self::#variant_name{#(#inplace_ref_type #all_fields_idents),*}
                    }
                }
                Fields::Indexed(indexed) => {
                    let all_fields_idents = indexed
                        .iter()
                        // prefixing enum fields indexes with underscore to make them valid idents
                        .map(|x| {
                            syn::Ident::new(
                                &format!("_{}", x.id.as_indexed().to_string()),
                                x.ty_ident.span(),
                            )
                        })
                        .collect::<Vec<_>>();

                    match is_mut_ref {
                        true => quote! {
                            Self::#variant_name(#(ref mut #all_fields_idents),*)
                        },
                        false => quote! {
                            Self::#variant_name(#(ref #all_fields_idents),*)
                        },
                    }
                }
                Fields::Unit => quote! {Self::#variant_name},
            };

            let arm = match &variant.fields {
                iterable_fields @ (Fields::Named(_) | Fields::Indexed(_)) => {
                    create_dyn_field_access_match(
                        None,
                        &input_id_ident,
                        iterable_fields,
                        is_mut_ref,
                        true,
                    )
                }
                Fields::Unit => quote! {
                    return Err(reflectix_core::FieldAccessError::Unit);
                },
            };

            patterns.push(pattern);
            arms.push(arm);
        }

        quote! {
            match #self_ident {
                #(#patterns => {#arms})*
                _ => {
                    return Err(reflectix_core::FieldAccessError::UnmatchingDiscriminator);
                }
            }
        }
    }

    // fn field<'s>(&'s self, id: FieldId) -> Result<&'s dyn Any, FieldAccessError>
    pub fn create_get_dyn_field_method_body(
        meta: &MetaType,
        is_mut: bool,
    ) -> proc_macro2::TokenStream {
        let id_ident = syn::Ident::new("id", proc_macro2::Span::call_site());
        let self_ident = syn::Ident::new("self", proc_macro2::Span::call_site());

        let body = match meta.data {
            crate::Data::Struct(ref fields) => {
                create_dyn_field_access_match(Some(&self_ident), &id_ident, fields, is_mut, false)
            }
            crate::Data::Enum(ref variants) => {
                create_dyn_variant_access_match(&self_ident, &id_ident, variants, is_mut)
            }
        };

        body
    }

    /*
    Generates match statement, which compares passed FieldId to "available" FieldId's

    Downcasts field initializers from Box<dyn _> to concrete type of which field
    Then, it takes out value directly from Box using some unsafe code

    Finally, if every type matches those of fields (note: that fields of same type are supported, they just can't be named)
    it constructs implementing type and returns it boxed with erased type

    It returns erased type because if we already can refer to concrete type, then why to use reflective constructor in first place?
    */
    fn create_dyn_fields_ctor_body(
        type_ident: &proc_macro2::TokenStream,
        args_ident: &syn::Ident,
        fields: &Fields,
    ) -> proc_macro2::TokenStream {
        match fields {
            fields @ (Fields::Named(..) | Fields::Indexed(..)) => {
                let mut field_downcast_stmts = Vec::new();
                let mut field_identifiers = HashMap::new();
                for (index, field) in fields.iter().enumerate().rev() {
                    let curr_box_ident = format_ident!("boxed_{}", { index });

                    let current_type = field.ty_ident.clone();
                    let current_type_str = format!("{}", current_type);

                    let downcast_stmt = quote! {
                        let #curr_box_ident = #args_ident.pop().ok_or(reflectix_core::RuntimeConstructError::NotEnoughArgs)?;
                        let #curr_box_ident = #curr_box_ident.downcast::<#current_type>().map_err(|_| reflectix_core::RuntimeConstructError::UnexpectedType{index: #index, expected: #current_type_str})?;

                        let #curr_box_ident = unsafe {
                            let as_raw = Box::into_raw(#curr_box_ident);
                            let new_item = std::ptr::read(as_raw);
                            std::mem::drop(Box::from_raw(as_raw));
                            new_item
                        };
                    };

                    field_downcast_stmts.push(downcast_stmt);
                    field_identifiers.insert(field.id.clone(), curr_box_ident);
                }

                let is_indexed = fields
                    .iter()
                    .all(|x| matches!(x.id, crate::FieldId::Index(_)));

                match is_indexed {
                    true => {
                        let keys = field_identifiers
                            .keys()
                            .map(FieldId::as_indexed)
                            .collect::<Vec<_>>();
                        quote! {
                            #(#field_downcast_stmts)*

                            return Ok(Box::new(#type_ident(#(#keys),*)));
                        }
                    }
                    false => {
                        let mut keys = Vec::new();
                        let mut values = Vec::new();

                        for (key, value) in field_identifiers.drain() {
                            values.push(value);

                            let crate::FieldId::Named(key) = key else {
                                unreachable!()
                            };
                            let key =
                                syn::Ident::new(&key.to_string(), proc_macro2::Span::call_site());
                            keys.push(key);
                        }

                        quote! {
                            #(#field_downcast_stmts)*

                            return Ok(Box::new(#type_ident{#(#keys: #values),*}));
                        }
                    }
                }
            }
            Fields::Unit => quote! {
                return Ok(Box::new(#type_ident));
            },
        }
    }

    // fn construct_enum(
    //         &self,
    //         variant: &'static str,
    //         args: Vec<Box<dyn Any>>,
    //     ) -> Result<Box<dyn Any>, RuntimeConstructError>;
    pub fn create_dyn_enum_ctor(meta: &MetaType) -> proc_macro2::TokenStream {
        let args_ident = syn::Ident::new("args", proc_macro2::Span::call_site());
        let requested_variant_ident = syn::Ident::new("variant", proc_macro2::Span::call_site());
        let self_ty_ident = syn::Ident::new("Self", proc_macro2::Span::call_site());

        let body = match &meta.data {
            crate::Data::Struct(_) => quote! {
                return Err(reflectix_core::RuntimeConstructError::NotEnum);
            },
            crate::Data::Enum(variants) => {
                let mut patterns = Vec::new();
                let mut bodies = Vec::new();
                for variant in variants.variants.iter() {
                    let variant_name_ident = &variant.name;
                    let ctor_body = match &variant.fields {
                        fields @ (Fields::Named(_) | Fields::Indexed(_)) => {
                            let variant_ty_ident = quote! {#self_ty_ident::#variant_name_ident};
                            create_dyn_fields_ctor_body(&variant_ty_ident, &args_ident, &fields)
                        }
                        Fields::Unit => quote! {
                            return Ok(Box::new(#self_ty_ident::#variant_name_ident));
                        },
                    };
                    let variant_name_str = variant.name.to_string();
                    let pattern = quote! {
                         #variant_name_str
                    };
                    patterns.push(pattern);
                    bodies.push(ctor_body);
                }

                let match_stmt = quote! {
                    match #requested_variant_ident {
                        #(#patterns => {
                            #bodies
                        })*

                        _ => {
                            return Err(reflectix_core::RuntimeConstructError::InvalidVariant);
                        }
                    }
                };

                match_stmt
            }
        };

        quote! {
            fn construct_enum(
                &self,
                #requested_variant_ident: &'static str,
                mut #args_ident: Vec<Box<dyn std::any::Any>>,
            ) -> Result<Box<dyn std::any::Any>, reflectix_core::RuntimeConstructError> {
                #body
            }

        }
    }

    // fn construct_struct(
    //     &self,
    //     args: Vec<Box<dyn Any>>,
    // ) -> Result<Box<dyn Any>, RuntimeConstructError>;
    pub fn create_dyn_struct_ctor(meta: &MetaType) -> proc_macro2::TokenStream {
        let args_ident = syn::Ident::new("args", proc_macro2::Span::call_site());
        let self_ty_ident = syn::Ident::new("Self", proc_macro2::Span::call_site());

        let body = match &meta.data {
            crate::Data::Struct(fields) => {
                create_dyn_fields_ctor_body(&self_ty_ident.to_token_stream(), &args_ident, &fields)
            }
            crate::Data::Enum(_) => {
                quote! {
                    return Err(reflectix_core::RuntimeConstructError::NotStruct);
                }
            }
        };

        quote! {
            fn construct_struct(
                &self,
                mut #args_ident: Vec<Box<dyn std::any::Any>>,
            ) -> Result<Box<dyn std::any::Any>, reflectix_core::RuntimeConstructError> {
                #body
            }

        }
    }
}

#[proc_macro_derive(TypeInfo)]
pub fn type_info_derive(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    if !ast.generics.params.is_empty() {
        panic!("Type info for generic struct is currently not supported");
    }

    let meta = MetaType::new(&ast);

    let const_definition = gen::create_const_definition(&meta);

    let const_def_ident = meta.info_ident.clone();
    let ty_ident = meta.ident.clone();

    let struct_ctor = gen::create_dyn_struct_ctor(&meta);
    let enum_ctor = gen::create_dyn_enum_ctor(&meta);

    let mut_field_access_body = gen::create_get_dyn_field_method_body(&meta, true);
    let field_access_body = gen::create_get_dyn_field_method_body(&meta, false);
    let tokens = quote! {
        #const_definition

        impl reflectix_core::TypeInfoDynamic for #ty_ident {
             fn get_dynamic(&self) -> &'static reflectix_core::Type {
                 &#const_def_ident
             }

             #struct_ctor
             #enum_ctor

            fn field<'s>(&'s self, id: reflectix_core::FieldId) -> Result<reflectix_core::Unsizeable<'s>, reflectix_core::FieldAccessError> {
                #field_access_body
            }
            fn field_mut<'s>(&'s mut self, id: reflectix_core::FieldId) -> Result<reflectix_core::UnsizeableMut<'s>, reflectix_core::FieldAccessError> {
                #mut_field_access_body
            }

        }

        impl reflectix_core::TypeInfo for #ty_ident {
            const INFO: &'static reflectix_core::Type = &#const_def_ident;
        }

    }
    .into();
    tokens
}
