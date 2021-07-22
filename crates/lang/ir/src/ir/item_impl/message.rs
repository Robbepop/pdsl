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

use super::{
    ensure_callable_invariants,
    Callable,
    CallableKind,
    InputsIter,
    Visibility,
};
use crate::ir;
use core::convert::TryFrom;
use proc_macro2::{
    Ident,
    Span,
};
use syn::spanned::Spanned as _;

/// The receiver of an ink! message.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Receiver {
    /// The `&self` message receiver.
    Ref,
    /// The `&mut self` message receiver.
    RefMut,
}

impl quote::ToTokens for Receiver {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let receiver = match self {
            Self::Ref => quote::quote! { &self },
            Self::RefMut => quote::quote! { &mut self },
        };
        tokens.extend(receiver);
    }
}

impl Receiver {
    /// Returns `true` if the receiver is `&self`.
    pub fn is_ref(self) -> bool {
        matches!(self, Self::Ref)
    }

    /// Returns `true` if the receiver is `&mut self`.
    pub fn is_ref_mut(self) -> bool {
        matches!(self, Self::RefMut)
    }
}

/// An ink! message definition.
///
/// # Example
///
/// ## Inherent implementation message:
///
/// ```
/// # use core::convert::TryFrom;
/// # <ink_lang_ir::ItemImpl as TryFrom<syn::ItemImpl>>::try_from(syn::parse_quote! {
/// impl MyStorage {
///     #[ink(message)]
///     pub fn my_message(&self, input: i32) -> bool {
///         /* message implementation goes here */
/// #       unimplemented!()
///     }
/// }
/// # }).unwrap();
/// ```
///
/// ## Trait implementation message:
///
/// ```
/// # use core::convert::TryFrom;
/// # let event = <ink_lang_ir::ItemImpl as TryFrom<syn::ItemImpl>>::try_from(syn::parse_quote! {
/// impl MyTrait for MyStorage {
///     #[ink(message)]
///     fn my_message(&mut self, input: bool) -> i32 {
///         /* message implementation goes here */
/// #       unimplemented!()
///     }
/// }
/// # }).unwrap();
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    /// The underlying Rust method item.
    pub(super) item: syn::ImplItemMethod,
    /// If the ink! message can receive funds.
    is_payable: bool,
    /// An optional user provided selector.
    ///
    /// # Note
    ///
    /// This overrides the computed selector, even when using a manual namespace
    /// for the parent implementation block.
    selector: Option<ir::Selector>,
    /// An optional user provided name of method for metadata.
    ///
    /// # Note
    ///
    /// This overrides the default name of method inside of metadata.
    metadata_name: Option<String>,
}

impl quote::ToTokens for Message {
    /// We mainly implement this trait for this ink! type to have a derived
    /// [`Spanned`](`syn::spanned::Spanned`) implementation for it.
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.item.to_tokens(tokens)
    }
}

impl Message {
    /// Ensures that the given method inputs start with `&self` or `&mut self`
    /// receivers.
    ///
    /// If not an appropriate error is returned.
    ///
    /// # Errors
    ///
    /// - If the method inputs yields no elements.
    /// - If the first method input is not `&self` or `&mut self`.
    fn ensure_receiver_is_self_ref(
        method_item: &syn::ImplItemMethod,
    ) -> Result<(), syn::Error> {
        let mut fn_args = method_item.sig.inputs.iter();
        fn bail(span: Span) -> syn::Error {
            format_err!(
                span,
                "ink! messages must have `&self` or `&mut self` receiver",
            )
        }
        match fn_args.next() {
            None => return Err(bail(method_item.span())),
            Some(syn::FnArg::Typed(pat_typed)) => return Err(bail(pat_typed.span())),
            Some(syn::FnArg::Receiver(receiver)) => {
                if receiver.reference.is_none() {
                    return Err(bail(receiver.span()))
                }
            }
        }
        Ok(())
    }

