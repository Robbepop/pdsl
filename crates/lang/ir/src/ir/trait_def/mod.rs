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

mod iter;
mod trait_item;

#[cfg(test)]
mod tests;

use self::iter::IterInkTraitItemsRaw;
pub use self::{
    iter::IterInkTraitItems,
    trait_item::{
        InkTraitConstructor,
        InkTraitItem,
        InkTraitMessage,
        InputsIter,
    },
};
use super::attrs::InkAttribute;
use crate::{
    ir,
    ir::idents_lint,
    Selector,
};
use core::convert::TryFrom;
use ir::TraitPrefix;
use proc_macro2::{
    Ident,
    Span,
    TokenStream as TokenStream2,
};
use std::collections::HashMap;
use syn::{
    spanned::Spanned as _,
    Result,
};

/// A checked ink! trait definition.
#[derive(Debug, PartialEq, Eq)]
pub struct InkTrait {
    item: syn::ItemTrait,
    message_selectors: HashMap<syn::Ident, Selector>,
    constructor_selectors: HashMap<syn::Ident, Selector>,
}

impl TryFrom<syn::ItemTrait> for InkTrait {
    type Error = syn::Error;

    fn try_from(item_trait: syn::ItemTrait) -> core::result::Result<Self, Self::Error> {
        idents_lint::ensure_no_ink_identifiers(&item_trait)?;
        Self::analyse_properties(&item_trait)?;
        Self::analyse_items(&item_trait)?;
        let mut message_selectors = <HashMap<syn::Ident, Selector>>::new();
        let mut constructor_selectors = <HashMap<syn::Ident, Selector>>::new();
        Self::extract_selectors(
            &item_trait,
            &mut message_selectors,
            &mut constructor_selectors,
        )?;
        Ok(Self {
            item: item_trait,
            message_selectors,
            constructor_selectors,
        })
    }
}

impl InkTrait {
    /// Returns the hash to verify that the trait definition has been checked.
    pub fn compute_verify_hash<C, M>(
        trait_name: &Ident,
        constructors: C,
        messages: M,
    ) -> [u8; 32]
    where
        // Name and number of inputs.
        C: Iterator<Item = (Ident, usize)>,
        // Name, number of inputs and true if message may mutate storage.
        M: Iterator<Item = (Ident, usize, bool)>,
    {
        let mut constructors = constructors
            .map(|(name, len_inputs)| {
                [name.to_string(), len_inputs.to_string()].join(":")
            })
            .collect::<Vec<_>>();
        let mut messages = messages
            .map(|(name, len_inputs, mutability)| {
                let mutability = match mutability {
                    true => "w",
                    false => "r",
                };
                [
                    name.to_string(),
                    len_inputs.to_string(),
                    mutability.to_string(),
                ]
                .join(":")
            })
            .collect::<Vec<_>>();
        constructors.sort_unstable();
        messages.sort_unstable();
        let joined_constructors = constructors.join(",");
        let joined_messages = messages.join(",");
        let mut buffer = vec!["__ink_trait".to_string(), trait_name.to_string()];
        if !joined_constructors.is_empty() {
            buffer.push(joined_constructors);
        }
        if !joined_messages.is_empty() {
            buffer.push(joined_messages);
        }
        let buffer = buffer.join("::").into_bytes();
        use blake2::digest::generic_array::sequence::Split as _;
        let (head_32, _rest) =
            <blake2::Blake2b as blake2::Digest>::digest(&buffer).split();
        head_32.into()
    }

    /// Returns the hash to verify that the trait definition has been checked.
    pub fn verify_hash(&self) -> [u8; 32] {
        let trait_name = self.ident();
        Self::compute_verify_hash(
            trait_name,
            self.iter_items()
                .map(|(item, _)| item)
                .flat_map(InkTraitItem::filter_map_constructor)
                .map(|constructor| {
                    let name = constructor.sig().ident.clone();
                    let len_inputs = constructor.sig().inputs.len();
                    (name, len_inputs)
                }),
            self.iter_items()
                .map(|(item, _)| item)
                .flat_map(InkTraitItem::filter_map_message)
                .map(|message| {
                    let name = message.sig().ident.clone();
                    let len_inputs = message.sig().inputs.len();
                    let mutability = message.mutates();
                    (name, len_inputs, mutability)
                }),
        )
    }

