// Copyright 2018-2020 Parity Technologies (UK) Ltd.
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

use crate::{error::ExtError, ir, ir::idents_lint};
use core::convert::TryFrom;
use proc_macro2::TokenStream as TokenStream2;
use std::collections::HashMap;
use syn::{spanned::Spanned as _, Result};
use core::slice::Iter as SliceIter;

/// An ink! chain extension.
#[derive(Debug, PartialEq, Eq)]
pub struct ChainExtension {
    item: syn::ItemTrait,
    methods: Vec<ChainExtensionMethod>,
}

impl ChainExtension {
    /// Returns the Rust attributes of the ink! chain extension.
    pub fn attrs(&self) -> Vec<syn::Attribute> {
        let (_, attrs) = ir::partition_attributes(self.item.attrs.iter().cloned())
            .expect("encountered unexpected invalid attributes for ink! chain extension");
        attrs
    }

    /// Returns the span of the ink! chain extension.
    pub fn span(&self) -> proc_macro2::Span {
        self.item.span()
    }

    /// Returns the identifier of the ink! chain extension.
    pub fn ident(&self) -> &proc_macro2::Ident {
        &self.item.ident
    }

    /// Returns a slice over all the chain extension methods.
    pub fn iter_methods(&self) -> SliceIter<ChainExtensionMethod> {
        self.methods.iter()
    }
}

/// An ink! chain extension method.
#[derive(Debug, PartialEq, Eq)]
pub struct ChainExtensionMethod {
    item: syn::TraitItemMethod,
    id: ExtensionId,
}

impl ChainExtensionMethod {
    /// Returns the Rust attributes of the ink! chain extension method.
    pub fn attrs(&self) -> Vec<syn::Attribute> {
        let (_, attrs) = ir::partition_attributes(self.item.attrs.iter().cloned())
            .expect("encountered unexpected invalid attributes for ink! chain extension method");
        attrs
    }

    /// Returns the span of the ink! chain extension method.
    pub fn span(&self) -> proc_macro2::Span {
        self.item.span()
    }

    /// Returns the identifier of the ink! chain extension method.
    pub fn ident(&self) -> &proc_macro2::Ident {
        &self.item.sig.ident
    }

    /// Returns the method signature of the ink! chain extension method.
    pub fn sig(&self) -> &syn::Signature {
        &self.item.sig
    }

    /// Returns the unique ID of the chain extension method.
    pub fn id(&self) -> ExtensionId {
        self.id
    }
}

/// The unique ID of an ink! chain extension method.
///
/// # Note
///
/// The ink! attribute `#[ink(extension = N: u32)]` for chain extension methods.
///
/// Has a `func_id` extension ID to identify the associated chain extension method.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtensionId {
    index: u32,
}

impl ExtensionId {
    /// Creates a new chain extension method ID from the given `u32`.
    pub fn from_u32(index: u32) -> Self {
        Self { index }
    }

    /// Returns the underlying raw `u32` index.
    pub fn into_u32(self) -> u32 {
        self.index
    }
}

impl TryFrom<syn::ItemTrait> for ChainExtension {
    type Error = syn::Error;

    fn try_from(
        item_trait: syn::ItemTrait,
    ) -> core::result::Result<Self, Self::Error> {
        idents_lint::ensure_no_ink_identifiers(&item_trait)?;
        Self::analyse_properties(&item_trait)?;
        let methods = Self::analyse_items(&item_trait)?;
        Ok(Self {
            item: item_trait,
            methods,
        })
    }
}

impl ChainExtension {
    /// Returns `Ok` if the trait matches all requirements for an ink! chain extension.
    pub fn new(attr: TokenStream2, input: TokenStream2) -> Result<Self> {
        if !attr.is_empty() {
            return Err(format_err_spanned!(
                attr,
                "unexpected attribute input for ink! chain extension"
            ))
        }
        let item_trait = syn::parse2::<syn::ItemTrait>(input)?;
        ChainExtension::try_from(item_trait)
    }

