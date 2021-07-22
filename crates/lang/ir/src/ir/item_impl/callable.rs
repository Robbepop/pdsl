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

//! Utilities and helper routines that are useful for both ink! messages
//! and ink! constructors.

use crate::ir;
use core::fmt;
use proc_macro2::{
    Ident,
    Span,
};
use quote::ToTokens as _;
use syn::spanned::Spanned as _;

/// The kind of externally callable smart contract entity.
#[derive(Debug, Copy, Clone)]
pub enum CallableKind {
    /// An ink! message externally callable.
    Message,
    /// An ink! constructor externally callable.
    Constructor,
}

impl fmt::Display for CallableKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message => write!(f, "message"),
            Self::Constructor => write!(f, "constructor"),
        }
    }
}

/// Wrapper for a callable that adds its composed selector.
#[derive(Debug)]
pub struct CallableWithSelector<'a, C> {
    /// The composed selector computed by the associated implementation block
    /// and the given callable.
    composed_selector: ir::Selector,
    /// The parent implementation block.
    item_impl: &'a ir::ItemImpl,
    /// The actual callable.
    callable: &'a C,
}

impl<C> Copy for CallableWithSelector<'_, C> {}
impl<C> Clone for CallableWithSelector<'_, C> {
    fn clone(&self) -> Self {
        Self {
            composed_selector: self.composed_selector,
            item_impl: self.item_impl,
            callable: self.callable,
        }
    }
}

impl<'a, C> CallableWithSelector<'a, C>
where
    C: Callable,
{
    /// Creates a new wrapper around the given callable and parent `impl` block.
    pub(super) fn new(item_impl: &'a ir::ItemImpl, callable: &'a C) -> Self {
        Self {
            composed_selector: compose_selector(item_impl, callable),
            item_impl,
            callable,
        }
    }
}

impl<'a, C> CallableWithSelector<'a, C> {
    /// Returns the composed selector of the ink! callable the `impl` block.
    pub fn composed_selector(&self) -> ir::Selector {
        self.composed_selector
    }

    /// Returns a shared reference to the underlying callable.
    pub fn callable(&self) -> &'a C {
        self.callable
    }

    /// Returns the parent implementation block of the ink! callable.
    pub fn item_impl(&self) -> &'a ir::ItemImpl {
        self.item_impl
    }
}

impl<'a, C> Callable for CallableWithSelector<'a, C>
where
    C: Callable,
{
    fn kind(&self) -> CallableKind {
        <C as Callable>::kind(self.callable)
    }

    fn ident(&self) -> &Ident {
        <C as Callable>::ident(self.callable)
    }

    fn metadata_name(&self) -> String {
        <C as Callable>::metadata_name(self.callable)
    }

    fn user_provided_selector(&self) -> Option<&ir::Selector> {
        <C as Callable>::user_provided_selector(self.callable)
    }

    fn is_payable(&self) -> bool {
        <C as Callable>::is_payable(self.callable)
    }

    fn visibility(&self) -> Visibility {
        <C as Callable>::visibility(self.callable)
    }

    fn inputs(&self) -> InputsIter {
        <C as Callable>::inputs(self.callable)
    }

    fn inputs_span(&self) -> Span {
        <C as Callable>::inputs_span(self.callable)
    }

    fn statements(&self) -> &[syn::Stmt] {
        <C as Callable>::statements(self.callable)
    }
}

impl<'a, C> ::core::ops::Deref for CallableWithSelector<'a, C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        self.callable
    }
}

/// An ink! callable.
///
/// This is either an ink! message or an ink! constructor.
/// Used to share common behavior between different callable types.
pub trait Callable {
    /// Returns the kind of the ink! callable.
    fn kind(&self) -> CallableKind;

    /// Returns the identifier of the ink! callable.
    fn ident(&self) -> &Ident;

    /// Returns the name of method for metadata.
    fn metadata_name(&self) -> String;

    /// Returns the selector of the ink! callable if any has been manually set.
    fn user_provided_selector(&self) -> Option<&ir::Selector>;

    /// Returns `true` if the ink! callable is flagged as payable.
    ///
    /// # Note
    ///
    /// Flagging as payable is done using the `#[ink(payable)]` attribute.
    fn is_payable(&self) -> bool;

    /// Returns the visibility of the ink! callable.
    fn visibility(&self) -> Visibility;

    /// Returns an iterator yielding all input parameters of the ink! callable.
    fn inputs(&self) -> InputsIter;

    /// Returns the span of the inputs of the ink! callable.
    fn inputs_span(&self) -> Span;

