// Copyright 2018-2021 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

/// Generates the tokens to compute the maximum of the numbers given via
/// their token streams at compilation time.
///
/// # Note
///
/// Since Rust currently does not allow conditionals in const contexts
/// we use the array indexing trick to compute the maximum element:
///
/// ```no_compile
/// max(a, b) = [a, b][(a < b) as usize]
/// ```
fn max_n(args: &[TokenStream2]) -> TokenStream2 {
    match args.split_first() {
        Some((head, rest)) => {
            let rest = max_n(rest);
            quote! {
                [#head, #rest][(#head < #rest) as ::core::primitive::usize]
            }
        }
        None => quote! { 0u64 },
    }
}

/// Generates the tokens for the `SpreadLayout` footprint of some type.
fn footprint(s: &synstructure::Structure) -> TokenStream2 {
    let variant_footprints = s
        .variants()
        .iter()
        .map(|variant| {
            variant
                .ast()
                .fields
                .iter()
                .map(|field| &field.ty)
                .map(|ty| quote! { <#ty as ::ink_storage::traits::SpreadLayout>::FOOTPRINT })
                .fold(quote! { 0u64 }, |lhs, rhs| {
                    quote! { (#lhs + #rhs) }
                })
        })
        .collect::<Vec<_>>();
    max_n(&variant_footprints[..])
}

/// Generates the tokens for the `SpreadLayout` `REQUIRES_DEEP_CLEAN_UP` constant for the given structure.
fn requires_deep_clean_up(s: &synstructure::Structure) -> TokenStream2 {
    s.variants()
        .iter()
        .map(|variant| {
            variant
            .ast()
            .fields
            .iter()
            .map(|field| &field.ty)
            .map(|ty| quote! { <#ty as ::ink_storage::traits::SpreadLayout>::REQUIRES_DEEP_CLEAN_UP })
            .fold(quote! { false }, |lhs, rhs| {
                quote! { (#lhs || #rhs) }
            })
        })
        .fold(quote! { false }, |lhs, rhs| {
            quote! { (#lhs || #rhs) }
        })
}

/// `SpreadLayout` derive implementation for `struct` types.
fn spread_layout_struct_derive(s: &synstructure::Structure) -> TokenStream2 {
    assert!(s.variants().len() == 1, "can only operate on structs");
    let footprint_body = footprint(s);
    let requires_deep_clean_up_body = requires_deep_clean_up(s);
    let variant: &synstructure::VariantInfo = &s.variants()[0];
    let pull_body = variant.construct(|field, _index| {
        let ty = &field.ty;
        quote! {
            <#ty as ::ink_storage::traits::SpreadLayout>::pull_spread(__key_ptr)
        }
    });
    let push_body = variant.each(|binding| {
        quote! {
            ::ink_storage::traits::SpreadLayout::push_spread(#binding, __key_ptr);
        }
    });
    let clear_body = s.each(|field| {
        quote! {
            ::ink_storage::traits::SpreadLayout::clear_spread(#field, __key_ptr);
        }
    });
    s.gen_impl(quote! {
        gen impl ::ink_storage::traits::SpreadLayout for @Self {
            #[allow(unused_comparisons)]
            const FOOTPRINT: ::core::primitive::u64 = #footprint_body;
            const REQUIRES_DEEP_CLEAN_UP: ::core::primitive::bool = #requires_deep_clean_up_body;

            fn pull_spread(__key_ptr: &mut ::ink_storage::traits::KeyPtr) -> Self {
                #pull_body
            }
            fn push_spread(&self, __key_ptr: &mut ::ink_storage::traits::KeyPtr) {
                match self { #push_body }
            }
            fn clear_spread(&self, __key_ptr: &mut ::ink_storage::traits::KeyPtr) {
                match self { #clear_body }
            }
        }
    })
}

/// `SpreadLayout` derive implementation for `enum` types.
fn spread_layout_enum_derive(s: &synstructure::Structure) -> TokenStream2 {
    assert!(s.variants().len() >= 2, "can only operate on enums");
    let footprint_body = footprint(s);
    let requires_deep_clean_up_body = requires_deep_clean_up(s);
    let pull_body = s
        .variants()
        .iter()
        .map(|variant| {
            variant.construct(|field, _index| {
                let ty = &field.ty;
                quote! {
                    <#ty as ::ink_storage::traits::SpreadLayout>::pull_spread(__key_ptr)
                }
            })
        })
        .enumerate()
        .fold(quote! {}, |acc, (index, variant)| {
            let index = index as u8;
            quote! {
                #acc
                #index => #variant,
            }
        });

    let push_body = s.variants().iter().enumerate().map(|(index, variant)| {
        let pat = variant.pat();
        let index = index as u8;
        let fields = variant.bindings().iter().map(|field| {
            quote! {
                ::ink_storage::traits::SpreadLayout::push_spread(#field, __key_ptr);
            }
        });
        quote! {
            #pat => {
                { <::core::primitive::u8 as ::ink_storage::traits::SpreadLayout>::push_spread(&#index, __key_ptr); }
                #(
                    { #fields }
                )*
            }
        }
    });
    let clear_body = s.each(|field| {
        quote! {
            ::ink_storage::traits::SpreadLayout::clear_spread(#field, __key_ptr);
        }
    });
    s.gen_impl(quote! {
        gen impl ::ink_storage::traits::SpreadLayout for @Self {
            #[allow(unused_comparisons)]
            const FOOTPRINT: ::core::primitive::u64 = 1 + #footprint_body;

            const REQUIRES_DEEP_CLEAN_UP: ::core::primitive::bool = #requires_deep_clean_up_body;

            fn pull_spread(__key_ptr: &mut ::ink_storage::traits::KeyPtr) -> Self {
                match <::core::primitive::u8 as ::ink_storage::traits::SpreadLayout>::pull_spread(__key_ptr) {
                    #pull_body
                    _ => unreachable!("encountered invalid enum discriminant"),
                }
            }
            fn push_spread(&self, __key_ptr: &mut ::ink_storage::traits::KeyPtr) {
                match self {
                    #(
                        #push_body
                    )*
                }
            }
            fn clear_spread(&self, __key_ptr: &mut ::ink_storage::traits::KeyPtr) {
                match self {
                    #clear_body
                }
            }
        }
    })
}

/// Derives `ink_storage`'s `SpreadLayout` trait for the given `struct` or `enum`.
pub fn spread_layout_derive(mut s: synstructure::Structure) -> TokenStream2 {
    s.bind_with(|_| synstructure::BindStyle::Move)
        .add_bounds(synstructure::AddBounds::Generics)
        .underscore_const(true);
    match s.ast().data {
        syn::Data::Struct(_) => spread_layout_struct_derive(&s),
        syn::Data::Enum(_) => spread_layout_enum_derive(&s),
        _ => {
            panic!(
                "cannot derive `SpreadLayout` or `PackedLayout` for Rust `union` items"
            )
        }
    }
}