    /// Analyses the properties of the ink! chain extension.
    ///
    /// # Errors
    ///
    /// - If the input trait has been defined as `unsafe`.
    /// - If the input trait is an automatically implemented trait (`auto trait`).
    /// - If the input trait is generic over some set of types.
    /// - If the input trait's visibility is not public (`pub`).
    fn analyse_properties(item_trait: &syn::ItemTrait) -> Result<()> {
        if let Some(unsafety) = &item_trait.unsafety {
            return Err(format_err_spanned!(
                unsafety,
                "ink! chain extensions cannot be unsafe"
            ))
        }
        if let Some(auto) = &item_trait.auto_token {
            return Err(format_err_spanned!(
                auto,
                "ink! chain extensions cannot be automatically implemented traits"
            ))
        }
        if !item_trait.generics.params.is_empty() {
            return Err(format_err_spanned!(
                item_trait.generics.params,
                "ink! chain extensions must not be generic"
            ))
        }
        if !matches!(item_trait.vis, syn::Visibility::Public(_)) {
            return Err(format_err_spanned!(
                item_trait.vis,
                "ink! chain extensions must have public visibility"
            ))
        }
        if !item_trait.supertraits.is_empty() {
            return Err(format_err_spanned!(
                item_trait.supertraits,
                "ink! chain extensions with supertraits are not supported, yet"
            ))
        }
        Ok(())
    }

    /// Returns `Ok` if all trait items respects the requirements for an ink! chain extension.
    ///
    /// # Errors
    ///
    /// - If the trait contains an unsupported trait item such as
    ///     - associated constants (`const`)
    ///     - associated types (`type`)
    ///     - macros definitions or usages
    ///     - unknown token sequences (verbatims)
    ///     - methods with default implementations
    /// - If the trait contains methods which do not respect the ink! trait definition requirements:
    ///     - All trait methods must not have a `self` receiver.
    ///     - All trait methods must have an `#[ink(function = N: usize)]` attribute that is the ID that
    ///       corresponds with the function ID of the respective chain extension call.
    ///
    /// # Note
    ///
    /// The input Rust trait item is going to be replaced with a concrete chain extension type definition
    /// as a result of this proc. macro invocation.
    fn analyse_items(
        item_trait: &syn::ItemTrait,
    ) -> Result<Vec<ChainExtensionMethod>> {
        let mut methods = Vec::new();
        let mut seen_ids = HashMap::new();
        for trait_item in &item_trait.items {
            match trait_item {
                syn::TraitItem::Const(const_trait_item) => {
                    return Err(format_err_spanned!(
                        const_trait_item,
                        "associated constants in ink! chain extensions are not supported, yet"
                    ))
                }
                syn::TraitItem::Macro(macro_trait_item) => {
                    return Err(format_err_spanned!(
                        macro_trait_item,
                        "macros in ink! chain extensions are not supported"
                    ))
                }
                syn::TraitItem::Type(type_trait_item) => {
                    return Err(format_err_spanned!(
                    type_trait_item,
                    "associated types in ink! chain extensions are not supported, yet"
                ))
                }
                syn::TraitItem::Verbatim(verbatim) => {
                    return Err(format_err_spanned!(
                        verbatim,
                        "encountered unsupported item in ink! chain extensions"
                    ))
                }
                syn::TraitItem::Method(method_trait_item) => {
                    let method = Self::analyse_methods(method_trait_item)?;
                    let method_id = method.id();
                    if let Some(previous) = seen_ids.get(&method_id) {
                        return Err(format_err!(
                            method.span(),
                            "encountered duplicate extension identifiers for the same chain extension",
                        ).into_combine(format_err!(
                            *previous,
                            "previous duplicate extension identifier here",
                        )))
                    }
                    seen_ids.insert(method_id, method.span());
                    methods.push(method);
                }
                unknown => {
                    return Err(format_err_spanned!(
                        unknown,
                        "encountered unknown or unsupported item in ink! chain extensions"
                    ))
                }
            }
        }
        Ok(methods)
    }

