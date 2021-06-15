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
use quote::{
    quote,
    quote_spanned,
};

impl<'a> TraitDefinition<'a> {
    /// Generates code for the global trait call forwarder for an ink! trait.
    ///
    /// # Note
    ///
    /// - The generated call forwarder type implements the ink! trait definition
    ///   and allows to build up contract calls that allow for customization by
    ///   the user to provide gas limit, endowment etc.
    /// - The call forwarder is associated to the call builder for the same ink!
    ///   trait definition and handles all ink! trait calls into another contract
    ///   instance on-chain. For constructing custom calls it forwards to the call
    ///   builder.
    pub fn generate_call_forwarder(&self) -> TokenStream2 {
        CallForwarder::from(*self).generate_code()
    }

    /// The identifier of the ink! trait call forwarder.
    pub fn call_forwarder_ident(&self) -> syn::Ident {
        self.append_trait_suffix(CallForwarder::SUFFIX)
    }
}

/// Generates code for the global ink! trait call forwarder.
#[derive(From)]
struct CallForwarder<'a> {
    trait_def: TraitDefinition<'a>,
}

impl GenerateCode for CallForwarder<'_> {
    fn generate_code(&self) -> TokenStream2 {
        let struct_definition = self.generate_struct_definition();
        let storage_layout_impl = self.generate_storage_layout_impl();
        let spread_layout_impl = self.generate_spread_layout_impl();
        let packed_layout_impl = self.generate_packed_layout_impl();
        let auxiliary_trait_impls = self.generate_auxiliary_trait_impls();
        let to_from_account_id_impls = self.generate_to_from_account_id_impls();
        let call_builder_impl = self.generate_call_builder_trait_impl();
        let ink_trait_impl = self.generate_ink_trait_impl();
        quote! {
            #struct_definition
            #storage_layout_impl
            #spread_layout_impl
            #packed_layout_impl
            #auxiliary_trait_impls
            #to_from_account_id_impls
            #call_builder_impl
            #ink_trait_impl
        }
    }
}

