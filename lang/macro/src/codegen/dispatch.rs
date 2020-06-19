// Copyright 2018-2019 Parity Technologies (UK) Ltd.
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
    codegen::{
        cross_calling::CrossCallingConflictCfg,
        GenerateCode,
        GenerateCodeUsing,
    },
    ir,
};
use derive_more::From;
use proc_macro2::{
    Ident,
    TokenStream as TokenStream2,
};
use quote::{
    quote,
    quote_spanned,
};
use syn::{
    punctuated::Punctuated,
    Token,
};

/// Generates code for the dispatch parts that dispatch constructors
/// and messages from the input and also handle the returning of data.
#[derive(From)]
pub struct Dispatch<'a> {
    /// The contract to generate code for.
    contract: &'a ir::Contract,
}

impl<'a> GenerateCodeUsing for Dispatch<'a> {
    fn contract(&self) -> &ir::Contract {
        self.contract
    }
}

impl GenerateCode for Dispatch<'_> {
    fn generate_code(&self) -> TokenStream2 {
        let message_trait_impls = self.generate_message_trait_impls();
        let message_dispatch_enum = self.generate_message_dispatch_enum();
        let constructor_dispatch_enum = self.generate_constructor_dispatch_enum();
        let message_namespaces = self.generate_message_namespaces();
        let dispatch_using_mode = self.generate_dispatch_using_mode();
        let entry_points = self.generate_entry_points();
        let cfg = self.generate_code_using::<CrossCallingConflictCfg>();

        quote! {
            // We do not generate contract dispatch code while the contract
            // is being tested or the contract is a dependency of another
            // since both resulting compilations do not require dispatching.
            #[cfg(not(test))]
            #cfg
            const _: () = {
                #message_dispatch_enum
                #constructor_dispatch_enum
                #message_namespaces
                #message_trait_impls
                #dispatch_using_mode
                #entry_points
            };
        }
    }
}