    /// Returns the unique trait identifier.
    pub fn unique_id(&self) -> u32 {
        let [h1, h2, h3, h4, ..] = self.verify_hash();
        u32::from_be_bytes([h1, h2, h3, h4])
    }
}

impl InkTrait {
    /// Returns `Ok` if the trait matches all requirements for an ink! trait definition.
    pub fn new(attr: TokenStream2, input: TokenStream2) -> Result<Self> {
        if !attr.is_empty() {
            return Err(format_err_spanned!(
                attr,
                "unexpected attribute input for ink! trait definition"
            ))
        }
        let item_trait = syn::parse2::<syn::ItemTrait>(input)?;
        InkTrait::try_from(item_trait)
    }

    /// Returns span of the ink! trait definition.
    pub fn span(&self) -> Span {
        self.item.span()
    }

    /// Returns the attributes of the ink! trait definition.
    pub fn attrs(&self) -> &[syn::Attribute] {
        &self.item.attrs
    }

    /// Returns the identifier of the ink! trait definition.
    pub fn ident(&self) -> &Ident {
        &self.item.ident
    }

    /// Returns an iterator yielding the ink! specific items of the ink! trait definition.
    pub fn iter_items(&self) -> IterInkTraitItems {
        IterInkTraitItems::new(self)
    }

    /// Analyses the properties of the ink! trait definition.
    ///
    /// # Errors
    ///
    /// - If the trait has been defined as `unsafe`.
    /// - If the trait is an automatically implemented trait (`auto trait`).
    /// - If the trait is generic over some set of types.
    /// - If the trait's visibility is not public (`pub`).
    fn analyse_properties(item_trait: &syn::ItemTrait) -> Result<()> {
        if let Some(unsafety) = &item_trait.unsafety {
            return Err(format_err_spanned!(
                unsafety,
                "ink! trait definitions cannot be unsafe"
            ))
        }
        if let Some(auto) = &item_trait.auto_token {
            return Err(format_err_spanned!(
                auto,
                "ink! trait definitions cannot be automatically implemented traits"
            ))
        }
        if !item_trait.generics.params.is_empty() {
            return Err(format_err_spanned!(
                item_trait.generics.params,
                "ink! trait definitions must not be generic"
            ))
        }
        if !matches!(item_trait.vis, syn::Visibility::Public(_)) {
            return Err(format_err_spanned!(
                item_trait.vis,
                "ink! trait definitions must have public visibility"
            ))
        }
        if !item_trait.supertraits.is_empty() {
            return Err(format_err_spanned!(
                item_trait.supertraits,
                "ink! trait definitions with supertraits are not supported, yet"
            ))
        }
        Ok(())
    }

    /// Returns `Ok` if all trait items respects the requirements for an ink! trait definition.
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
    ///     - All trait methods need to be declared as either `#[ink(message)]` or `#[ink(constructor)]`
    ///       and need to respect their respective rules.
    ///
    /// # Note
    ///
    /// Associated types and constants might be allowed in the future.
    fn analyse_items(item_trait: &syn::ItemTrait) -> Result<()> {
        for trait_item in &item_trait.items {
            match trait_item {
                syn::TraitItem::Const(const_trait_item) => {
                    return Err(format_err_spanned!(
                        const_trait_item,
                        "associated constants in ink! trait definitions are not supported, yet"
                    ))
                }
                syn::TraitItem::Macro(macro_trait_item) => {
                    return Err(format_err_spanned!(
                        macro_trait_item,
                        "macros in ink! trait definitions are not supported"
                    ))
                }
                syn::TraitItem::Type(type_trait_item) => {
                    return Err(format_err_spanned!(
                    type_trait_item,
                    "associated types in ink! trait definitions are not supported, yet"
                ))
                }
                syn::TraitItem::Verbatim(verbatim) => {
                    return Err(format_err_spanned!(
                        verbatim,
                        "encountered unsupported item in ink! trait definition"
                    ))
                }
                syn::TraitItem::Method(method_trait_item) => {
                    Self::analyse_methods(method_trait_item)?;
                }
                unknown => {
                    return Err(format_err_spanned!(
                        unknown,
                        "encountered unknown or unsupported item in ink! trait definition"
                    ))
                }
            }
        }
        Ok(())
    }