    /// Analyses a chain extension method.
    ///
    /// # Errors
    ///
    /// - If the method is missing the `#[ink(function = N: usize)]` attribute.
    /// - If the method has a `self` receiver.
    /// - If the method declared as `unsafe`, `const` or `async`.
    /// - If the method has some explicit API.
    /// - If the method is variadic or has generic parameters.
    fn analyse_methods(
        method: &syn::TraitItemMethod,
    ) -> Result<ChainExtensionMethod> {
        if let Some(default_impl) = &method.default {
            return Err(format_err_spanned!(
                default_impl,
                "ink! chain extension methods with default implementations are not supported"
            ))
        }
        if let Some(constness) = &method.sig.constness {
            return Err(format_err_spanned!(
                constness,
                "const ink! chain extension methods are not supported"
            ))
        }
        if let Some(asyncness) = &method.sig.asyncness {
            return Err(format_err_spanned!(
                asyncness,
                "async ink! chain extension methods are not supported"
            ))
        }
        if let Some(unsafety) = &method.sig.unsafety {
            return Err(format_err_spanned!(
                unsafety,
                "unsafe ink! chain extension methods are not supported"
            ))
        }
        if let Some(abi) = &method.sig.abi {
            return Err(format_err_spanned!(
                abi,
                "ink! chain extension methods with non default ABI are not supported"
            ))
        }
        if let Some(variadic) = &method.sig.variadic {
            return Err(format_err_spanned!(
                variadic,
                "variadic ink! chain extension methods are not supported"
            ))
        }
        if !method.sig.generics.params.is_empty() {
            return Err(format_err_spanned!(
                method.sig.generics.params,
                "generic ink! chain extension methods are not supported"
            ))
        }
        match ir::first_ink_attribute(&method.attrs)?
                .map(|attr| attr.first().kind().clone()) {
            Some(ir::AttributeArg::Extension(extension)) => {
                return Self::analyse_chain_extension_method(method, extension)
            }
            Some(_unsupported) => {
                return Err(format_err_spanned!(
                    method,
                    "encountered unsupported ink! attribute for ink! chain extension method. expected #[ink(function = N: usize)] attribute"
                ))
            }
            None => {
                return Err(format_err_spanned!(
                    method,
                    "missing #[ink(function = N: usize)] flag on ink! chain extension method"
                ))
            }
        }
    }

