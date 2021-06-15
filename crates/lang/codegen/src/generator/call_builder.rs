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

use crate::GenerateCode;
use derive_more::From;
use heck::CamelCase;
use proc_macro2::TokenStream as TokenStream2;
use quote::{
    format_ident,
    quote,
    quote_spanned,
};
use syn::spanned::Spanned as _;

/// Generates code for the call builder of the ink! smart contract.
///
/// The call builder is the entity that builds up calls for calling of other
/// smart contract on-chain in a type safe way.
/// It implements all ink! traits that the associated ink! smart contract implements
/// so that their underlying implementation directly calls the respective ink!
/// trait implementation on-chain.
///
/// The ink! call builder of a smart contract is directly used by the storage
/// type of the smart contract itself as well by other entities that use the
/// smart contract via long-hand calling notation to incrementally build up calls.
#[derive(From)]
pub struct CallBuilder<'a> {
    contract: &'a ir::Contract,
}

impl GenerateCode for CallBuilder<'_> {
    fn generate_code(&self) -> TokenStream2 {
        let call_builder_struct = self.generate_struct();
        let trait_impl = self.generate_trait_impl();
        let auxiliary_trait_impls = self.generate_auxiliary_trait_impls();
        let call_forwarder_impls = self.generate_call_forwarder_impls();
        let contract_trait_impls = self.generate_contract_trait_impls();
        quote! {
            const _: () = {
                #call_builder_struct
                #trait_impl
                #auxiliary_trait_impls
                #call_forwarder_impls
            };
            #contract_trait_impls
        }
    }
}