    /// Analyses an ink! method that can be either an ink! message or constructor.
    ///
    /// # Errors
    ///
    /// - If the method declared as `unsafe`, `const` or `async`.
    /// - If the method has some explicit API.
    /// - If the method is variadic or has generic parameters.
    /// - If the method does not respect the properties of either an
    ///   ink! message or ink! constructor.
    fn analyse_methods(method: &syn::TraitItemMethod) -> Result<()> {
        if let Some(default_impl) = &method.default {
            return Err(format_err_spanned!(
                default_impl,
                "ink! trait methods with default implementations are not supported"
            ))
        }
        if let Some(constness) = &method.sig.constness {
            return Err(format_err_spanned!(
                constness,
                "const ink! trait methods are not supported"
            ))
        }
        if let Some(asyncness) = &method.sig.asyncness {
            return Err(format_err_spanned!(
                asyncness,
                "async ink! trait methods are not supported"
            ))
        }
        if let Some(unsafety) = &method.sig.unsafety {
            return Err(format_err_spanned!(
                unsafety,
                "unsafe ink! trait methods are not supported"
            ))
        }
        if let Some(abi) = &method.sig.abi {
            return Err(format_err_spanned!(
                abi,
                "ink! trait methods with non default ABI are not supported"
            ))
        }
        if let Some(variadic) = &method.sig.variadic {
            return Err(format_err_spanned!(
                variadic,
                "variadic ink! trait methods are not supported"
            ))
        }
        if !method.sig.generics.params.is_empty() {
            return Err(format_err_spanned!(
                method.sig.generics.params,
                "generic ink! trait methods are not supported"
            ))
        }
        match ir::first_ink_attribute(&method.attrs) {
            Ok(Some(ink_attr)) => {
                match ink_attr.first().kind() {
                    ir::AttributeArg::Message => {
                        Self::analyse_message(method)?;
                    }
                    ir::AttributeArg::Constructor => {
                        Self::analyse_constructor(method)?;
                    }
                    _unsupported => {
                        return Err(format_err_spanned!(
                            method,
                            "encountered unsupported ink! attribute for ink! trait method",
                        ))
                    }
                }
            }
            Ok(None) => {
                return Err(format_err_spanned!(
                    method,
                    "missing #[ink(message)] or #[ink(constructor)] flags on ink! trait method"
                ))
            }
            Err(err) => return Err(err),
        }
        Ok(())
    }

    /// Analyses the properties of an ink! constructor.
    ///
    /// # Errors
    ///
    /// - If the constructor has a `self` receiver as first argument.
    /// - If the constructor has no `Self` return type.
    fn analyse_constructor(constructor: &syn::TraitItemMethod) -> Result<()> {
        InkTraitConstructor::extract_attributes(constructor.span(), &constructor.attrs)?;
        if let Some(receiver) = constructor.sig.receiver() {
            return Err(format_err_spanned!(
                receiver,
                "ink! constructors must not have a `self` receiver",
            ))
        }
        match &constructor.sig.output {
            syn::ReturnType::Default => {
                return Err(format_err_spanned!(
                    constructor.sig,
                    "ink! constructors must return Self"
                ))
            }
            syn::ReturnType::Type(_, ty) => {
                match &**ty {
                    syn::Type::Path(type_path) => {
                        if !type_path.path.is_ident("Self") {
                            return Err(format_err_spanned!(
                                type_path.path,
                                "ink! constructors must return Self"
                            ))
                        }
                    }
                    unknown => {
                        return Err(format_err_spanned!(
                            unknown,
                            "ink! constructors must return Self"
                        ))
                    }
                }
            }
        }
        Ok(())
    }