    /// Analyses the properties of an ink! chain extension method.
    ///
    /// # Errors
    ///
    /// - If the chain extension method has a `self` receiver as first argument.
    fn analyse_chain_extension_method(
        item_method: &syn::TraitItemMethod,
        extension: ExtensionId,
    ) -> Result<ChainExtensionMethod> {
        ir::sanitize_attributes(
            item_method.span(),
            item_method.attrs.clone(),
            &ir::AttributeArgKind::Extension,
            |c| !matches!(c, ir::AttributeArg::Extension(_)),
        )?;
        if let Some(receiver) = item_method.sig.receiver() {
            return Err(format_err_spanned!(
                receiver,
                "ink! chain extension method must not have a `self` receiver",
            ))
        }
        let result = ChainExtensionMethod {
            id: extension,
            item: item_method.clone(),
        };
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checks if the token stream in `$chain_extension` results in the expected error message.
    macro_rules! assert_ink_chain_extension_eq_err {
        ( error: $err_str:literal, $($chain_extension:tt)* ) => {
            assert_eq!(
                <ChainExtension as TryFrom<syn::ItemTrait>>::try_from(syn::parse_quote! {
                    $( $chain_extension )*
                })
                .map_err(|err| err.to_string()),
                Err(
                    $err_str.to_string()
                )
            )
        };
    }

    #[test]
    fn unsafe_chain_extension_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "ink! chain extensions cannot be unsafe",
            pub unsafe trait MyChainExtension {}
        );
    }

    #[test]
    fn auto_chain_extension_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "ink! chain extensions cannot be automatically implemented traits",
            pub auto trait MyChainExtension {}
        );
    }

    #[test]
    fn non_pub_chain_extension_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "ink! chain extensions must have public visibility",
            trait MyChainExtension {}
        );
        assert_ink_chain_extension_eq_err!(
            error: "ink! chain extensions must have public visibility",
            pub(crate) trait MyChainExtension {}
        );
    }

    #[test]
    fn generic_chain_extension_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "ink! chain extensions must not be generic",
            pub trait MyChainExtension<T> {}
        );
    }

    #[test]
    fn chain_extension_with_supertraits_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "ink! chain extensions with supertraits are not supported, yet",
            pub trait MyChainExtension: SuperChainExtension {}
        );
    }

    #[test]
    fn chain_extension_containing_const_item_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "associated constants in ink! chain extensions are not supported, yet",
            pub trait MyChainExtension {
                const T: i32;
            }
        );
    }

    #[test]
    fn chain_extension_containing_associated_type_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "associated types in ink! chain extensions are not supported, yet",
            pub trait MyChainExtension {
                type Type;
            }
        );
    }

    #[test]
    fn chain_extension_containing_macro_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "macros in ink! chain extensions are not supported",
            pub trait MyChainExtension {
                my_macro_call!();
            }
        );
    }

    #[test]
    fn chain_extension_containing_non_flagged_method_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "missing #[ink(function = N: usize)] flag on ink! chain extension method",
            pub trait MyChainExtension {
                fn non_flagged_1(&self);
            }
        );
        assert_ink_chain_extension_eq_err!(
            error: "missing #[ink(function = N: usize)] flag on ink! chain extension method",
            pub trait MyChainExtension {
                fn non_flagged_2(&mut self);
            }
        );
        assert_ink_chain_extension_eq_err!(
            error: "missing #[ink(function = N: usize)] flag on ink! chain extension method",
            pub trait MyChainExtension {
                fn non_flagged_3() -> Self;
            }
        );
    }

    #[test]
    fn chain_extension_containing_default_implemented_methods_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "ink! chain extension methods with default implementations are not supported",
            pub trait MyChainExtension {
                #[ink(constructor)]
                fn default_implemented() -> Self {}
            }
        );
    }

    #[test]
    fn chain_extension_containing_const_methods_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "const ink! chain extension methods are not supported",
            pub trait MyChainExtension {
                #[ink(extension = 1)]
                const fn const_constructor() -> Self;
            }
        );
    }

    #[test]
    fn chain_extension_containing_async_methods_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "async ink! chain extension methods are not supported",
            pub trait MyChainExtension {
                #[ink(extension = 1)]
                async fn const_constructor() -> Self;
            }
        );
    }

    #[test]
    fn chain_extension_containing_unsafe_methods_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "unsafe ink! chain extension methods are not supported",
            pub trait MyChainExtension {
                #[ink(extension = 1)]
                unsafe fn const_constructor() -> Self;
            }
        );
    }

    #[test]
    fn chain_extension_containing_methods_using_explicit_abi_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "ink! chain extension methods with non default ABI are not supported",
            pub trait MyChainExtension {
                #[ink(extension = 1)]
                extern fn const_constructor() -> Self;
            }
        );
    }

    #[test]
    fn chain_extension_containing_variadic_methods_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "variadic ink! chain extension methods are not supported",
            pub trait MyChainExtension {
                #[ink(extension = 1)]
                fn const_constructor(...) -> Self;
            }
        );
    }

    #[test]
    fn chain_extension_containing_generic_methods_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "generic ink! chain extension methods are not supported",
            pub trait MyChainExtension {
                #[ink(extension = 1)]
                fn const_constructor<T>() -> Self;
            }
        );
    }

    #[test]
    fn chain_extension_containing_method_with_unsupported_ink_attribute_is_denied(
    ) {
        assert_ink_chain_extension_eq_err!(
            error: "\
                encountered unsupported ink! attribute for ink! chain extension method. \
                expected #[ink(function = N: usize)] attribute",
            pub trait MyChainExtension {
                #[ink(message)]
                fn unsupported_ink_attribute(&self);
            }
        );
        assert_ink_chain_extension_eq_err!(
            error: "unknown ink! attribute (path)",
            pub trait MyChainExtension {
                #[ink(unknown)]
                fn unknown_ink_attribute(&self);
            }
        );
    }

    #[test]
    fn chain_extension_containing_method_with_invalid_marker() {
        assert_ink_chain_extension_eq_err!(
            error: "could not parse `N` in `#[ink(extension = N)]` into a `u32` integer",
            pub trait MyChainExtension {
                #[ink(extension = -1)]
                fn has_self_receiver();
            }
        );
        let too_large = (u32::MAX as u64) + 1;
        assert_ink_chain_extension_eq_err!(
            error: "could not parse `N` in `#[ink(extension = N)]` into a `u32` integer",
            pub trait MyChainExtension {
                #[ink(extension = #too_large)]
                fn has_self_receiver();
            }
        );
        assert_ink_chain_extension_eq_err!(
            error: "expecteded `u32` integer type for `N` in #[ink(extension = N)]",
            pub trait MyChainExtension {
                #[ink(extension = "Hello, World!")]
                fn has_self_receiver();
            }
        );
        assert_ink_chain_extension_eq_err!(
            error: "encountered #[ink(extension)] that is missing its N parameter. Did you mean #[ink(extension = N: usize)] ?",
            pub trait MyChainExtension {
                #[ink(extension)]
                fn has_self_receiver();
            }
        );

        assert_ink_chain_extension_eq_err!(
            error: "encountered duplicate ink! attribute",
            pub trait MyChainExtension {
                #[ink(extension = 42)]
                #[ink(extension = 42)]
                fn duplicate_attributes() -> Self;
            }
        );
        assert_ink_chain_extension_eq_err!(
            error: "encountered ink! attribute arguments with equal kinds",
            pub trait MyChainExtension {
                #[ink(extension = 1)]
                #[ink(extension = 2)]
                fn duplicate_attributes() -> Self;
            }
        );
        assert_ink_chain_extension_eq_err!(
            error: "encountered conflicting ink! attribute argument",
            pub trait MyChainExtension {
                #[ink(extension = 1)]
                #[ink(message)]
                fn conflicting_attributes() -> Self;
            }
        );
    }

    #[test]
    fn chain_extension_containing_method_with_self_receiver_is_denied() {
        assert_ink_chain_extension_eq_err!(
            error: "ink! chain extension method must not have a `self` receiver",
            pub trait MyChainExtension {
                #[ink(extension = 1)]
                fn has_self_receiver(&self) -> Self;
            }
        );
        assert_ink_chain_extension_eq_err!(
            error: "ink! chain extension method must not have a `self` receiver",
            pub trait MyChainExtension {
                #[ink(extension = 1)]
                fn has_self_receiver(&mut self) -> Self;
            }
        );
        assert_ink_chain_extension_eq_err!(
            error: "ink! chain extension method must not have a `self` receiver",
            pub trait MyChainExtension {
                #[ink(extension = 1)]
                fn has_self_receiver(self) -> Self;
            }
        );
        assert_ink_chain_extension_eq_err!(
            error: "ink! chain extension method must not have a `self` receiver",
            pub trait MyChainExtension {
                #[ink(extension = 1)]
                fn has_self_receiver(self: &Self) -> Self;
            }
        );
        assert_ink_chain_extension_eq_err!(
            error: "ink! chain extension method must not have a `self` receiver",
            pub trait MyChainExtension {
                #[ink(extension = 1)]
                fn has_self_receiver(self: Self) -> Self;
            }
        );
    }

    #[test]
    fn chain_extension_with_overlapping_extension_ids() {
        assert_ink_chain_extension_eq_err!(
            error: "encountered duplicate extension identifiers for the same chain extension",
            pub trait MyChainExtension {
                #[ink(extension = 1)]
                fn same_id_1();
                #[ink(extension = 1)]
                fn same_id_2();
            }
        );
    }

    #[test]
    fn chain_extension_is_ok() {
        let chain_extension = <ChainExtension as TryFrom<syn::ItemTrait>>::try_from(syn::parse_quote! {
                pub trait MyChainExtension {
                    #[ink(extension = 1)]
                    fn extension_1();
                    #[ink(extension = 2)]
                    fn extension_2(input: i32);
                    #[ink(extension = 3)]
                    fn extension_3() -> i32;
                    #[ink(extension = 4)]
                    fn extension_4(input: i32) -> i32;
                    #[ink(extension = 5)]
                    fn extension_5(in1: i8, in2: i16, in3: i32, in4: i64) -> (u8, u16, u32, u64);
                }
            }).unwrap();
        assert_eq!(chain_extension.methods.len(), 5);
        for (actual, expected) in chain_extension
            .methods
            .iter()
            .map(|method| method.id())
            .zip(1..=5u32)
        {
            assert_eq!(actual.index, expected);
        }
        for (actual, expected) in chain_extension
            .methods
            .iter()
            .map(|method| method.ident().to_string())
            .zip(
                [
                    "extension_1",
                    "extension_2",
                    "extension_3",
                    "extension_4",
                    "extension_5",
                ]
                .iter()
                .map(ToString::to_string),
            )
        {
            assert_eq!(actual, expected);
        }
    }
}