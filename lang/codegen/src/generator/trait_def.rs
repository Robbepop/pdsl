// Copyright 2018-2020 Parity Technologies (UK) Ltd.
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
use proc_macro2::TokenStream as TokenStream2;
use quote::{
    format_ident,
    quote_spanned,
};

/// Generator to create the ink! storage struct and important trait impls.
#[derive(From)]
pub struct TraitDefinition<'a> {
    trait_def: &'a ir::InkTrait,
}

impl<'a> TraitDefinition<'a> {
    fn generate_for_constructor(
        constructor: ir::InkTraitConstructor<'a>,
    ) -> TokenStream2 {
        let span = constructor.span();
        let attrs = constructor.attrs();
        let sig = constructor.sig();
        let ident = &sig.ident;
        let inputs = &sig.inputs;
        quote_spanned!(span =>
            #(#attrs)*
            fn #ident(#inputs) -> Self::Output;
        )
    }

    fn generate_for_message(message: ir::InkTraitMessage<'a>) -> TokenStream2 {
        let span = message.span();
        let attrs = message.attrs();
        let sig = message.sig();
        let ident = &sig.ident;
        let inputs = &sig.inputs;
        let output = &sig.output;
        quote_spanned!(span =>
            #(#attrs)*
            fn #ident(#inputs) -> #output;
        )
    }
}

impl GenerateCode for TraitDefinition<'_> {
    fn generate_code(&self) -> TokenStream2 {
        let span = self.trait_def.span();
        let attrs = self.trait_def.attrs();
        let hash = self.trait_def.verify_hash();
        let ident = self.trait_def.ident();
        let helper_ident = format_ident!(
            "__ink_Checked{}_0x{:X}{:X}{:X}{:X}",
            ident,
            hash[0],
            hash[1],
            hash[2],
            hash[3]
        );
        let verify_hash_id = u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]);
        let constructors = self
            .trait_def
            .iter_items()
            .flat_map(ir::InkTraitItem::filter_map_constructor)
            .map(Self::generate_for_constructor);
        let messages = self
            .trait_def
            .iter_items()
            .flat_map(ir::InkTraitItem::filter_map_message)
            .map(Self::generate_for_message);
        quote_spanned!(span =>
            #(#attrs)*
            pub trait #ident: ::ink_lang::CheckedInkTrait<[(); #verify_hash_id]> {
                type Output;
                #[doc(hidden)]
                type __ink_Checksum: #helper_ident;

                #(#constructors)*
                #(#messages)*
            }

            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            pub unsafe trait #helper_ident {}

            const _: () = {
                unsafe impl #helper_ident for [(); #verify_hash_id] {}
            };
        )
    }
}