    /// Returns a slice over shared references to the statements of the callable.
    fn statements(&self) -> &[syn::Stmt];
}

/// Returns the composed selector of the ink! callable.
///
/// Composition takes into account the given [`ir::ItemImpl`].
///
/// # Details
///
/// Given
/// - the callable's identifier `i`
/// - the optionally set callable's selector `s`
/// - the `impl` blocks trait path in case it implements a trait, `P`
/// - 16 kB blocks optional user provided namespace `S`
///
/// Then the selector is composed in the following way:
///
/// - If `s` is given we simply return `s`.
/// - Otherwise if `T` is not `None` (trait `impl` block) we concatenate
///   `S`, `T` and `i` with `::` as separator if `T` refers to a full-path.
///   If `T` refers to a relative path or is just an identifier we only take
///   its last segment `p` (e.g. the trait's identifier) into consideration
///   and use it instead of `P` in the above concatenation.
///   In the following we refer to the resulting concatenation as `C`.
/// - Now we take the BLAKE-2 hash of `C` which results in 32 bytes of output
///   and take the first 4 bytes that are returned in order as the composed
///   selector.
///
/// # Examples
///
/// ## Overriding the composed selector
///
/// Given
///
/// ```no_compile
/// impl MyStorage {
///     #[ink(message, selector = "0xDEADBEEF")]
///     fn my_message(&self) {}
/// }
/// ```
///
/// ... then the selector of `my_message` is simply `0xDEADBEEF` since it overrides
/// the composed selector.
///
/// ## Inherent implementation block
///
/// Given
///
/// ```no_compile
/// impl MyStorage {
///     #[ink(message)]
///     fn my_message(&self) {}
/// }
/// ```
///
/// ... then the selector of `my_message` is composed such as:
/// ```no_compile
/// BLAKE2("my_message".to_string().as_bytes())[0..4]
/// ```
///
/// ## Trait implementation block
///
/// Given
///
/// ```no_compile
/// impl MyTrait for MyStorage {
///     #[ink(message)]
///     fn my_message(&self) {}
/// }
/// ```
///
/// ... then the selector of `my_message` is composed such as:
/// ```no_compile
/// BLAKE2("MyTrait::my_message".to_string().as_bytes())[0..4]
/// ```
///
/// ## Using full path for trait
///
/// Given
///
/// ```no_compile
/// impl ::my_full::long_path::MyTrait for MyStorage {
///     #[ink(message)]
///     fn my_message(&self) {}
/// }
/// ```
///
/// ... then the selector of `my_message` is composed such as:
/// ```no_compile
/// BLAKE2("::my_full::long_path::MyTrait::my_message".to_string().as_bytes())[0..4]
/// ```
///
/// ## Using a namespace
///
/// Given
///
/// ```no_compile
/// #[ink(namespace = "my_namespace")]
/// impl MyTrait for MyStorage {
///     #[ink(message)]
///     fn my_message(&self) {}
/// }
/// ```
///
/// ... then the selector of `my_message` is composed such as:
/// ```no_compile
/// BLAKE2("my_namespace::MyTrait::my_message".to_string().as_bytes())[0..4]
/// ```
///
/// ## Note
///
/// All above examples work similarly for ink! constructors interchangeably.
///
/// ## Usage Recommendations
///
/// These recommendation mainly apply to trait implementation blocks:
///
/// - The recommendation by the ink! team is to use the full-path approach
/// wherever possible; OR import the trait and use only its identifier with
/// an additional namespace if required to disambiguate selectors.
/// - Try not to intermix the above recommendations.
/// - Avoid directly setting the selector of an ink! message or constructor.
///   Only do this if nothing else helps and you need a very specific selector,
///   e.g. in case of backwards compatibility.
/// - Do not use the namespace unless required to disambiguate.
pub fn compose_selector<C>(item_impl: &ir::ItemImpl, callable: &C) -> ir::Selector
where
    C: Callable,
{
    if let Some(selector) = callable.user_provided_selector() {
        return *selector
    }
    let callable_ident = callable.ident().to_string().into_bytes();
    let namespace_bytes = item_impl
        .namespace()
        .map(|namespace| namespace.as_bytes().to_vec())
        .unwrap_or_default();
    let separator = &b"::"[..];
    let joined = match item_impl.trait_path() {
        None => {
            // Inherent implementation block:
            if namespace_bytes.is_empty() {
                callable_ident
            } else {
                [namespace_bytes, callable_ident].join(separator)
            }
        }
        Some(path) => {
            // Trait implementation block:
            //
            // We need to separate between full-path, e.g. `::my::full::Path`
            // starting with `::` and relative paths for the composition.
            let path_bytes = if path.leading_colon.is_some() {
                let mut str_repr = path.to_token_stream().to_string();
                str_repr.retain(|c| !c.is_whitespace());
                str_repr.into_bytes()
            } else {
                path.segments
                    .last()
                    .expect("encountered empty trait path")
                    .ident
                    .to_string()
                    .into_bytes()
            };
            if namespace_bytes.is_empty() {
                [path_bytes, callable_ident].join(separator)
            } else {
                [namespace_bytes, path_bytes, callable_ident].join(separator)
            }
        }
    };
    ir::Selector::new(&joined)
}