impl CallBuilder<'_> {
    fn generate_struct(&self) -> TokenStream2 {
        let span = self.contract.module().storage().span();
        quote_spanned!(span=>
            /// The ink! smart contract's call builder.
            ///
            /// Implements the underlying on-chain calling of the ink! smart contract
            /// messages and trait implementations in a type safe way.
            #[repr(transparent)]
            #[cfg_attr(feature = "std", derive(
                ::core::fmt::Debug,
                ::ink_storage::traits::StorageLayout,
            ))]
            #[derive(
                ::scale::Encode,
                ::scale::Decode,
                ::ink_storage::traits::SpreadLayout,
                ::ink_storage::traits::PackedLayout,
            )]
            pub struct CallBuilder {
                account_id: AccountId,
            }
        )
    }

    /// Generates the `CallBuilder` trait implementation for the ink! contract.
    ///
    /// This creates the bridge between the ink! smart contract type and the
    /// associated call builder.
    fn generate_trait_impl(&self) -> TokenStream2 {
        let span = self.contract.module().storage().span();
        let ident = self.contract.module().storage().ident();
        quote_spanned!(span=>
            impl ::ink_lang::TraitCallBuilder for #ident {
                type Builder = CallBuilder;

                #[inline]
                fn call(&self) -> &Self::Builder {
                    &self.builder
                }

                #[inline]
                fn call_mut(&mut self) -> &mut Self::Builder {
                    &mut self.builder
                }
            }
        )
    }

    /// Generates some ink! specific auxiliary trait implementations for the
    /// smart contract call builder type.
    ///
    /// These are required to properly interoperate with the call builder.
    fn generate_auxiliary_trait_impls(&self) -> TokenStream2 {
        let span = self.contract.module().storage().span();
        quote_spanned!(span=>
            impl ::ink_env::call::FromAccountId<Environment> for CallBuilder {
                #[inline]
                fn from_account_id(account_id: AccountId) -> Self {
                    Self { account_id }
                }
            }

            impl ::ink_lang::ToAccountId<Environment> for CallBuilder {
                #[inline]
                fn to_account_id(&self) -> AccountId {
                    <AccountId as ::core::clone::Clone>::clone(&self.account_id)
                }
            }
        )
    }

    /// Generate the `TraitCallForwarder` trait implementations for the call builder
    /// for every ink! trait implemented by the associated ink! smart contract.
    ///
    /// These call forwarder trait implementations are used to dispatch to the global
    /// call builder for the respective ink! trait definition that is being called.
    /// The call builder only forwards the actual calls to those global call builders
    /// and does not have its own calling logic.
    fn generate_call_forwarder_impls(&self) -> TokenStream2 {
        self.contract
            .module()
            .impls()
            .filter_map(|impl_block| {
                // We are only interested in ink! trait implementation block.
                impl_block.trait_path().map(|trait_path| {
                    self.generate_code_for_trait_impl(trait_path, impl_block)
                })
            })
            .collect()
    }

    /// Generates code required by the ink! call builder of an ink! smart contract
    /// for a single ink! trait definition that the contract implements.
    fn generate_code_for_trait_impl(
        &self,
        trait_path: &syn::Path,
        impl_block: &ir::ItemImpl,
    ) -> TokenStream2 {
        let call_forwarder_impl =
            self.generate_call_forwarder_for_trait_impl(trait_path, impl_block);
        let implementation_marker =
            self.generate_implementation_marker_for_trait_impl(trait_path, impl_block);
        let ink_trait_impl = self.generate_ink_trait_impl(trait_path, impl_block);
        quote! {
            #call_forwarder_impl
            #implementation_marker
            #ink_trait_impl
        }
    }

    /// Generates code for a single ink! trait implementation to forward calls for
    /// the associated ink! smart contract call builder.
    fn generate_call_forwarder_for_trait_impl(
        &self,
        trait_path: &syn::Path,
        impl_block: &ir::ItemImpl,
    ) -> TokenStream2 {
        let span = impl_block.span();
        let unique_trait_id = self.generate_unique_trait_id(trait_path);
        quote_spanned!(span=>
            #[doc(hidden)]
            impl ::ink_lang::TraitCallForwarderFor<#unique_trait_id> for CallBuilder {
                type Forwarder = <<Self as Increment>::__ink_TraitInfo as ::ink_lang::TraitCallForwarder>::Forwarder;

                #[inline(always)]
                fn forward(&self) -> &Self::Forwarder {
                    // SAFETY:
                    //
                    // We convert from a shared reference to a type that thinly wraps
                    // only an `AccountId` to a shared reference to another type of which
                    // we know that it also thinly wraps an `AccountId`.
                    // Furthermore both types use `repr(transparent)`.
                    unsafe {
                        &*(&self.account_id as *const AccountId as *const Self::Forwarder)
                    }
                }

                #[inline(always)]
                fn forward_mut(&mut self) -> &mut Self::Forwarder {
                    // SAFETY:
                    //
                    // We convert from a exclusive reference to a type that thinly wraps
                    // only an `AccountId` to a exclusive reference to another type of which
                    // we know that it also thinly wraps an `AccountId`.
                    // Furthermore both types use `repr(transparent)`.
                    unsafe {
                        &mut *(&mut self.account_id as *mut AccountId as *mut Self::Forwarder)
                    }
                }

                #[inline(always)]
                fn build(
                    &self,
                ) -> &<Self::Forwarder as ::ink_lang::TraitCallBuilder>::Builder {
                    <_ as ::ink_lang::TraitCallBuilder>::call(
                        <Self as ::ink_lang::TraitCallForwarderFor<#unique_trait_id>>::forward(self)
                    )
                }

                #[inline(always)]
                fn build_mut(
                    &mut self,
                ) -> &mut <Self::Forwarder as ::ink_lang::TraitCallBuilder>::Builder
                {
                    <_ as ::ink_lang::TraitCallBuilder>::call_mut(
                        <Self as ::ink_lang::TraitCallForwarderFor<#unique_trait_id>>::forward_mut(self)
                    )
                }
            }
        )
    }

    /// Unsafely implements the required trait implementation marker.
    ///
    /// This marker only states that the ink! trait definition has been properly implemented.
    /// The marker trait is unsafe to make people think twice before manually implementing
    /// ink! trait definitions.
    fn generate_implementation_marker_for_trait_impl(
        &self,
        trait_path: &syn::Path,
        impl_block: &ir::ItemImpl,
    ) -> TokenStream2 {
        let span = impl_block.span();
        let unique_trait_id = self.generate_unique_trait_id(trait_path);
        quote_spanned!(span=>
            #[doc(hidden)]
            unsafe impl
                ::ink_lang::TraitImplementer<#unique_trait_id> for CallBuilder
            {}
        )
    }

    /// Generates code to uniquely identify a trait by its unique ID given only its identifier.
    ///
    /// # Note
    ///
    /// As with all Rust macros identifiers can shadow each other so the given identifier
    /// needs to be valid for the scope in which the returned code is generated.
    fn generate_unique_trait_id(&self, trait_path: &syn::Path) -> TokenStream2 {
        let span = self.contract.module().storage().span();
        quote_spanned!(span=>
            {
                <<::ink_lang::TraitCallForwarderRegistry<Environment>
                    as #trait_path>::__ink_TraitInfo
                    as ::ink_lang::TraitUniqueId>::ID
            }
        )
    }

    /// Generates the actual ink! trait implementation for the generated call builder.
    fn generate_ink_trait_impl(
        &self,
        trait_path: &syn::Path,
        impl_block: &ir::ItemImpl,
    ) -> TokenStream2 {
        let span = impl_block.span();
        let messages = impl_block
            .iter_messages()
            .map(|message| self.generate_ink_trait_impl_for_message(trait_path, message));
        quote_spanned!(span=>
            impl #trait_path for CallBuilder {
                type Env = Environment;

                type __ink_TraitInfo = <::ink_lang::TraitCallForwarderRegistry<Environment>
                    as #trait_path>::__ink_TraitInfo;

                #( #messages )*
            }
        )
    }

    /// Generates the code for the ink! trait implementation of the call builder
    /// of a single ink! trait message and its associated output type.
    fn generate_ink_trait_impl_for_message(
        &self,
        trait_path: &syn::Path,
        message: ir::CallableWithSelector<ir::Message>,
    ) -> TokenStream2 {
        use ir::Callable as _;
        let span = message.span();
        let message_ident = message.ident();
        let output_ident = self.output_ident(message_ident);
        let unique_trait_id = self.generate_unique_trait_id(trait_path);
        let input_bindings = message
            .callable()
            .inputs()
            .map(|input| &input.pat)
            .collect::<Vec<_>>();
        let input_types = message
            .callable()
            .inputs()
            .map(|input| &input.ty)
            .collect::<Vec<_>>();
        let mut_token = message
            .receiver()
            .is_ref_mut()
            .then(|| Some(quote! { mut }));
        let attrs = message.attrs();
        quote_spanned!(span=>
            type #output_ident = <<<
                Self
                as ::ink_lang::TraitCallForwarderFor<#unique_trait_id>>::Forwarder
                as ::ink_lang::TraitCallBuilder>::Builder
                as #trait_path>::#output_ident;

            #[inline]
            #( #attrs )*
            fn #message_ident(
                & #mut_token self
                #( , #input_bindings: #input_types )*
            ) -> Self::#output_ident {
                <_ as #trait_path>::#message_ident(
                    <Self as ::ink_lang::TraitCallForwarderFor<#unique_trait_id>>::build_mut(self)
                    #( , #input_bindings )*
                )
            }
        )
    }

    /// Returns the associated output type for an ink! trait message.
    ///
    /// TODO: Remove duplicated `output_ident` method before merging PR.
    fn output_ident(&self, message_name: &syn::Ident) -> syn::Ident {
        format_ident!("{}Output", message_name.to_string().to_camel_case())
    }

    /// Generates the code for all ink! trait implementations of the contract itself.
    ///
    /// # Note
    ///
    /// Since those implementations must live outside of an artificial `const` block
    /// we need to keep this in a separate expansion step.
    fn generate_contract_trait_impls(&self) -> TokenStream2 {
        self.contract
            .module()
            .impls()
            .filter_map(|impl_block| {
                // We are only interested in ink! trait implementation block.
                impl_block.trait_path().map(|trait_path| {
                    self.generate_contract_trait_impl(trait_path, impl_block)
                })
            })
            .collect()
    }

    /// Generates the code for a single ink! trait implementation of the contract itself.
    ///
    /// The generated implementation mainly forwards the calls to the previously generated
    /// associated call builder that implements each respective ink! trait.
    fn generate_contract_trait_impl(
        &self,
        trait_path: &syn::Path,
        impl_block: &ir::ItemImpl,
    ) -> TokenStream2 {
        let span = self.contract.module().storage().span();
        let unique_trait_id = self.generate_unique_trait_id(trait_path);
        let storage_ident = self.contract.module().storage().ident();
        let messages = self.generate_contract_trait_impl_messages(trait_path, impl_block);
        quote_spanned!(span=>
            #[doc(hidden)]
            unsafe impl
                ::ink_lang::TraitImplementer<#unique_trait_id> for #storage_ident
            {}

            impl #trait_path for #storage_ident {
                type Env = Environment;

                #[doc(hidden)]
                type __ink_TraitInfo = <::ink_lang::TraitCallForwarderRegistry<Environment>
                    as #trait_path>::__ink_TraitInfo;

                #messages
            }
        )
    }

    /// Generates the code for a all messages of a single ink! trait implementation of
    /// the ink! smart contract.
    fn generate_contract_trait_impl_messages(
        &self,
        trait_path: &syn::Path,
        impl_block: &ir::ItemImpl,
    ) -> TokenStream2 {
        impl_block
            .iter_messages()
            .map(|message| {
                self.generate_contract_trait_impl_for_message(trait_path, message)
            })
            .collect()
    }

    /// Generates the code for a single message of a single ink! trait implementation
    /// that is implemented by the ink! smart contract.
    fn generate_contract_trait_impl_for_message(
        &self,
        trait_path: &syn::Path,
        message: ir::CallableWithSelector<ir::Message>,
    ) -> TokenStream2 {
        use ir::Callable as _;
        let unique_trait_id = self.generate_unique_trait_id(trait_path);
        let span = message.span();
        let message_ident = message.ident();
        let output_ident = self.output_ident(message_ident);
        let call_operator = match message.receiver() {
            ir::Receiver::Ref => quote! { call },
            ir::Receiver::RefMut => quote! { call_mut },
        };
        let forward_operator = match message.receiver() {
            ir::Receiver::Ref => quote! { forward },
            ir::Receiver::RefMut => quote! { forward_mut },
        };
        let mut_token = message.receiver().is_ref_mut().then(|| quote! { mut });
        let input_bindings = message.inputs().map(|input| &input.pat).collect::<Vec<_>>();
        let input_types = message.inputs().map(|input| &input.ty).collect::<Vec<_>>();
        quote_spanned!(span=>
            type #output_ident =
                <<Self::__ink_TraitInfo as ::ink_lang::TraitCallForwarder>::Forwarder as #trait_path>::#output_ident;

            #[inline]
            fn #message_ident(
                & #mut_token self
                #( , #input_bindings : #input_types )*
            ) -> Self::#output_ident {
                <_ as #trait_path>::#message_ident(
                    <_ as ::ink_lang::TraitCallForwarderFor<#unique_trait_id>>::#forward_operator(
                        <Self as ::ink_lang::TraitCallBuilder>::#call_operator(self),
                    )
                    #( , #input_bindings )*
                )
            }
        )
    }
}
