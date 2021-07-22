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

use crate::{
    error::ExtError as _,
    ir,
    ir::attrs::Attrs as _,
};
use core::convert::TryFrom;
use proc_macro2::{
    Ident,
    Span,
};

mod callable;
mod constructor;
mod impl_item;
mod iter;
mod message;

#[cfg(test)]
mod tests;

use self::callable::ensure_callable_invariants;
pub use self::{
    callable::{
        Callable,
        CallableKind,
        CallableWithSelector,
        InputsIter,
        Visibility,
    },
    constructor::Constructor,
    impl_item::ImplItem,
    iter::{
        IterConstructors,
        IterMessages,
    },
    message::{
        Message,
        Receiver,
    },
};
use quote::TokenStreamExt as _;
use syn::spanned::Spanned;

/// An ink! implementation block.
///
/// # Note
///
/// - This can be either an inherent implementation block that implements some
///   constructors, messages or internal functions for the storage struct; OR it
///   can be a trait implementation for the storage struct.
/// - We try to support all fields that are supported by the underlying `syn`
///   implementation for [`syn::ItemImpl`] even though they are not really
///   required to represent ink!. This is done for consistency with `syn`.
#[derive(Debug, PartialEq, Eq)]
pub struct ItemImpl {
    attrs: Vec<syn::Attribute>,
    defaultness: Option<syn::token::Default>,
    unsafety: Option<syn::token::Unsafe>,
    impl_token: syn::token::Impl,
    generics: syn::Generics,
    trait_: Option<(Option<syn::token::Bang>, syn::Path, syn::token::For)>,
    self_ty: Box<syn::Type>,
    brace_token: syn::token::Brace,
    items: Vec<ImplItem>,
    /// A namespace to disambiguate trait implementation blocks with equal
    /// names. Generally can be used to change computation of message and
    /// constructor selectors of the implementation block.
    namespace: Option<ir::Namespace>,
    /// A name of trait used in metadata.
    ///
    /// # Note
    ///
    /// User can provide name of trait which will be used inside of metadata.
    /// But he is not obliged to implement this trait.
    trait_metadata_name: Option<String>,
}

impl quote::ToTokens for ItemImpl {
    /// We mainly implement this trait for this ink! type to have a derived
    /// [`Spanned`](`syn::spanned::Spanned`) implementation for it.
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.append_all(
            self.attrs
                .iter()
                .filter(|attr| matches!(attr.style, syn::AttrStyle::Outer)),
        );
        self.defaultness.to_tokens(tokens);
        self.unsafety.to_tokens(tokens);
        self.impl_token.to_tokens(tokens);
        self.generics.to_tokens(tokens);
        if let Some((polarity, path, for_token)) = &self.trait_ {
            polarity.to_tokens(tokens);
            path.to_tokens(tokens);
            for_token.to_tokens(tokens);
        }
        self.self_ty.to_tokens(tokens);
        self.generics.where_clause.to_tokens(tokens);
        self.brace_token.surround(tokens, |tokens| {
            tokens.append_all(
                self.attrs
                    .iter()
                    .filter(|attr| matches!(attr.style, syn::AttrStyle::Inner(_))),
            );
            tokens.append_all(&self.items);
        });
    }
}