/// Ensures that common invariants of externally callable ink! entities are met.
///
/// # Errors
///
/// In case any of the common externally callable invariants are not met:
/// - This is `true` if the externally callable is:
///  - generic
///  - `const` (compile-time evaluable)
///  - `async` (asynchronous WebAssembly smart contract calling is not allowed)
///  - `unsafe` (caller provided assertions not yet stable)
/// - Furthermore this is `true` if the externally callable is defined for a
///   non default ABI (e.g. `extern "C"`) or does not have valid visibility.
pub(super) fn ensure_callable_invariants(
    method_item: &syn::ImplItemMethod,
    kind: CallableKind,
) -> Result<(), syn::Error> {
    let bad_visibility = match &method_item.vis {
        syn::Visibility::Inherited => None,
        syn::Visibility::Restricted(vis_restricted) => Some(vis_restricted.span()),
        syn::Visibility::Crate(vis_crate) => Some(vis_crate.span()),
        syn::Visibility::Public(_) => None,
    };
    if let Some(bad_visibility) = bad_visibility {
        return Err(format_err!(
            bad_visibility,
            "ink! {}s must have public or inherited visibility",
            kind
        ))
    }
    if !method_item.sig.generics.params.is_empty() {
        return Err(format_err_spanned!(
            method_item.sig.generics.params,
            "ink! {}s must not be generic",
            kind,
        ))
    }
    if method_item.sig.constness.is_some() {
        return Err(format_err_spanned!(
            method_item.sig.constness,
            "ink! {}s must not be const",
            kind,
        ))
    }
    if method_item.sig.asyncness.is_some() {
        return Err(format_err_spanned!(
            method_item.sig.asyncness,
            "ink! {}s must not be async",
            kind,
        ))
    }
    if method_item.sig.unsafety.is_some() {
        return Err(format_err_spanned!(
            method_item.sig.unsafety,
            "ink! {}s must not be unsafe",
            kind,
        ))
    }
    if method_item.sig.abi.is_some() {
        return Err(format_err_spanned!(
            method_item.sig.abi,
            "ink! {}s must have explicit ABI",
            kind,
        ))
    }
    if method_item.sig.variadic.is_some() {
        return Err(format_err_spanned!(
            method_item.sig.variadic,
            "ink! {}s must not be variadic",
            kind,
        ))
    }

    if let Some(arg) = method_item.sig.inputs.iter().find(|input| {
        match input {
            syn::FnArg::Typed(pat) => !matches!(*pat.pat, syn::Pat::Ident(_)),
            _ => false,
        }
    }) {
        return Err(format_err_spanned!(
            arg,
            "ink! {} arguments must have an identifier",
            kind
        ))
    }
    Ok(())
}

/// The visibility of an ink! message or constructor.
#[derive(Debug, Clone)]
pub enum Visibility {
    Public(syn::VisPublic),
    Inherited,
}

impl Visibility {
    /// Returns `true` if the visibility of the ink! message of constructor is public (`pub`).
    ///
    /// # Note
    ///
    /// Messages in normal implementation blocks must have public visibility.
    pub fn is_pub(&self) -> bool {
        matches!(self, Self::Public(_))
    }

    /// Returns `true` if the visibility of the ink! message of constructor is inherited.
    ///
    /// # Note
    ///
    /// Messages in trait implementation blocks must have inherited visibility.
    pub fn is_inherited(&self) -> bool {
        matches!(self, Self::Inherited)
    }

    /// Returns the associated span if any.
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::Public(vis_public) => Some(vis_public.span()),
            Self::Inherited => None,
        }
    }
}

/// Iterator over the input parameters of an ink! message or constructor.
///
/// Does not yield the self receiver of ink! messages.
pub struct InputsIter<'a> {
    iter: syn::punctuated::Iter<'a, syn::FnArg>,
}

