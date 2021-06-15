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

use super::enforced_error::EnforcedErrors;
use crate::{
    generator,
    GenerateCode,
    GenerateCodeUsing as _,
};
use derive_more::From;
use heck::CamelCase as _;
use impl_serde::serialize as serde_hex;
use ir::Callable;
use itertools::Itertools as _;
use proc_macro2::{
    Ident,
    TokenStream as TokenStream2,
};
use quote::{
    format_ident,
    quote,
    quote_spanned,
};
use syn::spanned::Spanned as _;

/// Generates code for using this ink! contract as a dependency.
#[derive(From)]
pub struct CrossCalling<'a> {
    contract: &'a ir::Contract,
}
impl_as_ref_for_generator!(CrossCalling);

impl GenerateCode for CrossCalling<'_> {
    fn generate_code(&self) -> TokenStream2 {
        let storage = self.generate_storage();
        let standard_impls = self.generate_standard_impls();
        let call_forwarder = self.generate_call_forwarders();
        let impl_blocks = self.generate_impl_blocks();
        quote! {
            #storage
            #standard_impls
            #call_forwarder
            #impl_blocks
        }
    }
}

impl CrossCalling<'_> {
    /// Generates code for the ink! storage struct for cross-calling purposes.
    ///
    /// # Note
    ///
    /// This always consists of a single `AccountId` and can be viewed as a
    /// reference to a live smart contract instance of the same type. It will
    /// forward all calls via ink!'s provided cross-calling infrastructure
    /// automatically over the chain.
    fn generate_storage(&self) -> TokenStream2 {
        let only_as_dependency =
            self.generate_code_using::<generator::OnlyAsDependencyCfg>();
        let storage = self.contract.module().storage();
        let span = storage.span();
        let ident = storage.ident();
        let attrs = storage.attrs();
        quote_spanned!(span =>
            #only_as_dependency
            #( #attrs )*
            #[derive(
                Clone,
                Debug,
                ::scale::Encode,
                ::scale::Decode,
                ::ink_storage::traits::SpreadLayout,
                ::ink_storage::traits::PackedLayout,
            )]
            #[cfg_attr(
                feature = "std",
                derive(
                    ::scale_info::TypeInfo,
                    ::ink_storage::traits::StorageLayout,
                )
            )]
            pub struct #ident {
                account_id: AccountId,
            }
        )
    }

    /// Generates code for the trait implementations required to make the
    /// generated ink! storage struct for cross-calling work out-of-the-box
    /// for the cross-calling infrastructure.
    fn generate_standard_impls(&self) -> TokenStream2 {
        let only_as_dependency =
            self.generate_code_using::<generator::OnlyAsDependencyCfg>();
        let ident = self.contract.module().storage().ident();
        quote! {
            #only_as_dependency
            const _: () = {
                impl ::ink_env::call::FromAccountId<Environment> for #ident {
                    #[inline]
                    fn from_account_id(account_id: AccountId) -> Self {
                        Self { account_id }
                    }
                }

                impl ::ink_lang::ToAccountId<Environment> for #ident {
                    #[inline]
                    fn to_account_id(&self) -> AccountId {
                        self.account_id
                    }
                }
            };
        }
    }

    /// Builds up the `ink_env::call::utils::ArgumentList` type structure for the given types.
    fn generate_arg_list<'a, Args>(args: Args) -> TokenStream2
    where
        Args: IntoIterator<Item = &'a syn::Type>,
        <Args as IntoIterator>::IntoIter: DoubleEndedIterator,
    {
        args.into_iter().fold(
            quote! { ::ink_env::call::utils::EmptyArgumentList },
            |rest, arg| quote! {
                ::ink_env::call::utils::ArgumentList<::ink_env::call::utils::Argument<#arg>, #rest>
            }
        )
    }

    /// Returns the identifier for the generated call forwarder utility.
    fn call_forwarder_ident() -> Ident {
        format_ident!("__ink_CallForwarder")
    }

    /// Create the identifier of an enforced ink! compilation error.
    fn enforce_error_ident(error: EnforcedErrors) -> syn::Ident {
        format_ident!(
            "__ink_enforce_error_{}",
            serde_hex::to_hex(&scale::Encode::encode(&error), false)
        )
    }

    /// Returns the identifier for the generated `Out*` assoc. type.
    fn out_assoc_type_ident(method_ident: &Ident) -> syn::Ident {
        format_ident!("{}Out", method_ident.to_string().to_camel_case())
    }

    fn generate_call_forwarder_trait_ghost_message(
        message: ir::CallableWithSelector<ir::Message>,
    ) -> TokenStream2 {
        let span = message.span();
        let ident = message.ident();
        let output_ident = Self::out_assoc_type_ident(ident);
        let composed_selector = message.composed_selector().as_bytes().to_owned();
        let attrs = message.attrs();
        let input_bindings = message
            .inputs()
            .enumerate()
            .map(|(n, _)| format_ident!("__ink_binding_{}", n))
            .collect::<Vec<_>>();
        let input_types = message
            .inputs()
            .map(|pat_type| &*pat_type.ty)
            .collect::<Vec<_>>();
        let trait_ident = message
            .item_impl()
            .trait_ident()
            .expect("trait identifier must exist")
            .to_string();
        let linker_error_ident =
            Self::enforce_error_ident(EnforcedErrors::CannotCallTraitMessage {
                trait_ident,
                message_ident: ident.to_string(),
                message_selector: composed_selector,
                message_is_mut: message.receiver().is_ref_mut(),
            });
        let output_ty = message
            .output()
            .cloned()
            .unwrap_or_else(|| syn::parse_quote! { () });
        let pub_tok = match message.item_impl().trait_path() {
            Some(_) => None,
            None => Some(quote! { pub }),
        };
        let mut_tok = match message.receiver() {
            ir::Receiver::Ref => None,
            ir::Receiver::RefMut => Some(quote! { mut }),
        };
        quote_spanned!(span=>
            type #output_ident = #output_ty;

            #( #attrs )*
            #[cold]
            #[doc(hidden)]
            #pub_tok fn #ident(
                & #mut_tok self,
                #( #input_bindings : #input_types ),*
            ) -> Self::#output_ident {
                extern {
                    fn #linker_error_ident() -> !;
                }
                unsafe { #linker_error_ident() }
            }
        )
    }

    fn generate_call_forwarder_trait_proper_message(
        message: ir::CallableWithSelector<ir::Message>,
    ) -> TokenStream2 {
        let span = message.span();
        let ident = message.ident();
        let output_ident = Self::out_assoc_type_ident(ident);
        let composed_selector = message.composed_selector().as_bytes().to_owned();
        let attrs = message.attrs();
        let input_bindings = message
            .inputs()
            .enumerate()
            .map(|(n, _)| format_ident!("__ink_binding_{}", n))
            .collect::<Vec<_>>();
        let input_types = message
            .inputs()
            .map(|pat_type| &*pat_type.ty)
            .collect::<Vec<_>>();
        let arg_list = Self::generate_arg_list(input_types.iter().cloned());
        let output = message.output();
        let output_sig = output.map_or_else(
            || quote! { () },
            |output| quote! { ::ink_env::call::utils::ReturnType<#output> },
        );
        let pub_tok = match message.item_impl().trait_path() {
            Some(_) => None,
            None => Some(quote! { pub }),
        };
        let mut_tok = match message.receiver() {
            ir::Receiver::Ref => None,
            ir::Receiver::RefMut => Some(quote! { mut }),
        };
        quote_spanned!(span=>
            #[allow(clippy::type_complexity)]
            type #output_ident = ::ink_env::call::CallBuilder<
                Environment,
                ::ink_env::call::utils::Set<AccountId>,
                ::ink_env::call::utils::Unset<u64>,
                ::ink_env::call::utils::Unset<Balance>,
                ::ink_env::call::utils::Set<::ink_env::call::ExecutionInput<#arg_list>>,
                ::ink_env::call::utils::Set<#output_sig>,
            >;

            #( #attrs )*
            #[inline]
            #pub_tok fn #ident(
                & #mut_tok #(, #input_bindings : #input_types )*
            ) -> Self::#output_ident {
                ::ink_env::call::build_call::<Environment>()
                    .callee(::ink_lang::ToAccountId::to_account_id(self.contract))
                    .exec_input(
                        ::ink_env::call::ExecutionInput::new(
                            ::ink_env::call::Selector::new([ #( #composed_selector ),* ])
                        )
                        #(
                            .push_arg(#input_bindings)
                        )*
                    )
                    .returns::<#output_sig>()
            }
        )
    }

    /// Generates code for a single call forwarder trait message.
    ///
    /// The `mutable` parameter indicates whether only read-only (`false`) or
    /// write-only (`true`) messages shall be valid calls. For non valid messages
    /// an invalid implementation is provided so that actually calling those
    /// will result in a compiler or linker error.
    fn generate_call_forwarder_trait_message(
        mutable: bool,
        message: ir::CallableWithSelector<ir::Message>,
    ) -> TokenStream2 {
        if mutable == message.receiver().is_ref_mut() {
            Self::generate_call_forwarder_trait_proper_message(message)
        } else {
            Self::generate_call_forwarder_trait_ghost_message(message)
        }
    }

    /// Generates code for a single call forwarder trait constructor.
    ///
    /// Note that constructors never need to be forwarded and that we only
    /// provide their implementations to satisfy the implementation block.
    /// We generally try to generate code in a way that actually calling
    /// those constructors will result in a compiler or linker error.
    fn generate_call_forwarder_trait_constructor(
        constructor: ir::CallableWithSelector<ir::Constructor>,
    ) -> TokenStream2 {
        let span = constructor.span();
        let attrs = constructor.attrs();
        let ident = constructor.ident();
        let output_ident = format_ident!("{}Out", ident.to_string().to_camel_case());
        let composed_selector = constructor.composed_selector().as_bytes().to_owned();
        let trait_ident = constructor
            .item_impl()
            .trait_ident()
            .expect("trait identifier must exist")
            .to_string();
        let linker_error = EnforcedErrors::CannotCallTraitConstructor {
            trait_ident,
            constructor_ident: ident.to_string(),
            constructor_selector: composed_selector,
        };
        let linker_error_ident = format_ident!(
            "__ink_enforce_error_{}",
            serde_hex::to_hex(&scale::Encode::encode(&linker_error), false)
        );
        let input_bindings = constructor
            .inputs()
            .enumerate()
            .map(|(n, _)| format_ident!("__ink_binding_{}", n))
            .collect::<Vec<_>>();
        let input_types = constructor
            .inputs()
            .map(|pat_type| &*pat_type.ty)
            .collect::<Vec<_>>();
        quote_spanned!(span =>
            type #output_ident = ::ink_lang::NeverReturns;

            #( #attrs )*
            #[cold]
            #[doc(hidden)]
            fn #ident(
                #( #input_bindings : #input_types ),*
            ) -> Self::#output_ident {
                extern {
                    fn #linker_error_ident() -> !;
                }
                unsafe { #linker_error_ident() }
            }
        )
    }

    /// Generates code for a single call forwarder trait implementation block.
    ///
    /// The `mutable` parameter indicates whether only read-only (`false`) or
    /// write-only (`true`) messages and constructors are to be considered.
    fn generate_call_forwarder_trait_impl_block(
        &self,
        mutable: bool,
        item_impl: &ir::ItemImpl,
    ) -> TokenStream2 {
        assert!(item_impl.trait_path().is_some());
        let span = item_impl.span();
        let attrs = item_impl.attrs();
        let forwarder_ident = Self::call_forwarder_ident();
        let storage_ident = self.contract.module().storage().ident();
        let mut_tok = if mutable { Some(quote! { mut }) } else { None };
        let constructors = item_impl.iter_constructors().map(|constructor| {
            Self::generate_call_forwarder_trait_constructor(constructor)
        });
        let messages = item_impl
            .iter_messages()
            .map(|message| Self::generate_call_forwarder_trait_message(mutable, message));
        let trait_path = item_impl
            .trait_path()
            .expect("encountered missing trait path for trait impl block");
        let trait_ident = item_impl
            .trait_ident()
            .expect("encountered missing trait identifier for trait impl block");
        let hash = ir::InkTrait::compute_verify_hash(
            trait_ident,
            item_impl.iter_constructors().map(|constructor| {
                let ident = constructor.ident().clone();
                let len_inputs = constructor.inputs().count();
                (ident, len_inputs)
            }),
            item_impl.iter_messages().map(|message| {
                let ident = message.ident().clone();
                let len_inputs = message.inputs().count() + 1;
                let is_mut = message.receiver().is_ref_mut();
                (ident, len_inputs, is_mut)
            }),
        );
        let checksum = u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]) as usize;
        quote_spanned!(span =>
            unsafe impl<'a> ::ink_lang::CheckedInkTrait<[(); #checksum]> for #forwarder_ident<&'a #mut_tok #storage_ident> {}

            #( #attrs )*
            impl<'a> #trait_path for #forwarder_ident<&'a #mut_tok #storage_ident> {
                type __ink_Checksum = [(); #checksum];

                #( #constructors )*
                #( #messages )*
            }
        )
    }

    fn generate_call_forwarder_inherent_message(
        message: ir::CallableWithSelector<ir::Message>,
    ) -> TokenStream2 {
        let span = message.span();
        let ident = message.ident();
        let composed_selector = message.composed_selector().as_bytes().to_owned();
        let attrs = message.attrs();
        let input_bindings = message
            .inputs()
            .enumerate()
            .map(|(n, _)| format_ident!("__ink_binding_{}", n))
            .collect::<Vec<_>>();
        let input_types = message
            .inputs()
            .map(|pat_type| &*pat_type.ty)
            .collect::<Vec<_>>();
        let arg_list = Self::generate_arg_list(input_types.iter().cloned());
        let output = message.output();
        let output_sig = output.map_or_else(
            || quote! { () },
            |output| quote! { ::ink_env::call::utils::ReturnType<#output> },
        );
        let pub_tok = match message.item_impl().trait_path() {
            Some(_) => None,
            None => Some(quote! { pub }),
        };
        quote_spanned!(span=>
            #( #attrs )*
            #[inline]
            #[allow(clippy::type_complexity)]
            #pub_tok fn #ident(
                self,
                #( #input_bindings : #input_types ),*
            ) -> ::ink_env::call::CallBuilder<
                Environment,
                ::ink_env::call::utils::Set<AccountId>,
                ::ink_env::call::utils::Unset<u64>,
                ::ink_env::call::utils::Unset<Balance>,
                ::ink_env::call::utils::Set<::ink_env::call::ExecutionInput<#arg_list>>,
                ::ink_env::call::utils::Set<#output_sig>,
            > {
                ::ink_env::call::build_call::<Environment>()
                    .callee(::ink_lang::ToAccountId::to_account_id(self.contract))
                    .exec_input(
                        ::ink_env::call::ExecutionInput::new(
                            ::ink_env::call::Selector::new([ #( #composed_selector ),* ])
                        )
                        #(
                            .push_arg(#input_bindings)
                        )*
                    )
                    .returns::<#output_sig>()
            }
        )
    }

    /// Generates code for a single call forwarder inherent implementation block.
    ///
    /// The `mutable` parameter indicates whether only read-only (`false`) or
    /// write-only (`true`) messages and constructors are to be considered.
    fn generate_call_forwarder_inherent_impl_block(
        &self,
        mutable: bool,
        item_impl: &ir::ItemImpl,
    ) -> TokenStream2 {
        assert!(item_impl.trait_path().is_none());
        let span = item_impl.span();
        let attrs = item_impl.attrs();
        let forwarder_ident = Self::call_forwarder_ident();
        let storage_ident = self.contract.module().storage().ident();
        let mut_tok = if mutable { Some(quote! { mut }) } else { None };
        let messages = item_impl
            .iter_messages()
            .filter(|message| mutable == message.receiver().is_ref_mut())
            .map(Self::generate_call_forwarder_inherent_message);
        quote_spanned!(span =>
            #( #attrs )*
            impl<'a> #forwarder_ident<&'a #mut_tok #storage_ident> {
                #( #messages )*
            }
        )
    }

    /// Generates code for the call forwarder implementation blocks.
    ///
    /// The `mutable` parameter indicates whether only read-only (`false`) or
    /// write-only (`true`) messages and constructors are to be considered.
    fn generate_call_forwarder_impl_blocks(&self, mutable: bool) -> TokenStream2 {
        let impl_blocks = self.contract.module().impls().map(|item_impl| {
            match item_impl.trait_path() {
                Some(_) => {
                    self.generate_call_forwarder_trait_impl_block(mutable, item_impl)
                }
                None => {
                    self.generate_call_forwarder_inherent_impl_block(mutable, item_impl)
                }
            }
        });
        quote! { #( #impl_blocks )* }
    }

    /// Generates code for the call forwarder utility struct.
    fn generate_call_forwarders(&self) -> TokenStream2 {
        let forwarder_ident = Self::call_forwarder_ident();
        let storage_ident = self.contract.module().storage().ident();
        let impl_blocks_ref = self.generate_call_forwarder_impl_blocks(false);
        let impl_blocks_refmut = self.generate_call_forwarder_impl_blocks(true);
        let only_as_dependency =
            self.generate_code_using::<generator::OnlyAsDependencyCfg>();
        quote! {
            #only_as_dependency
            const _: () = {
                impl<'a> ::ink_lang::ForwardCall for &'a #storage_ident {
                    type Forwarder = #forwarder_ident<&'a #storage_ident>;

                    #[inline]
                    fn call(self) -> Self::Forwarder {
                        #forwarder_ident { contract: self }
                    }
                }

                impl<'a> ::ink_lang::ForwardCallMut for &'a mut #storage_ident {
                    type Forwarder = #forwarder_ident<&'a mut #storage_ident>;

                    #[inline]
                    fn call_mut(self) -> Self::Forwarder {
                        #forwarder_ident { contract: self }
                    }
                }

                // Forwards contract messages to the chain.
                #[doc(hidden)]
                pub struct #forwarder_ident<T> {
                    contract: T,
                }

                #impl_blocks_ref
                #impl_blocks_refmut
            };
        }
    }

    /// Generates the code to allow short-hand cross-chain contract calls for messages.
    fn generate_trait_impl_block_message(
        &self,
        message: ir::CallableWithSelector<ir::Message>,
    ) -> TokenStream2 {
        let storage_ident_str = self.contract.module().storage().ident().to_string();
        let span = message.span();
        let ident = message.ident();
        let output_ident = format_ident!("{}Out", ident.to_string().to_camel_case());
        let ident_str = ident.to_string();
        let trait_path = message
            .item_impl()
            .trait_path()
            .expect("encountered missing trait path for trait impl block")
            .segments
            .iter()
            .map(|path_segment| &path_segment.ident)
            .map(ToString::to_string)
            .join("::");
        let error_str = format!(
            "encountered error while calling <{} as {}>::{}",
            storage_ident_str, trait_path, ident_str
        );
        let inputs_sig = message.inputs();
        let inputs_params = message.inputs().map(|pat_type| &pat_type.pat);
        let output_ty = message
            .output()
            .cloned()
            .unwrap_or_else(|| syn::parse_quote! { () });
        let receiver = message.receiver();
        let forward_ident = match receiver {
            ir::Receiver::Ref => format_ident!("call"),
            ir::Receiver::RefMut => format_ident!("call_mut"),
        };
        let forward_trait = match receiver {
            ir::Receiver::Ref => format_ident!("ForwardCall"),
            ir::Receiver::RefMut => format_ident!("ForwardCallMut"),
        };
        let opt_mut = match receiver {
            ir::Receiver::Ref => None,
            ir::Receiver::RefMut => Some(quote! { mut }),
        };
        let opt_pub = match message.item_impl().trait_path() {
            None => Some(quote! { pub }),
            Some(_) => None,
        };
        quote_spanned!(span =>
            type #output_ident = #output_ty;

            #[inline]
            #opt_pub fn #ident( #receiver #(, #inputs_sig )* ) -> Self::#output_ident {
                <&#opt_mut Self as ::ink_lang::#forward_trait>::#forward_ident(self)
                    .#ident( #( #inputs_params ),* )
                    .fire()
                    .expect(#error_str)
            }
        )
    }

    /// Generates the code to allow cross-chain contract calls for trait constructors.
    fn generate_trait_impl_block_constructor(
        constructor: ir::CallableWithSelector<ir::Constructor>,
    ) -> TokenStream2 {
        let span = constructor.span();
        let attrs = constructor.attrs();
        let ident = constructor.ident();
        let output_ident = format_ident!("{}Out", ident.to_string().to_camel_case());
        let composed_selector = constructor.composed_selector().as_bytes().to_owned();
        let input_bindings = constructor
            .inputs()
            .enumerate()
            .map(|(n, _)| format_ident!("__ink_binding_{}", n))
            .collect::<Vec<_>>();
        let input_types = constructor
            .inputs()
            .map(|pat_type| &*pat_type.ty)
            .collect::<Vec<_>>();
        let arg_list = Self::generate_arg_list(input_types.iter().cloned());
        quote_spanned!(span =>
            #[allow(clippy::type_complexity)]
            type #output_ident = ::ink_env::call::CreateBuilder<
                Environment,
                ::ink_env::call::utils::Unset<Hash>,
                ::ink_env::call::utils::Unset<u64>,
                ::ink_env::call::utils::Unset<Balance>,
                ::ink_env::call::utils::Set<::ink_env::call::ExecutionInput<#arg_list>>,
                ::ink_env::call::utils::Unset<::ink_env::call::state::Salt>,
                Self,
            >;

            #( #attrs )*
            #[inline]
            fn #ident(
                #( #input_bindings : #input_types ),*
            ) -> Self::#output_ident {
                ::ink_env::call::build_create::<Environment, Salt, Self>()
                    .exec_input(
                        ::ink_env::call::ExecutionInput::new(
                            ::ink_env::call::Selector::new([ #( #composed_selector ),* ])
                        )
                        #(
                            .push_arg(#input_bindings)
                        )*
                    )
            }
        )
    }

    fn generate_trait_impl_block(&self, impl_block: &ir::ItemImpl) -> TokenStream2 {
        assert!(impl_block.trait_path().is_some());
        let only_as_dependency =
            self.generate_code_using::<generator::OnlyAsDependencyCfg>();
        let span = impl_block.span();
        let attrs = impl_block.attrs();
        let trait_path = impl_block
            .trait_path()
            .expect("encountered missing trait path");
        let trait_ident = impl_block
            .trait_ident()
            .expect("encountered missing trait identifier");
        let self_type = impl_block.self_type();
        let messages = impl_block
            .iter_messages()
            .map(|message| self.generate_trait_impl_block_message(message));
        let constructors = impl_block
            .iter_constructors()
            .map(Self::generate_trait_impl_block_constructor);
        let hash = ir::InkTrait::compute_verify_hash(
            trait_ident,
            impl_block.iter_constructors().map(|constructor| {
                let ident = constructor.ident().clone();
                let len_inputs = constructor.inputs().count();
                (ident, len_inputs)
            }),
            impl_block.iter_messages().map(|message| {
                let ident = message.ident().clone();
                let len_inputs = message.inputs().count() + 1;
                let is_mut = message.receiver().is_ref_mut();
                (ident, len_inputs, is_mut)
            }),
        );
        let checksum = u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]) as usize;
        quote_spanned!(span =>
            #only_as_dependency
            unsafe impl ::ink_lang::CheckedInkTrait<[(); #checksum]> for #self_type {}

            #only_as_dependency
            #( #attrs )*
            impl #trait_path for #self_type {
                type __ink_Checksum = [(); #checksum];

                #( #messages )*
                #( #constructors )*
            }
        )
    }

    /// Generates the code to allow short-hand cross-chain contract calls for constructors.
    ///
    /// # Note
    ///
    /// For constructors this is the only way they are able to be called.
    fn generate_inherent_impl_block_constructor(
        constructor: ir::CallableWithSelector<ir::Constructor>,
    ) -> TokenStream2 {
        let span = constructor.span();
        let attrs = constructor.attrs();
        let ident = constructor.ident();
        let composed_selector = constructor.composed_selector().as_bytes().to_owned();
        let input_bindings = constructor
            .inputs()
            .enumerate()
            .map(|(n, _)| format_ident!("__ink_binding_{}", n))
            .collect::<Vec<_>>();
        let input_types = constructor
            .inputs()
            .map(|pat_type| &*pat_type.ty)
            .collect::<Vec<_>>();
        let arg_list = Self::generate_arg_list(input_types.iter().cloned());
        quote_spanned!(span =>
            #( #attrs )*
            #[inline]
            #[allow(clippy::type_complexity)]
            pub fn #ident(
                #( #input_bindings : #input_types ),*
            ) -> ::ink_env::call::CreateBuilder<
                Environment,
                ::ink_env::call::utils::Unset<Hash>,
                ::ink_env::call::utils::Unset<u64>,
                ::ink_env::call::utils::Unset<Balance>,
                ::ink_env::call::utils::Set<::ink_env::call::ExecutionInput<#arg_list>>,
                ::ink_env::call::utils::Unset<::ink_env::call::state::Salt>,
                Self,
            > {
                ::ink_env::call::build_create::<Environment, Self>()
                    .exec_input(
                        ::ink_env::call::ExecutionInput::new(
                            ::ink_env::call::Selector::new([ #( #composed_selector ),* ])
                        )
                        #(
                            .push_arg(#input_bindings)
                        )*
                    )
            }
        )
    }

    /// Generates the code to allow short-hand cross-chain contract calls for messages.
    fn generate_inherent_impl_block_message(
        &self,
        message: ir::CallableWithSelector<ir::Message>,
    ) -> TokenStream2 {
        let storage_ident_str = self.contract.module().storage().ident().to_string();
        let span = message.span();
        let ident = message.ident();
        let ident_str = ident.to_string();
        let error_str = format!(
            "encountered error while calling {}::{}",
            storage_ident_str, ident_str
        );
        let inputs_sig = message.inputs();
        let inputs_params = message.inputs().map(|pat_type| &pat_type.pat);
        let output_sig = message.output().map(|output| quote! { -> #output });
        let receiver = message.receiver();
        let forward_ident = match receiver {
            ir::Receiver::Ref => format_ident!("call"),
            ir::Receiver::RefMut => format_ident!("call_mut"),
        };
        let forward_trait = match receiver {
            ir::Receiver::Ref => format_ident!("ForwardCall"),
            ir::Receiver::RefMut => format_ident!("ForwardCallMut"),
        };
        let opt_mut = match receiver {
            ir::Receiver::Ref => None,
            ir::Receiver::RefMut => Some(quote! { mut }),
        };
        let opt_pub = match message.item_impl().trait_path() {
            None => Some(quote! { pub }),
            Some(_) => None,
        };
        quote_spanned!(span =>
            #[inline]
            #opt_pub fn #ident( #receiver #(, #inputs_sig )* ) #output_sig {
                <&#opt_mut Self as ::ink_lang::#forward_trait>::#forward_ident(self)
                    .#ident( #( #inputs_params ),* )
                    .fire()
                    .expect(#error_str)
            }
        )
    }

    fn generate_inherent_impl_block(&self, impl_block: &ir::ItemImpl) -> TokenStream2 {
        assert!(impl_block.trait_path().is_none());
        let only_as_dependency =
            self.generate_code_using::<generator::OnlyAsDependencyCfg>();
        let span = impl_block.span();
        let attrs = impl_block.attrs();
        let self_type = impl_block.self_type();
        let messages = impl_block
            .iter_messages()
            .map(|message| self.generate_inherent_impl_block_message(message));
        let constructors = impl_block.iter_constructors().map(|constructor| {
            Self::generate_inherent_impl_block_constructor(constructor)
        });
        quote_spanned!(span =>
            #only_as_dependency
            #( #attrs )*
            impl #self_type {
                #( #messages )*
                #( #constructors )*
            }
        )
    }

    fn generate_impl_blocks(&self) -> TokenStream2 {
        let impl_blocks = self.contract.module().impls().map(|impl_block| {
            match impl_block.trait_path() {
                Some(_) => self.generate_trait_impl_block(impl_block),
                None => self.generate_inherent_impl_block(impl_block),
            }
        });
        quote! {
            #( #impl_blocks )*
        }
    }
}