impl CallForwarder<'_> {
    /// The name suffix for ink! trait call forwarder.
    const SUFFIX: &'static str = "TraitCallForwarder";

    /// Returns the span of the ink! trait definition.
    fn span(&self) -> Span {
        self.trait_def.span()
    }

    /// Returns the identifier of the ink! trait call forwarder.
    fn ident(&self) -> syn::Ident {
        self.trait_def.call_forwarder_ident()
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
        let call_forwarder_ident = self.ident();
        quote_spanned!(span =>
            /// The global call forwarder for the ink! trait definition.
            ///
            /// All cross-contract calls to contracts implementing the associated ink! trait
            /// will be handled by this type.
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #[derive(::scale::Encode, ::scale::Decode)]
            #[repr(transparent)]
            pub struct #call_forwarder_ident<E>
            where
                E: ::ink_env::Environment,
            {
                builder: <Self as ::ink_lang::TraitCallBuilder>::Builder,
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
        let call_forwarder_ident = self.ident();
        quote_spanned!(span=>
            #[cfg(feature = "std")]
            impl<E> ::ink_storage::traits::StorageLayout
                for #call_forwarder_ident<E>
            where
                E: ::ink_env::Environment,
                <E as ::ink_env::Environment>::AccountId: ::ink_storage::traits::StorageLayout,
            {
                fn layout(
                    __key_ptr: &mut ::ink_storage::traits::KeyPtr,
                ) -> ::ink_metadata::layout::Layout {
                    <<Self as ::ink_lang::TraitCallBuilder>::Builder
                        as ::ink_storage::traits::StorageLayout>::layout(__key_ptr)
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
        let call_forwarder_ident = self.ident();
        quote_spanned!(span=>
            impl<E> ::ink_storage::traits::SpreadLayout
                for #call_forwarder_ident<E>
            where
                E: ::ink_env::Environment,
                <E as ::ink_env::Environment>::AccountId: ::ink_storage::traits::SpreadLayout,
            {
                const FOOTPRINT: ::core::primitive::u64 = 1;
                const REQUIRES_DEEP_CLEAN_UP: ::core::primitive::bool = false;

                #[inline]
                fn pull_spread(ptr: &mut ::ink_primitives::KeyPtr) -> Self {
                    Self {
                        builder: <<Self as ::ink_lang::TraitCallBuilder>::Builder
                            as ::ink_storage::traits::SpreadLayout>::pull_spread(ptr)
                    }
                }

                #[inline]
                fn push_spread(&self, ptr: &mut ::ink_primitives::KeyPtr) {
                    <<Self as ::ink_lang::TraitCallBuilder>::Builder
                        as ::ink_storage::traits::SpreadLayout>::push_spread(&self.builder, ptr)
                }

                #[inline]
                fn clear_spread(&self, ptr: &mut ::ink_primitives::KeyPtr) {
                    <<Self as ::ink_lang::TraitCallBuilder>::Builder
                        as ::ink_storage::traits::SpreadLayout>::clear_spread(&self.builder, ptr)
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
        let call_forwarder_ident = self.ident();
        quote_spanned!(span=>
            impl<E> ::ink_storage::traits::PackedLayout
                for #call_forwarder_ident<E>
            where
                E: ::ink_env::Environment,
                <E as ::ink_env::Environment>::AccountId: ::ink_storage::traits::PackedLayout,
            {
                #[inline]
                fn pull_packed(&mut self, _at: &::ink_primitives::Key) {}
                #[inline]
                fn push_packed(&self, _at: &::ink_primitives::Key) {}
                #[inline]
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
        let call_forwarder_ident = self.ident();
        let call_forwarder_name = call_forwarder_ident.to_string();
        quote_spanned!(span=>
            impl<E> ::core::clone::Clone for #call_forwarder_ident<E>
            where
                E: ::ink_env::Environment,
                <E as ::ink_env::Environment>::AccountId: ::core::clone::Clone,
            {
                #[inline]
                fn clone(&self) -> Self {
                    Self {
                        builder: <<Self as ::ink_lang::TraitCallBuilder>::Builder
                            as ::core::clone::Clone>::clone(&self.builder),
                    }
                }
            }

            impl<E> ::core::fmt::Debug for #call_forwarder_ident<E>
            where
                E: ::ink_env::Environment,
                <E as ::ink_env::Environment>::AccountId: ::core::fmt::Debug,
            {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    f.debug_struct(#call_forwarder_name)
                        .field("account_id", &self.builder.account_id)
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
        let call_forwarder_ident = self.ident();
        quote_spanned!(span=>
            impl<E> ::ink_env::call::FromAccountId<E>
                for #call_forwarder_ident<E>
            where
                E: ::ink_env::Environment,
            {
                #[inline]
                fn from_account_id(account_id: <E as ::ink_env::Environment>::AccountId) -> Self {
                    Self { builder: <<Self as ::ink_lang::TraitCallBuilder>::Builder
                        as ::ink_env::call::FromAccountId<E>>::from_account_id(account_id) }
                }
            }

            impl<E> ::ink_lang::ToAccountId<E> for #call_forwarder_ident<E>
            where
                E: ::ink_env::Environment,
            {
                #[inline]
                fn to_account_id(&self) -> <E as ::ink_env::Environment>::AccountId {
                    <<Self as ::ink_lang::TraitCallBuilder>::Builder
                        as ::ink_lang::ToAccountId<E>>::to_account_id(&self.builder)
                }
            }
        )
    }

    /// Generate the trait impl for `CallBuilder` for the ink! trait call forwarder.
    ///
    /// # Note
    ///
    /// Through the implementation of this trait it is possible to refer to the
    /// ink! trait call builder that is associated to this ink! trait call forwarder.
    fn generate_call_builder_trait_impl(&self) -> TokenStream2 {
        let span = self.trait_def.span();
        let call_forwarder_ident = self.ident();
        let call_builder_ident = self.trait_def.call_builder_ident();
        quote_spanned!(span=>
            /// This trait allows to bridge from call forwarder to call builder.
            ///
            /// Also this explains why we designed the generated code so that we have
            /// both types and why the forwarder is a thin-wrapper around the builder
            /// as this allows to perform this operation safely.
            impl<E> ::ink_lang::TraitCallBuilder for #call_forwarder_ident<E>
            where
                E: ::ink_env::Environment,
            {
                type Builder = #call_builder_ident<E>;

                #[inline]
                fn call(&self) -> &<Self as ::ink_lang::TraitCallBuilder>::Builder {
                    &self.builder
                }

                #[inline]
                fn call_mut(&mut self) -> &mut <Self as ::ink_lang::TraitCallBuilder>::Builder {
                    &mut self.builder
                }
            }
        )
    }

    /// Generates the implementation of the associated ink! trait definition.
    ///
    /// # Note
    ///
    /// The implementation mainly forwards to the associated ink! call builder
    /// of the same ink! trait definition.
    fn generate_ink_trait_impl(&self) -> TokenStream2 {
        let span = self.trait_def.span();
        let trait_ident = self.trait_def.trait_def.ident();
        let trait_uid = self.trait_def.trait_def.unique_id();
        let trait_info_ident = self.trait_def.trait_info_ident();
        let forwarder_ident = self.ident();
        let message_impls = self.generate_ink_trait_impl_messages();
        quote_spanned!(span=>
            unsafe impl<E> ::ink_lang::TraitImplementer<#trait_uid>
                for #forwarder_ident<E>
            where
                E: ::ink_env::Environment,
            {
            }

            impl<E> #trait_ident for #forwarder_ident<E>
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

    /// Generate the code for all ink! trait messages implemented by the trait call forwarder.
    fn generate_ink_trait_impl_messages(&self) -> TokenStream2 {
        let messages = self
            .trait_def
            .trait_def
            .iter_items()
            .filter_map(|(item, _)| {
                item.filter_map_message()
                    .map(|message| self.generate_ink_trait_impl_for_message(&message))
            });
        quote! {
            #( #messages )*
        }
    }

    /// Generate the code for a single ink! trait message implemented by the trait call forwarder.
    fn generate_ink_trait_impl_for_message(
        &self,
        message: &ir::InkTraitMessage,
    ) -> TokenStream2 {
        let span = message.span();
        let trait_ident = self.trait_def.trait_def.ident();
        let forwarder_ident = self.ident();
        let message_ident = message.ident();
        let attrs = message.attrs();
        let output_ident = self.trait_def.output_ident(message.ident());
        let output_type = message.output().cloned().unwrap_or_else(|| syn::parse_quote!(()));
        let input_bindings = message.inputs().map(|input| &input.pat).collect::<Vec<_>>();
        let input_types = message.inputs().map(|input| &input.ty).collect::<Vec<_>>();
        let call_op = match message.receiver() {
            ir::Receiver::Ref => quote! { call },
            ir::Receiver::RefMut => quote! { call_mut },
        };
        let mut_tok = message.mutates().then(|| quote! { mut });
        let panic_str = format!(
            "encountered error while calling <{} as {}>::{}",
            forwarder_ident, trait_ident, message_ident,
        );
        quote_spanned!(span =>
            type #output_ident = #output_type;

            #( #attrs )*
            #[inline]
            fn #message_ident(
                & #mut_tok self
                #( , #input_bindings : #input_types )*
            ) -> Self::#output_ident {
                <<Self as ::ink_lang::TraitCallBuilder>::Builder as #trait_ident>::#message_ident(
                    <Self as ::ink_lang::TraitCallBuilder>::#call_op(self)
                    #(
                        , #input_bindings: #input_types
                    ),*
                )
                    .fire()
                    .unwrap_or_else(|err| ::core::panic!("{}: {}", #panic_str, err))
            }
        )
    }
}