    /// Ensures that the ink! message does not return `Self`.
    ///
    /// # Errors
    ///
    /// If the given Rust method has a `Self` return type.
    fn ensure_not_return_self(
        method_item: &syn::ImplItemMethod,
    ) -> Result<(), syn::Error> {
        match &method_item.sig.output {
            syn::ReturnType::Default => (),
            syn::ReturnType::Type(_arrow, ret_type) => {
                if let syn::Type::Path(type_path) = &**ret_type {
                    if type_path.path.is_ident("Self") {
                        return Err(format_err!(
                            ret_type,
                            "ink! messages must not return `Self`"
                        ))
                    }
                }
            }
        }
        Ok(())
    }

    /// Sanitizes the attributes for the ink! message.
    ///
    /// Returns a tuple of ink! attributes and non-ink! attributes.
    fn sanitize_attributes(
        method_item: &syn::ImplItemMethod,
    ) -> Result<(ir::InkAttribute, Vec<syn::Attribute>), syn::Error> {
        ir::sanitize_attributes(
            method_item.span(),
            method_item.attrs.clone(),
            &ir::AttributeArgKind::Message,
            |arg| {
                match arg.kind() {
                    ir::AttributeArg::Message
                    | ir::AttributeArg::Payable
                    | ir::AttributeArg::Selector(_)
                    | ir::AttributeArg::MetadataName(_) => Ok(()),
                    _ => Err(None),
                }
            },
        )
    }
}

impl TryFrom<syn::ImplItemMethod> for Message {
    type Error = syn::Error;

    fn try_from(method_item: syn::ImplItemMethod) -> Result<Self, Self::Error> {
        ensure_callable_invariants(&method_item, CallableKind::Message)?;
        Self::ensure_receiver_is_self_ref(&method_item)?;
        Self::ensure_not_return_self(&method_item)?;
        let (ink_attrs, other_attrs) = Self::sanitize_attributes(&method_item)?;
        let is_payable = ink_attrs.is_payable();
        let selector = ink_attrs.selector();
        let metadata_name = ink_attrs.metadata_name();
        Ok(Self {
            is_payable,
            selector,
            metadata_name,
            item: syn::ImplItemMethod {
                attrs: other_attrs,
                ..method_item
            },
        })
    }
}

impl Callable for Message {
    fn kind(&self) -> CallableKind {
        CallableKind::Message
    }

    fn ident(&self) -> &Ident {
        &self.item.sig.ident
    }

    fn metadata_name(&self) -> String {
        if self.metadata_name.is_some() {
            self.metadata_name.clone().unwrap()
        } else {
            self.item.sig.ident.to_string()
        }
    }

    fn user_provided_selector(&self) -> Option<&ir::Selector> {
        self.selector.as_ref()
    }

    fn is_payable(&self) -> bool {
        self.is_payable
    }

    fn visibility(&self) -> Visibility {
        match &self.item.vis {
            syn::Visibility::Public(vis_public) => Visibility::Public(vis_public.clone()),
            syn::Visibility::Inherited => Visibility::Inherited,
            _ => unreachable!("encountered invalid visibility for ink! message"),
        }
    }

    fn inputs(&self) -> InputsIter {
        InputsIter::from(self)
    }

    fn inputs_span(&self) -> Span {
        self.item.sig.inputs.span()
    }

    fn statements(&self) -> &[syn::Stmt] {
        &self.item.block.stmts
    }
}

impl Message {
    /// Returns a slice of all non-ink! attributes of the ink! message.
    pub fn attrs(&self) -> &[syn::Attribute] {
        &self.item.attrs
    }

    /// Returns the `self` receiver of the ink! message.
    pub fn receiver(&self) -> Receiver {
        match self.item.sig.inputs.iter().next() {
            Some(syn::FnArg::Receiver(receiver)) => {
                debug_assert!(receiver.reference.is_some());
                if receiver.mutability.is_some() {
                    Receiver::RefMut
                } else {
                    Receiver::Ref
                }
            }
            _ => unreachable!("encountered invalid receiver argument for ink! message"),
        }
    }