    /// Analyses the properties of an ink! message.
    ///
    /// # Errors
    ///
    /// - If the message has no `&self` or `&mut self` receiver.
    fn analyse_message(message: &syn::TraitItemMethod) -> Result<()> {
        InkTraitMessage::extract_attributes(message.span(), &message.attrs)?;
        match message.sig.receiver() {
            None | Some(syn::FnArg::Typed(_)) => {
                return Err(format_err_spanned!(
                message.sig,
                "missing or malformed `&self` or `&mut self` receiver for ink! message",
            ))
            }
            Some(syn::FnArg::Receiver(receiver)) => {
                if receiver.reference.is_none() {
                    return Err(format_err_spanned!(
                        receiver,
                        "self receiver of ink! message must be `&self` or `&mut self`"
                    ))
                }
            }
        }
        Ok(())
    }

    /// Extract selectors for ink! trait constructors and messages.
    ///
    /// The composed or manually specified selectors are stored into the provided
    /// hashtables for later look-up when querying ink! constructors or messages.
    /// This way we are more flexible with regard to the underlying structures of the IR.
    ///
    /// In this step we assume that all sanitation checks have taken place prior so
    /// instead of returning errors we simply panic upon failures.
    ///
    /// # Errors
    ///
    /// Returns an error if there are overlapping selectors for ink! constructors
    /// or ink! messages. Note that overlaps between ink! constructor and message
    /// selectors are allowed.
    fn extract_selectors(
        item_trait: &syn::ItemTrait,
        message_selectors: &mut HashMap<syn::Ident, Selector>,
        constructor_selectors: &mut HashMap<syn::Ident, Selector>,
    ) -> Result<()> {
        let mut seen_constructor_selectors = <HashMap<Selector, syn::Ident>>::new();
        let mut seen_message_selectors = <HashMap<Selector, syn::Ident>>::new();
        let (ink_attrs, _) = ir::sanitize_optional_attributes(
            item_trait.span(),
            item_trait.attrs.iter().cloned(),
            |arg| {
                match arg.kind() {
                    ir::AttributeArg::Namespace(_) => Ok(()),
                    _ => Err(None),
                }
            },
        )
        .expect("encountered unexpected invalid attributes on ink! trait definition");
        let namespace = ink_attrs
            .as_ref()
            .map(InkAttribute::namespace)
            .flatten()
            .unwrap_or_else(Default::default);
        let ident = &item_trait.ident;
        let trait_prefix = TraitPrefix::new(ident, &namespace);
        for callable in IterInkTraitItemsRaw::from_raw(item_trait) {
            let ident = callable.ident();
            let ink_attrs = callable.ink_attrs();
            let selector = match ink_attrs.selector() {
                Some(manual_selector) => manual_selector,
                None => Selector::compose(trait_prefix, ident),
            };
            let (duplicate_selector, duplicate_ident) = match callable {
                InkTraitItem::Constructor(_) => {
                    let duplicate_selector =
                        seen_constructor_selectors.insert(selector, ident.clone());
                    let duplicate_ident =
                        constructor_selectors.insert(ident.clone(), selector);
                    (duplicate_selector, duplicate_ident)
                }
                InkTraitItem::Message(_) => {
                    let duplicate_selector =
                        seen_message_selectors.insert(selector, ident.clone());
                    let duplicate_ident =
                        message_selectors.insert(ident.clone(), selector);
                    (duplicate_selector, duplicate_ident)
                }
            };
            if let Some(duplicate_selector) = duplicate_selector {
                use crate::error::ExtError as _;
                return Err(format_err_spanned!(
                    ident,
                    "encountered duplicate selector ({:x?}) in the same ink! trait definition",
                    selector.as_bytes(),
                ).into_combine(format_err_spanned!(
                    duplicate_selector,
                    "first ink! trait constructor or message with same selector found here",
                )))
            }
            assert!(
                duplicate_ident.is_none(),
                "encountered unexpected overlapping ink! trait constructor or message identifier",
            );
        }
        Ok(())
    }
}
