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

/// An ink! constructor definition.
///
/// # Example
///
/// ## Inherent implementation constructor:
///
/// ```
/// # use core::convert::TryFrom;
/// # let event = <ink_lang_ir::ItemImpl as TryFrom<syn::ItemImpl>>::try_from(syn::parse_quote! {
/// impl MyStorage {
///     #[ink(constructor)]
///     pub fn new(init_value: i32) -> Self {
///         /* contract initialization goes here */
/// #       unimplemented!()
///     }
/// }
/// # }).unwrap();
/// ```
///
/// ## Trait implementation constructor:
///
/// ```
/// # use core::convert::TryFrom;
/// # <ink_lang_ir::ItemImpl as TryFrom<syn::ItemImpl>>::try_from(syn::parse_quote! {
/// impl MyTrait for MyStorage {
///     #[ink(constructor)]
///     fn new(init_value: i32) -> Self {
///         /* contract initialization goes here */
/// #       unimplemented!()
///     }
/// }
/// # }).unwrap();
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Constructor {
    /// The underlying Rust method item.
    pub(super) item: syn::ImplItemMethod,
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

impl quote::ToTokens for Constructor {
    /// We mainly implement this trait for this ink! type to have a derived
    /// [`Spanned`](`syn::spanned::Spanned`) implementation for it.
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.item.to_tokens(tokens)
    }
}

impl Constructor {
    /// Returns `true` if the given type is `Self`.
    fn type_is_self_val(ty: &syn::Type) -> bool {
        matches!(ty, syn::Type::Path(syn::TypePath {
            qself: None,
            path
        }) if path.is_ident("Self"))
    }

    /// Ensures that the return type of the ink! constructor is `Self`.
    ///
    /// Returns an appropriate error otherwise.
    ///
    /// # Errors
    ///
    /// If the ink! constructor does not return `Self` or is missing a return
    /// type entirely.
    fn ensure_valid_return_type(
        method_item: &syn::ImplItemMethod,
    ) -> Result<(), syn::Error> {
        match &method_item.sig.output {
            syn::ReturnType::Default => {
                return Err(format_err_spanned!(
                    &method_item.sig,
                    "missing return for ink! constructor",
                ))
            }
            syn::ReturnType::Type(_, return_type) => {
                if !Self::type_is_self_val(return_type.as_ref()) {
                    return Err(format_err_spanned!(
                        return_type,
                        "ink! constructors must return Self",
                    ))
                }
            }
        }
        Ok(())
    }

    /// Ensures that the ink! constructor has no `self` receiver.
    ///
    /// Returns an appropriate error otherwise.
    ///
    /// # Errors
    ///
    /// If the ink! constructor has a `&self`, `&mut self`, `self` or any other
    /// kind of a `self` receiver as first argument.
    fn ensure_no_self_receiver(
        method_item: &syn::ImplItemMethod,
    ) -> Result<(), syn::Error> {
        match method_item.sig.inputs.iter().next() {
            None | Some(syn::FnArg::Typed(_)) => (),
            Some(syn::FnArg::Receiver(receiver)) => {
                return Err(format_err_spanned!(
                    receiver,
                    "ink! constructors must have no `self` receiver",
                ))
            }
        }
        Ok(())
    }

    /// Sanitizes the attributes for the ink! constructor.
    ///
    /// Returns a tuple of ink! attributes and non-ink! attributes.
    fn sanitize_attributes(
        method_item: &syn::ImplItemMethod,
    ) -> Result<(ir::InkAttribute, Vec<syn::Attribute>), syn::Error> {
        ir::sanitize_attributes(
            method_item.span(),
            method_item.attrs.clone(),
            &ir::AttributeArgKind::Constructor,
            |arg| {
                match arg.kind() {
                    ir::AttributeArg::Constructor
                    | ir::AttributeArg::MetadataName(_)
                    | ir::AttributeArg::Selector(_) => Ok(()),
                    ir::AttributeArg::Payable => {
                        Err(Some(format_err!(
                            arg.span(),
                            "constructors are implicitly payable"
                        )))
                    }
                    _ => Err(None),
                }
            },
        )
    }
}

impl TryFrom<syn::ImplItemMethod> for Constructor {
    type Error = syn::Error;

