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

use super::TraitDefinition;
use crate::traits::GenerateCode;
use derive_more::From;
use proc_macro2::{
    Span,
    TokenStream as TokenStream2,
};
use quote::{format_ident, quote, quote_spanned};

impl<'a> TraitDefinition<'a> {
    /// Generates code for the global trait call builder for an ink! trait.
    ///
    /// # Note
    ///
    /// - The generated call builder type implements the ink! trait definition
    ///   and allows to build up contract calls that allow for customization by
    ///   the user to provide gas limit, endowment etc.
    /// - The call builder is used directly by the generated call forwarder.
    ///   There exists one global call forwarder and call builder pair for every
    ///   ink! trait definition.
    pub fn generate_call_builder(&self) -> TokenStream2 {
        CallBuilder::from(*self).generate_code()
    }

    /// The identifier of the ink! trait call builder.
    pub fn call_builder_ident(&self) -> syn::Ident {
        self.append_trait_suffix(CallBuilder::SUFFIX)
    }
}

/// Generates code for the global ink! trait call builder.
#[derive(From)]
struct CallBuilder<'a> {
    trait_def: TraitDefinition<'a>,
}

impl GenerateCode for CallBuilder<'_> {
    fn generate_code(&self) -> TokenStream2 {
        let struct_definition = self.generate_struct_definition();
        let storage_layout_impl = self.generate_storage_layout_impl();
        let spread_layout_impl = self.generate_spread_layout_impl();
        let packed_layout_impl = self.generate_packed_layout_impl();
        let auxiliary_trait_impls = self.generate_auxiliary_trait_impls();
        let to_from_account_id_impls = self.generate_to_from_account_id_impls();
        let ink_trait_impl = self.generate_ink_trait_impl();
        quote! {
            #struct_definition
            #storage_layout_impl
            #spread_layout_impl
            #packed_layout_impl
            #auxiliary_trait_impls
            #to_from_account_id_impls
            #ink_trait_impl
        }
    }
}