impl ItemImpl {
    /// Returns `true` if the Rust implementation block is an ink! implementation
    /// block.
    ///
    /// # Note
    ///
    /// This is the case if:
    ///
    /// - The ink! implementation block has been annotated as in:
    ///
    /// ```
    /// # use core::convert::TryFrom;
    /// # <ink_lang_ir::ItemImpl as TryFrom<syn::ItemImpl>>::try_from(syn::parse_quote! {
    /// #[ink(impl)]
    /// impl MyStorage {
    ///     fn my_function(&self) {
    ///         /* inherent method implementation */
    ///         unimplemented!()
    ///     }
    /// }
    /// # }).unwrap();
    /// ```
    ///
    /// - Or if any of the ink! implementation block methods do have ink!
    ///   specific annotations:
    ///
    /// ```
    /// # use core::convert::TryFrom;
    /// # <ink_lang_ir::ItemImpl as TryFrom<syn::ItemImpl>>::try_from(syn::parse_quote! {
    /// impl MyStorage {
    ///     #[ink(constructor)]
    ///     pub fn my_constructor() -> Self {
    ///         /* constructor implementation */
    ///         unimplemented!()
    ///     }
    /// }
    /// # }).unwrap();
    /// ```
    ///
    /// The same rules apply to ink! trait implementation blocks.
    ///
    /// # Errors
    ///
    /// Returns an error in case of encountered malformed ink! attributes.
    pub(super) fn is_ink_impl_block(
        item_impl: &syn::ItemImpl,
    ) -> Result<bool, syn::Error> {
        // Quick check in order to efficiently bail out in case where there are
        // no ink! attributes:
        if !ir::contains_ink_attributes(&item_impl.attrs)
            && item_impl
                .items
                .iter()
                .all(|item| !ir::contains_ink_attributes(item.attrs()))
        {
            return Ok(false)
        }
        // Check if the implementation block itself has been annotated with
        // `#[ink(impl)]` and return `true` if this is the case.
        let (ink_attrs, _) = ir::partition_attributes(item_impl.attrs.clone())?;
        let impl_block_span = item_impl.span();
        if !ink_attrs.is_empty() {
            let normalized =
                ir::InkAttribute::from_expanded(ink_attrs).map_err(|err| {
                    err.into_combine(format_err!(impl_block_span, "at this invocation",))
                })?;
            if normalized
                .ensure_first(&ir::AttributeArgKind::Implementation)
                .is_ok()
            {
                return Ok(true)
            }
        }
        // Check if any of the implementation block's methods either resembles
        // an ink! constructor or an ink! message:
        'repeat: for item in &item_impl.items {
            match item {
                syn::ImplItem::Method(method_item) => {
                    if !ir::contains_ink_attributes(&method_item.attrs) {
                        continue 'repeat
                    }
                    let attr = ir::first_ink_attribute(&method_item.attrs)?
                        .expect("missing expected ink! attribute for struct");
                    match attr.first().kind() {
                        ir::AttributeArg::Constructor | ir::AttributeArg::Message => {
                            return Ok(true)
                        }
                        _ => continue 'repeat,
                    }
                }
                _ => continue 'repeat,
            }
        }
        Ok(false)
    }
}

impl TryFrom<syn::ItemImpl> for ItemImpl {
    type Error = syn::Error;