    fn try_from(method_item: syn::ImplItemMethod) -> Result<Self, Self::Error> {
        ensure_callable_invariants(&method_item, CallableKind::Constructor)?;
        Self::ensure_valid_return_type(&method_item)?;
        Self::ensure_no_self_receiver(&method_item)?;
        let (ink_attrs, other_attrs) = Self::sanitize_attributes(&method_item)?;
        let selector = ink_attrs.selector();
        let metadata_name = ink_attrs.metadata_name();
        Ok(Self {
            selector,
            metadata_name,
            item: syn::ImplItemMethod {
                attrs: other_attrs,
                ..method_item
            },
        })
    }
}

impl Callable for Constructor {
    fn kind(&self) -> CallableKind {
        CallableKind::Constructor
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
        true
    }

    fn visibility(&self) -> Visibility {
        match &self.item.vis {
            syn::Visibility::Public(vis_public) => Visibility::Public(vis_public.clone()),
            syn::Visibility::Inherited => Visibility::Inherited,
            _ => unreachable!("encountered invalid visibility for ink! constructor"),
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

impl Constructor {
    /// Returns a slice of all non-ink! attributes of the ink! constructor.
    pub fn attrs(&self) -> &[syn::Attribute] {
        &self.item.attrs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                    #[ink(constructor)]
                    fn my_constructor() -> Self {}
                },
            ),
            (
                // Single input:
                expected_inputs!(a: i32),
                syn::parse_quote! {
                    #[ink(constructor)]
                    fn my_constructor(a: i32) -> Self {}
                },
            ),
            (
                // Some inputs:
                expected_inputs!(a: i32, b: u64, c: [u8; 32]),
                syn::parse_quote! {
                    #[ink(constructor)]
                    fn my_constructor(a: i32, b: u64, c: [u8; 32]) -> Self {}
                },
            ),
        ];
        for (expected_inputs, item_method) in test_inputs {
            let actual_inputs = <ir::Constructor as TryFrom<_>>::try_from(item_method)
                .unwrap()
                .inputs()
                .cloned()
                .map(|pat_type| syn::FnArg::Typed(pat_type))
                .collect::<Vec<_>>();
            assert_eq!(actual_inputs, expected_inputs);
        }
    }

    #[test]
    fn visibility_works() {
        let test_inputs: Vec<(bool, syn::ImplItemMethod)> = vec![
            // inherited
            (
                false,
                syn::parse_quote! {
                    #[ink(constructor)]
                    fn my_constructor() -> Self {}
                },
            ),
            // public
            (
                true,
                syn::parse_quote! {
                    #[ink(constructor)]
                    pub fn my_constructor() -> Self {}
                },
            ),
        ];
        for (is_pub, item_method) in test_inputs {
            let visibility = <ir::Constructor as TryFrom<_>>::try_from(item_method)
                .unwrap()
                .visibility();
            assert_eq!(visibility.is_pub(), is_pub);
            assert_eq!(visibility.is_inherited(), !is_pub);
        }
    }