    /// Returns the return type of the ink! message if any.
    pub fn output(&self) -> Option<&syn::Type> {
        match &self.item.sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, return_type) => Some(return_type),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_works() {
        let test_inputs: Vec<(Option<syn::Type>, syn::ImplItemMethod)> = vec![
            (
                // No output:
                None,
                syn::parse_quote! {
                    #[ink(message)]
                    fn my_message(&self) {}
                },
            ),
            (
                // Single output:
                Some(syn::parse_quote! { i32 }),
                syn::parse_quote! {
                    #[ink(message)]
                    fn my_message(&self) -> i32 {}
                },
            ),
            (
                // Tuple output:
                Some(syn::parse_quote! { (i32, u64, bool) }),
                syn::parse_quote! {
                    #[ink(message)]
                    fn my_message(&self) -> (i32, u64, bool) {}
                },
            ),
        ];
        for (expected_output, item_method) in test_inputs {
            let actual_output = <ir::Message as TryFrom<_>>::try_from(item_method)
                .unwrap()
                .output()
                .cloned();
            assert_eq!(actual_output, expected_output);
        }
    }

    #[test]
    fn inputs_works() {
        macro_rules! expected_inputs {
            ( $( $name:ident: $ty:ty ),* ) => {{
                vec![
                    $(
                        syn::parse_quote! {
                            $name: $ty
                        }
                    ),*
                ]
            }};
        }
        let test_inputs: Vec<(Vec<syn::FnArg>, syn::ImplItemMethod)> = vec![
            (
                // No inputs:
                expected_inputs!(),
                syn::parse_quote! {
                    #[ink(message)]
                    fn my_message(&self) {}
                },
            ),
            (
                // Single input:
                expected_inputs!(a: i32),
                syn::parse_quote! {
                    #[ink(message)]
                    fn my_message(&self, a: i32) {}
                },
            ),
            (
                // Some inputs:
                expected_inputs!(a: i32, b: u64, c: [u8; 32]),
                syn::parse_quote! {
                    #[ink(message)]
                    fn my_message(&self, a: i32, b: u64, c: [u8; 32]) {}
                },
            ),
        ];
        for (expected_inputs, item_method) in test_inputs {
            let actual_inputs = <ir::Message as TryFrom<_>>::try_from(item_method)
                .unwrap()
                .inputs()
                .cloned()
                .map(|pat_type| syn::FnArg::Typed(pat_type))
                .collect::<Vec<_>>();
            assert_eq!(actual_inputs, expected_inputs);
        }
    }

    #[test]
    fn is_payable_works() {
        let test_inputs: Vec<(bool, syn::ImplItemMethod)> = vec![
            // Not payable.
            (
                false,
                syn::parse_quote! {
                    #[ink(message)]
                    fn my_message(&self) {}
                },
            ),
            // Normalized ink! attribute.
            (
                true,
                syn::parse_quote! {
                    #[ink(message, payable)]
                    pub fn my_message(&self) {}
                },
            ),
            // Different ink! attributes.
            (
                true,
                syn::parse_quote! {
                    #[ink(message)]
                    #[ink(payable)]
                    pub fn my_message(&self) {}
                },
            ),
            // Another ink! attribute, separate and normalized attribute.
            (
                true,
                syn::parse_quote! {
                    #[ink(message)]
                    #[ink(selector = "0xDEADBEEF", payable)]
                    pub fn my_message(&self) {}
                },
            ),
            // Another ink! attribute, separate and normalized attribute.
            (
                true,
                syn::parse_quote! {
                    #[ink(message)]
                    #[ink(selector = "0xDEADBEEF", payable, metadata_name = "your_message")]
                    pub fn my_message(&self) {}
                },
            ),
        ];
        for (expect_payable, item_method) in test_inputs {
            let is_payable = <ir::Message as TryFrom<_>>::try_from(item_method)
                .unwrap()
                .is_payable();
            assert_eq!(is_payable, expect_payable);
        }
    }