impl Dispatch<'_> {
    fn generate_dispatch_variant_ident(
        &self,
        message: &ir::Function,
        prefix: &str,
    ) -> Ident {
        let selector = message
            .selector()
            .expect("encountered a non-constructor function");
        let selector_bytes = selector.as_bytes();
        quote::format_ident!(
            "__{}_0x{:02X}{:02X}{:02X}{:02X}",
            prefix,
            selector_bytes[0],
            selector_bytes[1],
            selector_bytes[2],
            selector_bytes[3]
        )
    }

    fn generate_dispatch_variant_decode(
        &self,
        message: &ir::Function,
        prefix: &str,
    ) -> TokenStream2 {
        let selector_bytes = *message
            .selector()
            .expect("encountered a non-message function")
            .as_bytes();
        let s0 = selector_bytes[0];
        let s1 = selector_bytes[1];
        let s2 = selector_bytes[2];
        let s3 = selector_bytes[3];
        let variant_ident = self.generate_dispatch_variant_ident(message, prefix);
        let variant_types = message.sig.inputs().map(|arg| &arg.ty);
        quote! {
            [#s0, #s1, #s2, #s3] => {
                Ok(Self::#variant_ident(
                    #(
                        <#variant_types as ::scale::Decode>::decode(input)?
                    ),*
                ))
            }
        }
    }

    fn generate_dispatch_variant_arm(
        &self,
        message: &ir::Function,
        prefix: &str,
    ) -> TokenStream2 {
        let input_types = message.sig.inputs().map(|arg| &arg.ty);
        let variant_ident = self.generate_dispatch_variant_ident(message, prefix);
        quote! {
            #variant_ident(#(#input_types),*)
        }
    }

    /// Returns an iterator yielding the functions of a contract that are messages.
    fn contract_messages<'a>(&'a self) -> impl Iterator<Item = &'a ir::Function> + 'a {
        self.contract
            .functions
            .iter()
            .filter(|function| function.is_message())
    }

    fn generate_message_dispatch_enum(&self) -> TokenStream2 {
        let storage_ident = &self.contract.storage.ident;
        let message_variants = self
            .contract_messages()
            .map(|message| self.generate_dispatch_variant_arm(message, "Message"));
        let decode_message = self
            .contract_messages()
            .map(|message| self.generate_dispatch_variant_decode(message, "Message"));
        let execute_variants = self.contract_messages()
            .map(|function| {
                let ident = self.generate_dispatch_variant_ident(function, "Message");
                let arg_idents = function
                    .sig
                    .inputs()
                    .map(|arg| &arg.ident)
                    .collect::<Vec<_>>();
                let (mut_mod, msg_trait, exec_fn) = if function
                    .sig
                    .is_mut()
                    .expect("encountered non-ink! message or constructor")
                {
                    (
                        Some(quote! { mut }),
                        quote! { MessageMut },
                        quote! { execute_message_mut },
                    )
                } else {
                    (
                        None,
                        quote! { MessageRef },
                        quote! { execute_message },
                    )
                };
                let arg_inputs = if arg_idents.len() == 1 {
                    quote! { #(#arg_idents),* }
                } else {
                    quote! { ( #(#arg_idents),* ) }
                };
                let selector_id = function
                    .selector()
                    .expect("encountered non-ink! message or constructor")
                    .unique_id();
                quote! {
                    Self::#ident(#(#arg_idents),*) => {
                        ::ink_lang::#exec_fn::<Msg<[(); #selector_id]>, _>(move |state: &#mut_mod #storage_ident| {
                            <Msg<[(); #selector_id]> as ::ink_lang::#msg_trait>::CALLABLE(
                                state, #arg_inputs
                            )
                        })
                    }
                }
            });
        quote! {
            const _: () = {
                pub enum MessageDispatchEnum {
                    #( #message_variants ),*
                }

                impl ::ink_lang::MessageDispatcher for #storage_ident {
                    type Type = MessageDispatchEnum;
                }

                impl ::scale::Decode for MessageDispatchEnum {
                    fn decode<I: ::scale::Input>(input: &mut I) -> ::core::result::Result<Self, ::scale::Error> {
                        match <[u8; 4] as ::scale::Decode>::decode(input)? {
                            #(
                                #decode_message
                            )*
                            _invalid => Err(::scale::Error::from("invalid message selector"))
                        }
                    }
                }

                impl ::ink_lang::Execute for MessageDispatchEnum {
                    fn execute(self) -> ::core::result::Result<(), ::ink_lang::DispatchError> {
                        match self {
                            #(
                                #execute_variants
                            )*
                        }
                    }
                }
            };
        }
    }

    /// Returns an iterator yielding the functions of a contract that are constructors.
    fn contract_constructors<'a>(
        &'a self,
    ) -> impl Iterator<Item = &'a ir::Function> + 'a {
        self.contract
            .functions
            .iter()
            .filter(|function| function.is_constructor())
    }

    fn generate_constructor_dispatch_enum(&self) -> TokenStream2 {
        let storage_ident = &self.contract.storage.ident;
        let message_variants = self
            .contract_constructors()
            .map(|message| self.generate_dispatch_variant_arm(message, "Constructor"));
        let decode_message = self
            .contract_constructors()
            .map(|message| self.generate_dispatch_variant_decode(message, "Constructor"));
        let execute_variants = self.contract_constructors()
                .map(|function| {
                    let ident = self.generate_dispatch_variant_ident(function, "Constructor");
                    let arg_idents = function
                        .sig
                        .inputs()
                        .map(|arg| &arg.ident)
                        .collect::<Vec<_>>();
                    let arg_inputs = if arg_idents.len() == 1 {
                        quote! { #(#arg_idents),* }
                    } else {
                        quote! { ( #(#arg_idents),* ) }
                    };
                    let selector_id = function
                        .selector()
                        .expect("encountered non-ink! message or constructor")
                        .unique_id();
                    quote! {
                        Self::#ident(#(#arg_idents),*) => {
                            ::ink_lang::execute_constructor::<Constr<[(); #selector_id]>, _>(move || {
                                <Constr<[(); #selector_id]> as ::ink_lang::Constructor>::CALLABLE(
                                    #arg_inputs
                                )
                            })
                        }
                    }
                });
        quote! {
            const _: () = {
                pub enum ConstructorDispatchEnum {
                    #( #message_variants ),*
                }

                impl ::ink_lang::ConstructorDispatcher for #storage_ident {
                    type Type = ConstructorDispatchEnum;
                }

                impl ::scale::Decode for ConstructorDispatchEnum {
                    fn decode<I: ::scale::Input>(input: &mut I) -> ::core::result::Result<Self, ::scale::Error> {
                        match <[u8; 4] as ::scale::Decode>::decode(input)? {
                            #(
                                #decode_message
                            )*
                            _invalid => Err(::scale::Error::from("invalid constructor selector"))
                        }
                    }
                }

                impl ::ink_lang::Execute for ConstructorDispatchEnum {
                    fn execute(self) -> ::core::result::Result<(), ::ink_lang::DispatchError> {
                        match self {
                            #(
                                #execute_variants
                            )*
                        }
                    }
                }
            };
        }
    }

    fn generate_trait_impls_for_message(&self, function: &ir::Function) -> TokenStream2 {
        if !(function.is_constructor() || function.is_message()) {
            return quote! {}
        }
        let span = function.span();
        let selector = function
            .selector()
            .expect("this is either a message or constructor at this point; qed");
        let (selector_bytes, selector_id) = (selector.as_bytes(), selector.unique_id());
        let sig = &function.sig;
        let inputs = sig.inputs().map(|ident_type| &ident_type.ty);
        let inputs_punct = inputs.collect::<Punctuated<_, Token![,]>>();
        let output = &sig.output;
        let output_type = match output {
            syn::ReturnType::Default => quote! { () },
            syn::ReturnType::Type(_, ty) => quote! { #ty },
        };
        let is_mut = sig.is_mut().unwrap_or(true);
        let is_constructor = function.is_constructor();
        let state_ident = &self.contract.storage.ident;
        let fn_ident = &function.sig.ident;

        use syn::spanned::Spanned as _;

        let namespace = match function.kind() {
            ir::FunctionKind::Constructor(_) => quote! { Constr },
            ir::FunctionKind::Message(_) => quote! { Msg },
            ir::FunctionKind::Method => panic!("ICE: can't match a method at this point"),
        };
        let inputs = if inputs_punct.len() != 1 {
            quote! { ( #inputs_punct )}
        } else {
            quote! { #inputs_punct }
        };
        let fn_input = quote_spanned!(sig.inputs.span() =>
            impl ::ink_lang::FnInput for #namespace<[(); #selector_id]> {
                type Input = #inputs;
            }
        );
        let fn_output2 = if !is_constructor {
            quote_spanned!(sig.output.span() =>
                impl ::ink_lang::FnOutput for #namespace<[(); #selector_id]> {
                    #[allow(unused_parens)]
                    type Output = #output_type;
                }
            )
        } else {
            quote! {}
        };
        let fn_selector = quote_spanned!(span =>
            impl ::ink_lang::FnSelector for #namespace<[(); #selector_id]> {
                const SELECTOR: ::ink_core::env::call::Selector = ::ink_core::env::call::Selector::new([
                    #( #selector_bytes ),*
                ]);
            }
        );
        let fn_state = quote_spanned!(span =>
            impl ::ink_lang::FnState for #namespace<[(); #selector_id]> {
                type State = #state_ident;
            }
        );
        let input_idents = sig
            .inputs()
            .map(|ident_type| &ident_type.ident)
            .collect::<Punctuated<_, Token![,]>>();
        let input_params = if input_idents.len() >= 2 {
            quote! { (#input_idents) }
        } else if input_idents.len() == 1 {
            quote! { #input_idents }
        } else {
            quote! { _ }
        };
        let input_forward = quote! { #input_idents };
        let message2_impl = if is_constructor {
            quote_spanned!(span =>
                impl ::ink_lang::Constructor for #namespace<[(); #selector_id]> {
                    const CALLABLE: fn(
                        <Self as ::ink_lang::FnInput>::Input
                    ) -> <Self as ::ink_lang::FnState>::State = |#input_params| #state_ident::#fn_ident(#input_forward);
                }
            )
        } else if is_mut {
            quote_spanned!(span =>
                impl ::ink_lang::MessageMut for #namespace<[(); #selector_id]> {
                    const CALLABLE: fn(
                        &mut <Self as ::ink_lang::FnState>::State,
                        <Self as ::ink_lang::FnInput>::Input
                    ) -> <Self as ::ink_lang::FnOutput>::Output = |state, #input_params| #state_ident::#fn_ident(state, #input_forward);
                }
            )
        } else {
            quote_spanned!(span =>
                impl ::ink_lang::MessageRef for #namespace<[(); #selector_id]> {
                    const CALLABLE: fn(
                        &<Self as ::ink_lang::FnState>::State,
                        <Self as ::ink_lang::FnInput>::Input
                    ) -> <Self as ::ink_lang::FnOutput>::Output = |state, #input_params| #state_ident::#fn_ident(state, #input_forward);
                }
            )
        };

        quote_spanned!(span =>
            #fn_input
            #fn_output2
            #fn_selector
            #fn_state
            #message2_impl
        )
    }

    fn generate_message_trait_impls(&self) -> TokenStream2 {
        let fns = self
            .contract
            .functions
            .iter()
            .map(|fun| self.generate_trait_impls_for_message(fun));
        quote! {
            #( #fns )*
        }
    }

    fn generate_message_namespaces(&self) -> TokenStream2 {
        quote! {
            // Namespace for messages.
            //
            // # Note
            //
            // The `S` parameter is going to refer to array types `[(); N]`
            // where `N` is the unique identifier of the associated message
            // selector.
            pub struct Msg<S> {
                // We need to wrap inner because of Rust's orphan rules.
                marker: core::marker::PhantomData<fn() -> S>,
            }

            // Namespace for constructors.
            //
            // # Note
            //
            // The `S` parameter is going to refer to array types `[(); N]`
            // where `N` is the unique identifier of the associated constructor
            // selector.
            pub struct Constr<S> {
                // We need to wrap inner because of Rust's orphan rules.
                marker: core::marker::PhantomData<fn() -> S>,
            }
        }
    }

    fn generate_dispatch_using_mode(&self) -> TokenStream2 {
        let storage_ident = &self.contract.storage.ident;
        quote! {
            impl ::ink_lang::DispatchUsingMode for #storage_ident {
                #[allow(unused_parens)]
                fn dispatch_using_mode(
                    mode: ::ink_lang::DispatchMode
                ) -> core::result::Result<(), ::ink_lang::DispatchError> {
                    match mode {
                        ::ink_lang::DispatchMode::Instantiate => {
                            <<#storage_ident as ::ink_lang::ConstructorDispatcher>::Type as ::ink_lang::Execute>::execute(
                                ::ink_core::env::decode_input::<<#storage_ident as ::ink_lang::ConstructorDispatcher>::Type>()
                                    .map_err(|_| ::ink_lang::DispatchError::CouldNotReadInput)?
                            )
                        }
                        ::ink_lang::DispatchMode::Call => {
                            <<#storage_ident as ::ink_lang::MessageDispatcher>::Type as ::ink_lang::Execute>::execute(
                                ::ink_core::env::decode_input::<<#storage_ident as ::ink_lang::MessageDispatcher>::Type>()
                                    .map_err(|_| ::ink_lang::DispatchError::CouldNotReadInput)?
                            )
                        }
                    }
                }
            }
        }
    }

    fn generate_entry_points(&self) -> TokenStream2 {
        let storage_ident = &self.contract.storage.ident;
        quote! {
            #[cfg(not(test))]
            #[no_mangle]
            fn deploy() -> u32 {
                ::ink_lang::DispatchRetCode::from(
                    <#storage_ident as ::ink_lang::DispatchUsingMode>::dispatch_using_mode(
                        ::ink_lang::DispatchMode::Instantiate,
                    ),
                )
                .to_u32()
            }

            #[cfg(not(test))]
            #[no_mangle]
            fn call() -> u32 {
                ::ink_lang::DispatchRetCode::from(
                    <#storage_ident as ::ink_lang::DispatchUsingMode>::dispatch_using_mode(
                        ::ink_lang::DispatchMode::Call,
                    ),
                )
                .to_u32()
            }
        }
    }
}