    #[test]
    fn try_from_works() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            // simple + inherited visibility
            syn::parse_quote! {
                #[ink(constructor)]
                fn my_constructor() -> Self {}
            },
            // simple + public visibility
            syn::parse_quote! {
                #[ink(constructor)]
                pub fn my_constructor() -> Self {}
            },
            // many inputs
            syn::parse_quote! {
                #[ink(constructor)]
                fn my_constructor(input1: i32, input2: i64, input3: u32, input4: u64) -> Self {}
            },
        ];
        for item_method in item_methods {
            assert!(<ir::Constructor as TryFrom<_>>::try_from(item_method).is_ok());
        }
    }

    fn assert_try_from_fails(item_method: syn::ImplItemMethod, expected_err: &str) {
        assert_eq!(
            <ir::Constructor as TryFrom<_>>::try_from(item_method)
                .map_err(|err| err.to_string()),
            Err(expected_err.to_string()),
        );
    }

    #[test]
    fn try_from_missing_return_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            syn::parse_quote! {
                #[ink(constructor)]
                fn my_constructor() {}
            },
            syn::parse_quote! {
                #[ink(constructor)]
                pub fn my_constructor() {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(item_method, "missing return for ink! constructor")
        }
    }

    #[test]
    fn try_from_invalid_return_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            syn::parse_quote! {
                #[ink(constructor)]
                fn my_constructor() -> &Self {}
            },
            syn::parse_quote! {
                #[ink(constructor)]
                pub fn my_constructor() -> &mut Self {}
            },
            syn::parse_quote! {
                #[ink(constructor)]
                pub fn my_constructor() -> i32 {}
            },
            syn::parse_quote! {
                #[ink(constructor)]
                pub fn my_constructor() -> Result<Self, ()> {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(item_method, "ink! constructors must return Self")
        }
    }

    #[test]
    fn try_from_invalid_self_receiver_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            syn::parse_quote! {
                #[ink(constructor)]
                fn my_constructor(&self) -> Self {}
            },
            syn::parse_quote! {
                #[ink(constructor)]
                pub fn my_constructor(&mut self) -> Self {}
            },
            syn::parse_quote! {
                #[ink(constructor)]
                pub fn my_constructor(self) -> Self {}
            },
            syn::parse_quote! {
                #[ink(constructor)]
                pub fn my_constructor(mut self) -> Self {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(
                item_method,
                "ink! constructors must have no `self` receiver",
            )
        }
    }

    #[test]
    fn try_from_generics_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            syn::parse_quote! {
                #[ink(constructor)]
                fn my_constructor<T>() -> Self {}
            },
            syn::parse_quote! {
                #[ink(constructor)]
                pub fn my_constructor<T>() -> Self {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(item_method, "ink! constructors must not be generic")
        }
    }

    #[test]
    fn try_from_const_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            syn::parse_quote! {
                #[ink(constructor)]
                const fn my_constructor() -> Self {}
            },
            syn::parse_quote! {
                #[ink(constructor)]
                pub const fn my_constructor() -> Self {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(item_method, "ink! constructors must not be const")
        }
    }

    #[test]
    fn try_from_async_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            syn::parse_quote! {
                #[ink(constructor)]
                async fn my_constructor() -> Self {}
            },
            syn::parse_quote! {
                #[ink(constructor)]
                async fn my_constructor() -> Self {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(item_method, "ink! constructors must not be async")
        }
    }

    #[test]
    fn try_from_unsafe_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            syn::parse_quote! {
                #[ink(constructor)]
                unsafe fn my_constructor() -> Self {}
            },
            syn::parse_quote! {
                #[ink(constructor)]
                unsafe fn my_constructor() -> Self {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(item_method, "ink! constructors must not be unsafe")
        }
    }

    #[test]
    fn try_from_explicit_abi_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            syn::parse_quote! {
                #[ink(constructor)]
                extern "C" fn my_constructor() -> Self {}
            },
            syn::parse_quote! {
                #[ink(constructor)]
                extern "C" fn my_constructor() -> Self {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(item_method, "ink! constructors must have explicit ABI")
        }
    }

    #[test]
    fn try_from_variadic_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            syn::parse_quote! {
                #[ink(constructor)]
                fn my_constructor(...) -> Self {}
            },
            syn::parse_quote! {
                #[ink(constructor)]
                fn my_constructor(...) -> Self {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(item_method, "ink! constructors must not be variadic")
        }
    }

    #[test]
    fn try_from_visibility_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            syn::parse_quote! {
                #[ink(constructor)]
                crate fn my_constructor() -> Self {}
            },
            syn::parse_quote! {
                #[ink(constructor)]
                pub(in my::path) fn my_constructor() -> Self {}
            },
        ];
        for item_method in item_methods {
            assert_try_from_fails(
                item_method,
                "ink! constructors must have public or inherited visibility",
            )
        }
    }

    #[test]
    fn conflicting_attributes_fails() {
        let item_methods: Vec<syn::ImplItemMethod> = vec![
            // storage
            syn::parse_quote! {
                #[ink(constructor, storage)]
                fn my_constructor() -> Self {}
            },
            // namespace
            syn::parse_quote! {
                #[ink(constructor, namespace = "my_namespace")]
                fn my_constructor() -> Self {}
            },
            // event + multiple attributes
            syn::parse_quote! {
                #[ink(constructor)]
                #[ink(event)]
                fn my_constructor() -> Self {}
            },
            // constructor + payable
            syn::parse_quote! {
                #[ink(constructor)]
                #[ink(payable)]
                fn my_constructor() -> Self {}
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