    #[test]
    fn receiver_works() {
        let test_inputs: Vec<(Receiver, syn::ImplItemMethod)> = vec![
            (
                Receiver::Ref,
                syn::parse_quote! {
                    #[ink(message)]
                    fn my_message(&self) {}
                },
            ),
            (
                Receiver::RefMut,
                syn::parse_quote! {
                    #[ink(message, payable)]
                    fn my_message(&mut self) {}
                },
            ),
        ];
        for (expected_receiver, item_method) in test_inputs {
            let actual_receiver = <ir::Message as TryFrom<_>>::try_from(item_method)
                .unwrap()
                .receiver();
            assert_eq!(actual_receiver, expected_receiver);
        }
    }

    #[test]
    fn visibility_works() {
        let test_inputs: Vec<(bool, syn::ImplItemMethod)> = vec![
            // &self
            (
                false,
                syn::parse_quote! {
                    #[ink(message)]
                    fn my_message(&self) {}
                },
            ),
            // &self + pub
            (
                true,
                syn::parse_quote! {
                    #[ink(message)]
                    pub fn my_message(&self) {}
                },
            ),
            // &mut self
            (
                false,
                syn::parse_quote! {
                    #[ink(message)]
                    fn my_message(&mut self) {}
                },
            ),
            // &mut self + pub
            (
                true,
                syn::parse_quote! {
                    #[ink(message)]
                    pub fn my_message(&mut self) {}
                },
            ),
        ];
        for (is_pub, item_method) in test_inputs {
            let visibility = <ir::Message as TryFrom<_>>::try_from(item_method)
                .unwrap()
                .visibility();
            assert_eq!(visibility.is_pub(), is_pub);
            assert_eq!(visibility.is_inherited(), !is_pub);
        }
    }

