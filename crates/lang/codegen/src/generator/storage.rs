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
    generator,
    GenerateCode,
    GenerateCodeUsing,
};
use derive_more::From;
use proc_macro2::TokenStream as TokenStream2;
use quote::{
    quote,
    quote_spanned,
};
use syn::spanned::Spanned as _;

/// Generator to create the ink! storage struct and important trait impls.
#[derive(From)]
pub struct Storage<'a> {
    contract: &'a ir::Contract,
}
impl_as_ref_for_generator!(Storage);

impl GenerateCode for Storage<'_> {
    fn generate_code(&self) -> TokenStream2 {
        let storage_span = self.contract.module().storage().span();
        let access_env_impls = self.generate_access_env_trait_impls();
        let storage_struct = self.generate_storage_struct();
        let use_emit_event = if self.contract.module().events().next().is_some() {
            // Required to allow for `self.env().emit_event(..)` in messages and constructors.
            Some(quote! { use ::ink_lang::EmitEvent as _; })
        } else {
            None
        };
        let cfg = self.generate_code_using::<generator::NotAsDependencyCfg>();
        quote_spanned!(storage_span =>
            #access_env_impls
            #storage_struct

            #cfg
            const _: () = {
                // Used to make `self.env()` available in message code.
                #[allow(unused_imports)]
                use ::ink_lang::{
                    Env as _,
                    StaticEnv as _,
                };
                #use_emit_event
            };
        )
    }
}

impl Storage<'_> {
    fn generate_access_env_trait_impls(&self) -> TokenStream2 {
        let storage_ident = &self.contract.module().storage().ident();
        let cfg = self.generate_code_using::<generator::NotAsDependencyCfg>();
        quote! {
            #cfg
            const _: () = {
                impl<'a> ::ink_lang::Env for &'a #storage_ident {
                    type EnvAccess = ::ink_lang::EnvAccess<'a, <#storage_ident as ::ink_lang::ContractEnv>::Env>;

                    fn env(self) -> Self::EnvAccess {
                        Default::default()
                    }
                }

                impl<'a> ::ink_lang::StaticEnv for #storage_ident {
                    type EnvAccess = ::ink_lang::EnvAccess<'static, <#storage_ident as ::ink_lang::ContractEnv>::Env>;

                    fn env() -> Self::EnvAccess {
                        Default::default()
                    }
                }
            };
        }
    }

    /// Generates the storage struct definition.
    fn generate_storage_struct(&self) -> TokenStream2 {
        let storage = self.contract.module().storage();
        let span = storage.span();
        let ident = &storage.ident();
        let attrs = &storage.attrs();
        let fields = storage.fields();
        let cfg = self.generate_code_using::<generator::NotAsDependencyCfg>();
        quote_spanned!( span =>
            #cfg
            #(#attrs)*
            #[cfg_attr(
                feature = "std",
                derive(::ink_storage::traits::StorageLayout)
            )]
            #[derive(::ink_storage::traits::SpreadLayout)]
            #[cfg_attr(test, derive(Debug))]
            pub struct #ident {
                #( #fields ),*
            }
        )
    }
}