    fn try_from(item_impl: syn::ItemImpl) -> Result<Self, Self::Error> {
        let impl_block_span = item_impl.span();
        if !Self::is_ink_impl_block(&item_impl)? {
            return Err(format_err_spanned!(
                item_impl,
                "missing ink! annotations on 16 kB block or on any of its items"
            ))
        }
        if let Some(defaultness) = item_impl.defaultness {
            return Err(format_err_spanned!(
                defaultness,
                "default implementations are unsupported for ink! implementation blocks",
            ))
        }
        if let Some(unsafety) = item_impl.unsafety {
            return Err(format_err_spanned!(
                unsafety,
                "unsafe ink! implementation blocks are not supported",
            ))
        }
        if !item_impl.generics.params.is_empty() {
            return Err(format_err_spanned!(
                item_impl.generics.params,
                "generic ink! implementation blocks are not supported",
            ))
        }
        let impl_items = item_impl
            .items
            .into_iter()
            .map(<ImplItem as TryFrom<_>>::try_from)
            .collect::<Result<Vec<_>, syn::Error>>()?;
        let is_trait_impl = item_impl.trait_.is_some();
        for impl_item in &impl_items {
            /// Ensures that visibility of ink! messages and constructors is
            /// valid in dependency of the containing ink! `impl` block.
            ///
            /// # Note
            ///
            /// Trait implementation blocks expect inherited visibility
            /// while inherent implementation block expect public visibility.
            fn ensure_valid_visibility(
                vis: ir::Visibility,
                span: Span,
                what: &str,
                is_trait_impl: bool,
            ) -> Result<(), syn::Error> {
                let requires_pub = !is_trait_impl;
                if requires_pub != vis.is_pub() {
                    return Err(format_err!(
                        span,
                        "ink! {} in {} impl blocks must have {} visibility",
                        what,
                        if is_trait_impl { "trait" } else { "inherent" },
                        if requires_pub { "public" } else { "inherited" },
                    ))
                }
                Ok(())
            }
            match impl_item {
                ir::ImplItem::Message(message) => {
                    ensure_valid_visibility(
                        message.visibility(),
                        message.item.span(),
                        "message",
                        is_trait_impl,
                    )?;
                }
                ir::ImplItem::Constructor(constructor) => {
                    ensure_valid_visibility(
                        constructor.visibility(),
                        constructor.item.span(),
                        "constructor",
                        is_trait_impl,
                    )?;
                }
                _ => (),
            }
        }
        let (ink_attrs, other_attrs) = ir::partition_attributes(item_impl.attrs)?;
        let mut namespace = None;
        let mut trait_metadata_name = None;
        if !ink_attrs.is_empty() {
            let normalized =
                ir::InkAttribute::from_expanded(ink_attrs).map_err(|err| {
                    err.into_combine(format_err!(impl_block_span, "at this invocation",))
                })?;
            normalized.ensure_no_conflicts(|arg| {
                match arg.kind() {
                    ir::AttributeArg::Implementation
                    | ir::AttributeArg::Namespace(_)
                    | ir::AttributeArg::MetadataName(_) => Ok(()),
                    _ => Err(None),
                }
            })?;
            namespace = normalized.namespace();
            trait_metadata_name = normalized.metadata_name();
        }
        Ok(Self {
            attrs: other_attrs,
            defaultness: item_impl.defaultness,
            unsafety: item_impl.unsafety,
            impl_token: item_impl.impl_token,
            generics: item_impl.generics,
            trait_: item_impl.trait_,
            self_ty: item_impl.self_ty,
            brace_token: item_impl.brace_token,
            items: impl_items,
            namespace,
            trait_metadata_name,
        })
    }
}

impl ItemImpl {
    /// Returns all non-ink! specific attributes of the implementation block.
    pub fn attrs(&self) -> &[syn::Attribute] {
        &self.attrs
    }

    /// Returns the `Self` type of the implementation block.
    pub fn self_type(&self) -> &syn::Type {
        self.self_ty.as_ref()
    }

    /// Returns the trait type path if this is a trait implementation block.
    ///
    /// Returns `None` if this is an inherent implementation block.
    pub fn trait_path(&self) -> Option<&syn::Path> {
        self.trait_.as_ref().map(|(_, path, _)| path)
    }

    /// Returns the trait identifier if this is a trait implementation block.
    ///
    /// Returns `None` if this is an inherent implementation block.
    pub fn trait_ident(&self) -> Option<&Ident> {
        self.trait_path()
            .map(|trait_path| trait_path.segments.last())
            .flatten()
            .map(|segment| &segment.ident)
    }

    /// Returns the namespace of the implementation block if any has been provided.
    pub fn namespace(&self) -> Option<&ir::Namespace> {
        self.namespace.as_ref()
    }

    /// Returns the trait metadata name of the implementation block if any has been provided.
    ///
    /// # Note
    ///
    /// If metadata name is `""`, it means that user wants to remove trait naming in metadata
    pub fn trait_metadata_name(&self) -> Option<String> {
        if self.trait_metadata_name.is_some() {
            let name = self.trait_metadata_name.clone();
            if name.as_ref().unwrap().is_empty() {
                None
            } else {
                name
            }
        } else {
            self.trait_path()
                .map(|path| path.segments.last().map(|seg| seg.ident.to_string()))
                .flatten()
        }
    }

    /// Returns an iterator yielding the ink! messages of the implementation block.
    pub fn iter_messages(&self) -> IterMessages {
        IterMessages::new(self)
    }

    /// Returns an iterator yielding the ink! messages of the implementation block.
    pub fn iter_constructors(&self) -> IterConstructors {
        IterConstructors::new(self)
    }

    /// Returns a slice over the shared references of the items of the `impl`.
    pub fn items(&self) -> &[ir::ImplItem] {
        &self.items
    }
}