impl<'a> From<&'a ir::Message> for InputsIter<'a> {
    fn from(message: &'a ir::Message) -> Self {
        Self {
            iter: message.item.sig.inputs.iter(),
        }
    }
}

impl<'a> From<&'a ir::Constructor> for InputsIter<'a> {
    fn from(constructor: &'a ir::Constructor) -> Self {
        Self {
            iter: constructor.item.sig.inputs.iter(),
        }
    }
}

impl<'a> Iterator for InputsIter<'a> {
    type Item = &'a syn::PatType;

    fn next(&mut self) -> Option<Self::Item> {
        'repeat: loop {
            match self.iter.next() {
                None => return None,
                Some(syn::FnArg::Typed(pat_typed)) => return Some(pat_typed),
                Some(syn::FnArg::Receiver(_)) => continue 'repeat,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::{
        convert::TryFrom,
        fmt::Debug,
    };

    pub enum ExpectedSelector {
        Raw([u8; 4]),
        Blake2(Vec<u8>),
    }

    impl From<[u8; 4]> for ExpectedSelector {
        fn from(raw_selector: [u8; 4]) -> Self {
            ExpectedSelector::Raw(raw_selector)
        }
    }

    impl From<Vec<u8>> for ExpectedSelector {
        fn from(blake2_input: Vec<u8>) -> Self {
            ExpectedSelector::Blake2(blake2_input)
        }
    }

    impl ExpectedSelector {
        pub fn expected_selector(self) -> ir::Selector {
            match self {
                Self::Raw(raw_selector) => ir::Selector::from_bytes(raw_selector),
                Self::Blake2(blake2_input) => ir::Selector::new(&blake2_input),
            }
        }
    }

    /// Asserts that the given ink! implementation block and the given ink!
    /// message result in the same composed selector as the expected bytes.
    fn assert_compose_selector<C, S>(
        item_impl: syn::ItemImpl,
        item_method: syn::ImplItemMethod,
        expected_selector: S,
    ) where
        C: Callable + TryFrom<syn::ImplItemMethod>,
        <C as TryFrom<syn::ImplItemMethod>>::Error: Debug,
        S: Into<ExpectedSelector>,
    {
        assert_eq!(
            compose_selector(
                &<ir::ItemImpl as TryFrom<syn::ItemImpl>>::try_from(item_impl).unwrap(),
                &<C as TryFrom<syn::ImplItemMethod>>::try_from(item_method).unwrap(),
            ),
            expected_selector.into().expected_selector(),
        )
    }

    #[test]
    fn compose_selector_works() {
        assert_compose_selector::<ir::Message, _>(
            syn::parse_quote! {
                #[ink(impl)]
                impl MyStorage {}
            },
            syn::parse_quote! {
                #[ink(message)]
                fn my_message(&self) {}
            },
            b"my_message".to_vec(),
        );
        assert_compose_selector::<ir::Message, _>(
            syn::parse_quote! {
                #[ink(impl)]
                impl MyTrait for MyStorage {}
            },
            syn::parse_quote! {
                #[ink(message)]
                fn my_message(&self) {}
            },
            b"MyTrait::my_message".to_vec(),
        );
        assert_compose_selector::<ir::Message, _>(
            syn::parse_quote! {
                #[ink(impl)]
                impl ::my::full::path::MyTrait for MyStorage {}
            },
            syn::parse_quote! {
                #[ink(message)]
                fn my_message(&self) {}
            },
            b"::my::full::path::MyTrait::my_message".to_vec(),
        );
        assert_compose_selector::<ir::Message, _>(
            syn::parse_quote! {
                #[ink(impl, namespace = "my_namespace")]
                impl MyTrait for MyStorage {}
            },
            syn::parse_quote! {
                #[ink(message)]
                fn my_message(&self) {}
            },
            b"my_namespace::MyTrait::my_message".to_vec(),
        );
        assert_compose_selector::<ir::Message, _>(
            syn::parse_quote! {
                #[ink(impl)]
                impl MyTrait for MyStorage {}
            },
            syn::parse_quote! {
                #[ink(message, selector = "0xDEADBEEF")]
                fn my_message(&self) {}
            },
            [0xDE, 0xAD, 0xBE, 0xEF],
        );
        assert_compose_selector::<ir::Message, _>(
            syn::parse_quote! {
                #[ink(impl)]
                impl relative::path_to::MyTrait for MyStorage {}
            },
            syn::parse_quote! {
                #[ink(message)]
                fn my_message(&self) {}
            },
            b"MyTrait::my_message".to_vec(),
        );
    }
}