impl CallBuilder<'_> {
    /// The name suffix for ink! trait call builder.
    const SUFFIX: &'static str = "TraitCallBuilder";

    /// Returns the span of the ink! trait definition.
    fn span(&self) -> Span {
        self.trait_def.span()
    }

    /// Returns the identifier of the ink! trait call builder.
    fn ident(&self) -> syn::Ident {
        self.trait_def.call_builder_ident()
    }

    /// Generates the struct type definition for the account wrapper type.
    ///
    /// This type is going to implement the trait so that invoking its trait
    /// methods will perform contract calls via SEAL's contract execution
    /// abstraction.
    ///
    /// # Note
    ///
    /// Unlike the layout specific traits it is possible to derive the SCALE
    /// `Encode` and `Decode` traits since they generate trait bounds per field
    /// instead of per generic parameter which is exactly what we need here.
    /// However, it should be noted that this is not Rust default behaviour.
    fn generate_struct_definition(&self) -> TokenStream2 {
        let span = self.span();
        let call_builder_ident = self.ident();
        quote_spanned!(span =>
            /// The global call builder type for all trait implementers.
            ///
            /// All calls to types (contracts) implementing the trait will be built by this type.
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #[derive(::scale::Encode, ::scale::Decode)]
            #[repr(transparent)]
            pub struct #call_builder_ident<E>
            where
                E: ::ink_env::Environment,
            {
                account_id: <E as ::ink_env::Environment>::AccountId,
            }
        )
    }

    /// Generates the `StorageLayout` trait implementation for the account wrapper.
    ///
    /// # Note
    ///
    /// Due to the generic parameter `E` and Rust's default rules for derive generated
    /// trait bounds it is not recommended to derive the `StorageLayout` trait implementation.
    fn generate_storage_layout_impl(&self) -> TokenStream2 {
        let span = self.span();
        let call_builder_ident = self.ident();
        quote_spanned!(span=>
            #[cfg(feature = "std")]
            impl<E> ::ink_storage::traits::StorageLayout
                for #call_builder_ident<E>
            where
                E: ::ink_env::Environment,
                <E as ::ink_env::Environment>::AccountId: ::ink_storage::traits::StorageLayout,
            {
                fn layout(
                    __key_ptr: &mut ::ink_storage::traits::KeyPtr,
                ) -> ::ink_metadata::layout::Layout {
                    ::ink_metadata::layout::Layout::Struct(
                        ::ink_metadata::layout::StructLayout::new(
                            <[_]>::into_vec(::std::vec![
                                ::ink_metadata::layout::FieldLayout::new(
                                    Some("account_id"),
                                    <<E as ::ink_env::Environment>::AccountId
                                        as ::ink_storage::traits::StorageLayout>::layout(__key_ptr)
                                )
                            ].into_boxed_slice())
                        )
                    )
                }
            }
        )
    }

    /// Generates the `SpreadLayout` trait implementation for the account wrapper.
    ///
    /// # Note
    ///
    /// Due to the generic parameter `E` and Rust's default rules for derive generated
    /// trait bounds it is not recommended to derive the `SpreadLayout` trait implementation.
    fn generate_spread_layout_impl(&self) -> TokenStream2 {
        let span = self.span();
        let call_builder_ident = self.ident();
        quote_spanned!(span=>
            /// We require this manual impl since the derive produces incorrect trait bounds.
            impl<E> ::ink_storage::traits::SpreadLayout
                for #call_builder_ident<E>
            where
                E: ::ink_env::Environment,
                <E as ::ink_env::Environment>::AccountId: ::ink_storage::traits::SpreadLayout,
            {
                const FOOTPRINT: ::core::primitive::u64 = 1;
                const REQUIRES_DEEP_CLEAN_UP: ::core::primitive::bool = false;

                #[inline]
                fn pull_spread(ptr: &mut ::ink_primitives::KeyPtr) -> Self {
                    Self {
                        account_id: <<E as ::ink_env::Environment>::AccountId
                            as ::ink_storage::traits::SpreadLayout>::pull_spread(ptr)
                    }
                }

                #[inline]
                fn push_spread(&self, ptr: &mut ::ink_primitives::KeyPtr) {
                    <<E as ::ink_env::Environment>::AccountId
                        as ::ink_storage::traits::SpreadLayout>::push_spread(&self.account_id, ptr)
                }

                #[inline]
                fn clear_spread(&self, ptr: &mut ::ink_primitives::KeyPtr) {
                    <<E as ::ink_env::Environment>::AccountId
                        as ::ink_storage::traits::SpreadLayout>::clear_spread(&self.account_id, ptr)
                }
            }
        )
    }

    /// Generates the `PackedLayout` trait implementation for the account wrapper.
    ///
    /// # Note
    ///
    /// Due to the generic parameter `E` and Rust's default rules for derive generated
    /// trait bounds it is not recommended to derive the `PackedLayout` trait implementation.
    fn generate_packed_layout_impl(&self) -> TokenStream2 {
        let span = self.span();
        let call_builder_ident = self.ident();
        quote_spanned!(span=>
            /// We require this manual impl since the derive produces incorrect trait bounds.
            impl<E> ::ink_storage::traits::PackedLayout
                for #call_builder_ident<E>
            where
                E: ::ink_env::Environment,
                <E as ::ink_env::Environment>::AccountId: ::ink_storage::traits::PackedLayout,
            {
                #[inline(always)]
                fn pull_packed(&mut self, _at: &::ink_primitives::Key) {}
                #[inline(always)]
                fn push_packed(&self, _at: &::ink_primitives::Key) {}
                #[inline(always)]
                fn clear_packed(&self, _at: &::ink_primitives::Key) {}
            }
        )
    }

    /// Generates trait implementations for auxiliary traits for the account wrapper.
    ///
    /// # Note
    ///
    /// Auxiliary traits currently include:
    ///
    /// - `Clone`: To allow cloning contract references in the long run.
    /// - `Debug`: To better debug internal contract state.
    fn generate_auxiliary_trait_impls(&self) -> TokenStream2 {
        let span = self.span();
        let call_builder_ident = self.ident();
        let call_builder_name = call_builder_ident.to_string();
        quote_spanned!(span=>
            /// We require this manual impl since the derive produces incorrect trait bounds.
            impl<E> ::core::clone::Clone for #call_builder_ident<E>
            where
                E: ::ink_env::Environment,
                <E as ::ink_env::Environment>::AccountId: ::core::clone::Clone,
            {
                #[inline]
                fn clone(&self) -> Self {
                    Self {
                        account_id: ::core::clone::Clone::clone(&self.account_id),
                    }
                }
            }

            /// We require this manual impl since the derive produces incorrect trait bounds.
            impl<E> ::core::fmt::Debug for #call_builder_ident<E>
            where
                E: ::ink_env::Environment,
                <E as ::ink_env::Environment>::AccountId: ::core::fmt::Debug,
            {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    f.debug_struct(#call_builder_name)
                        .field("account_id", &self.account_id)
                        .finish()
                }
            }
        )
    }

    /// Generate trait impls for `FromAccountId` and `ToAccountId` for the account wrapper.
    ///
    /// # Note
    ///
    /// This allows user code to conveniently transform from and to `AccountId` when
    /// interacting with typed contracts.
    fn generate_to_from_account_id_impls(&self) -> TokenStream2 {
        let span = self.span();
        let call_builder_ident = self.ident();
        quote_spanned!(span=>
            impl<E> ::ink_env::call::FromAccountId<E>
                for #call_builder_ident<E>
            where
                E: ::ink_env::Environment,
            {
                #[inline]
                fn from_account_id(account_id: <E as ::ink_env::Environment>::AccountId) -> Self {
                    Self { account_id }
                }
            }

            impl<E> ::ink_lang::ToAccountId<E> for #call_builder_ident<E>
            where
                E: ::ink_env::Environment,
            {
                #[inline]
                fn to_account_id(&self) -> <E as ::ink_env::Environment>::AccountId {
                    self.account_id.clone()
                }
            }
        )
    }

    /// Generates the implementation of the associated ink! trait definition.
    ///
    /// # Note
    ///
    /// The implemented messages call the SEAL host runtime in order to dispatch
    /// the respective ink! trait message calls of the called smart contract
    /// instance.
    /// The way these messages are built-up allows the caller to customize message
    /// parameters such as gas limit and transferred balance.
    fn generate_ink_trait_impl(&self) -> TokenStream2 {
        let span = self.trait_def.span();
        let trait_ident = self.trait_def.trait_def.ident();
        let trait_uid = self.trait_def.trait_def.unique_id();
        let trait_info_ident = self.trait_def.trait_info_ident();
        let builder_ident = self.ident();
        let message_impls = self.generate_ink_trait_impl_messages();
        quote_spanned!(span=>
            unsafe impl<E> ::ink_lang::TraitImplementer<#trait_uid>
                for #builder_ident<E>
            where
                E: ::ink_env::Environment,
            {
            }

            impl<E> #trait_ident for #builder_ident<E>
            where
                E: ::ink_env::Environment,
            {
                type Env = E;

                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                type __ink_TraitInfo = #trait_info_ident<E>;

                #message_impls
            }
        )
    }

    /// Generate the code for all ink! trait messages implemented by the trait call builder.
    fn generate_ink_trait_impl_messages(&self) -> TokenStream2 {
        let messages = self
            .trait_def
            .trait_def
            .iter_items()
            .filter_map(|(item, selector)| {
                item.filter_map_message()
                    .map(|message| self.generate_ink_trait_impl_for_message(&message, selector))
            });
        quote! {
            #( #messages )*
        }
    }

    /// Generate the code for a single ink! trait message implemented by the trait call builder.
    fn generate_ink_trait_impl_for_message(
        &self,
        message: &ir::InkTraitMessage,
        selector: ir::Selector,
    ) -> TokenStream2 {
        let span = message.span();
        let message_ident = message.ident();
        let attrs = message.attrs();
        let output_ident = self.trait_def.output_ident(message.ident());
        let output = message.output();
        let output_sig = output.map_or_else(
            || quote! { () },
            |output| quote! { ::ink_env::call::utils::ReturnType<#output> },
        );
        let selector_bytes = selector.as_bytes();
        let input_bindings = Self::input_bindings(message.inputs());
        let input_types = Self::input_types(message.inputs());
        let arg_list = Self::generate_argument_list_for(input_types.iter().cloned());
        let mut_tok = message.mutates().then(|| quote! { mut });
        quote_spanned!(span =>
            #[allow(clippy::type_complexity)]
            type #output_ident = ::ink_env::call::CallBuilder<
                Self::Env,
                ::ink_env::call::utils::Set< <Self::Env as ::ink_env::Environment>::AccountId >,
                ::ink_env::call::utils::Unset< ::core::primitive::u64 >,
                ::ink_env::call::utils::Unset< <Self::Env as ::ink_env::Environment>::Balance >,
                ::ink_env::call::utils::Set< ::ink_env::call::ExecutionInput<#arg_list> >,
                ::ink_env::call::utils::Set<#output_sig>,
            >;

            #( #attrs )*
            #[inline]
            fn #message_ident(
                & #mut_tok self
                #( , #input_bindings : #input_types )*
            ) -> Self::#output_ident {
                ::ink_env::call::build_call::<Self::Env>()
                    .callee(::ink_lang::ToAccountId::to_account_id(self.contract))
                    .exec_input(
                        ::ink_env::call::ExecutionInput::new(
                            ::ink_env::call::Selector::new([ #( #selector_bytes ),* ])
                        )
                        #(
                            .push_arg(#input_bindings)
                        )*
                    )
                    .returns::<#output_sig>()
            }
        )
    }

    /// Returns the sequence of artificial input parameter bindings for the message.
    ///
    /// # Note
    ///
    /// This returns `__ink_binding_N` for every message input where `N` is the number
    /// of the input from first to last.
    fn input_bindings(inputs: ir::TraitItemInputsIter) -> Vec<syn::Ident> {
        inputs
            .enumerate()
            .map(|(n, _)| format_ident!("__ink_binding_{}", n))
            .collect::<Vec<_>>()
    }

    /// Returns the sequence of input types for the message.
    fn input_types(inputs: ir::TraitItemInputsIter) -> Vec<&syn::Type> {
        inputs.map(|pat_type| &*pat_type.ty).collect::<Vec<_>>()
    }

    /// Builds up the `ink_env::call::utils::ArgumentList` type structure for the given types.
    fn generate_argument_list_for<'b, Args>(args: Args) -> TokenStream2
    where
        Args: IntoIterator<Item = &'b syn::Type>,
        <Args as IntoIterator>::IntoIter: DoubleEndedIterator,
    {
        use syn::spanned::Spanned as _;
        args.into_iter().fold(
            quote! { ::ink_env::call::utils::EmptyArgumentList },
            |rest, arg| {
                let span = arg.span();
                quote_spanned!(span=>
                    ::ink_env::call::utils::ArgumentList<::ink_env::call::utils::Argument<#arg>, #rest>
                )
            }
        )
    }
}