    #[test]
    fn try_from_works() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            // &self
            syn::parse_quote! {
                #[ink(message)]
                fn my_message(&self) {}
            },
            // &self + pub
            syn::parse_quote! {
                #[ink(message)]
                pub fn my_message(&self) {}
            },
            // &mut self
            syn::parse_quote! {
                #[ink(message)]
                fn my_message(&mut self) {}
            },
            // &mut self + pub
            syn::parse_quote! {
                #[ink(message)]
                pub fn my_message(&mut self) {}
            },
            // &self + payable
            syn::parse_quote! {
                #[ink(message, payable)]
                fn my_message(&self) {}
            },
            // &mut self + payable
            syn::parse_quote! {
                #[ink(message, payable)]
                fn my_message(&mut self) {}
            },
            // &self + many inputs + output works
            syn::parse_quote! {
                #[ink(message)]
                fn my_message(&self, input1: i32, input2: i64, input3: u32, input4: u64) -> bool {}
            },
            // &mut self + many inputs + output works
            syn::parse_quote! {
                #[ink(message)]
                fn my_message(&mut self, input1: i32, input2: i64, input3: u32, input4: u64) -> bool {}
            },
        ];
        for item_method in item_methods {
            assert!(<ir::Message as TryFrom<_>>::try_from(item_method).is_ok());
        }
    }

    fn assert_try_from_fails(item_method: syn::ImplItemMethod, expected_err: &str) {
        assert_eq!(
            <ir::Message as TryFrom<_>>::try_from(item_method)
                .map_err(|err| err.to_string()),
            Err(expected_err.to_string()),
        );
    }

    #[test]
    fn try_from_generics_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            syn::parse_quote! {
                #[ink(message)]
                fn my_message<T>(&self) {}
            },
            syn::parse_quote! {
                #[ink(message)]
                pub fn my_message<T>(&self) {}
            },
            syn::parse_quote! {
                #[ink(message)]
                fn my_message<T>(&mut self) {}
            },
            syn::parse_quote! {
                #[ink(message)]
                pub fn my_message<T>(&mut self) {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(item_method, "ink! messages must not be generic")
        }
    }

    #[test]
    fn try_from_receiver_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            syn::parse_quote! {
                #[ink(message)]
                fn my_message() {}
            },
            syn::parse_quote! {
                #[ink(message)]
                fn my_message(self) {}
            },
            syn::parse_quote! {
                #[ink(message)]
                pub fn my_message(mut self) {}
            },
            syn::parse_quote! {
                #[ink(message)]
                fn my_message(this: &Self) {}
            },
            syn::parse_quote! {
                #[ink(message)]
                pub fn my_message(this: &mut Self) {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(
                item_method,
                "ink! messages must have `&self` or `&mut self` receiver",
            )
        }
    }

    #[test]
    fn try_from_const_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            // &self
            syn::parse_quote! {
                #[ink(message)]
                const fn my_message(&self) {}
            },
            // &mut self
            syn::parse_quote! {
                #[ink(message)]
                const fn my_message(&mut self) {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(item_method, "ink! messages must not be const")
        }
    }

    #[test]
    fn try_from_async_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            // &self
            syn::parse_quote! {
                #[ink(message)]
                async fn my_message(&self) {}
            },
            // &mut self
            syn::parse_quote! {
                #[ink(message)]
                async fn my_message(&mut self) {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(item_method, "ink! messages must not be async")
        }
    }

    #[test]
    fn try_from_unsafe_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            // &self
            syn::parse_quote! {
                #[ink(message)]
                unsafe fn my_message(&self) {}
            },
            // &mut self
            syn::parse_quote! {
                #[ink(message)]
                unsafe fn my_message(&mut self) {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(item_method, "ink! messages must not be unsafe")
        }
    }

    #[test]
    fn try_from_explicit_abi_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            // &self
            syn::parse_quote! {
                #[ink(message)]
                extern "C" fn my_message(&self) {}
            },
            // &mut self
            syn::parse_quote! {
                #[ink(message)]
                extern "C" fn my_message(&mut self) {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(item_method, "ink! messages must have explicit ABI")
        }
    }

    #[test]
    fn try_from_variadic_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            // &self
            syn::parse_quote! {
                #[ink(message)]
                fn my_message(&self, ...) {}
            },
            // &mut self
            syn::parse_quote! {
                #[ink(message)]
                fn my_message(&mut self, ...) {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(item_method, "ink! messages must not be variadic")
        }
    }

    #[test]
    fn try_from_visibility_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            // &self + crate visibility
            syn::parse_quote! {
                #[ink(message)]
                crate fn my_message(&self) {}
            },
            // &mut self + crate visibility
            syn::parse_quote! {
                #[ink(message)]
                crate fn my_message(&mut self) {}
            },
            // &self + pub restricted visibility
            syn::parse_quote! {
                #[ink(message)]
                pub(in my::path) fn my_message(&self) {}
            },
            // &mut self + pub restricted visibility
            syn::parse_quote! {
                #[ink(message)]
                pub(in my::path) fn my_message(&mut self) {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(
                item_method,
                "ink! messages must have public or inherited visibility",
            )
        }
    }

    #[test]
    fn conflicting_attributes_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            // storage
            syn::parse_quote! {
                #[ink(message, storage)]
                fn my_message(&self) {}
            },
            // namespace
            syn::parse_quote! {
                #[ink(message, namespace = "my_namespace")]
                fn my_message(&self) {}
            },
            // event + multiple attributes
            syn::parse_quote! {
                #[ink(message)]
                #[ink(event)]
                fn my_message(&self) {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(
                item_method,
                "encountered conflicting ink! attribute argument",
            )
        }
    }
}
