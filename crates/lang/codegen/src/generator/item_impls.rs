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
    GenerateCodeUsing as _,
};
use derive_more::From;
use heck::CamelCase as _;
use ir::Callable as _;
use proc_macro2::TokenStream as TokenStream2;
use quote::{
    format_ident,
    quote,
    quote_spanned,
    ToTokens,
};
use syn::spanned::Spanned as _;

/// Generates code for all ink! implementation blocks.
#[derive(From)]
pub struct ItemImpls<'a> {
    contract: &'a ir::Contract,
}
impl_as_ref_for_generator!(ItemImpls);

impl GenerateCode for ItemImpls<'_> {
    fn generate_code(&self) -> TokenStream2 {
        let item_impls = self
            .contract
            .module()
            .impls()
            .map(|item_impl| self.generate_item_impl(item_impl));
        let no_cross_calling_cfg =
            self.generate_code_using::<generator::NotAsDependencyCfg>();
        quote! {
            #no_cross_calling_cfg
            const _: () = {
                use ::ink_lang::{Env, EmitEvent, StaticEnv};

                #( #item_impls )*
            };
        }
    }
}

impl ItemImpls<'_> {
    /// Generates the code for the given ink! constructor within a trait implementation block.
    fn generate_trait_constructor(constructor: &ir::Constructor) -> TokenStream2 {
        let span = constructor.span();
        let attrs = constructor.attrs();
        let vis = match constructor.visibility() {
            ir::Visibility::Inherited => None,
            ir::Visibility::Public(vis_public) => Some(vis_public),
        };
        let ident = constructor.ident();
        let output_ident = format_ident!("{}Out", ident.to_string().to_camel_case());
        let inputs = constructor.inputs();
        let statements = constructor.statements();
        quote_spanned!(span =>
            type #output_ident = Self;

            #( #attrs )*
            #vis fn #ident( #( #inputs ),* ) -> Self::#output_ident {
                #( #statements )*
            }
        )
    }

    /// Generates the code for the given ink! message within a trait implementation block.
    fn generate_trait_message(message: &ir::Message) -> TokenStream2 {
        let span = message.span();
        let attrs = message.attrs();
        let vis = match message.visibility() {
            ir::Visibility::Inherited => None,
            ir::Visibility::Public(vis_public) => Some(vis_public),
        };
        let receiver = match message.receiver() {
            ir::Receiver::RefMut => quote! { &mut self },
            ir::Receiver::Ref => quote! { &self },
        };
        let ident = message.ident();
        let output_ident = format_ident!("{}Out", ident.to_string().to_camel_case());
        let inputs = message.inputs();
        let output = message
            .output()
            .cloned()
            .unwrap_or_else(|| syn::parse_quote! { () });
        let statements = message.statements();
        quote_spanned!(span =>
            type #output_ident = #output;

            #( #attrs )*
            #vis fn #ident(#receiver #(, #inputs )* ) -> Self::#output_ident {
                #( #statements )*
            }
        )
    }

    fn generate_trait_item_impl(item_impl: &ir::ItemImpl) -> TokenStream2 {
        assert!(item_impl.trait_path().is_some());
        let span = item_impl.span();
        let attrs = item_impl.attrs();
        let messages = item_impl
            .iter_messages()
            .map(|cws| Self::generate_trait_message(cws.callable()));
        let constructors = item_impl
            .iter_constructors()
            .map(|cws| Self::generate_trait_constructor(cws.callable()));
        let other_items = item_impl
            .items()
            .iter()
            .filter_map(ir::ImplItem::filter_map_other_item)
            .map(ToTokens::to_token_stream);
        let trait_path = item_impl
            .trait_path()
            .expect("encountered missing trait path for trait impl block");
        let trait_ident = item_impl
            .trait_ident()
            .expect("encountered missing trait identifier for trait impl block");
        let self_type = item_impl.self_type();
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
            unsafe impl ::ink_lang::CheckedInkTrait<[(); #checksum]> for #self_type {}

            #( #attrs )*
            impl #trait_path for #self_type {
                type __ink_Checksum = [(); #checksum];
                type __ink_ConcreteImplementer = ::ink_lang::NoConcreteImplementer;

                #( #constructors )*
                #( #messages )*
                #( #other_items )*
            }
        )
    }

    /// Generates the code for the given ink! constructor within an inherent implementation block.
    fn generate_inherent_constructor(constructor: &ir::Constructor) -> TokenStream2 {
        let span = constructor.span();
        let attrs = constructor.attrs();
        let vis = match constructor.visibility() {
            ir::Visibility::Inherited => None,
            ir::Visibility::Public(vis_public) => Some(vis_public),
        };
        let ident = constructor.ident();
        let inputs = constructor.inputs();
        let statements = constructor.statements();
        quote_spanned!(span =>
            #( #attrs )*
            #vis fn #ident( #( #inputs ),* ) -> Self {
                #( #statements )*
            }
        )
    }

    /// Generates the code for the given ink! message within an inherent implementation block.
    fn generate_inherent_message(message: &ir::Message) -> TokenStream2 {
        let span = message.span();
        let attrs = message.attrs();
        let vis = match message.visibility() {
            ir::Visibility::Inherited => None,
            ir::Visibility::Public(vis_public) => Some(vis_public),
        };
        let receiver = match message.receiver() {
            ir::Receiver::RefMut => quote! { &mut self },
            ir::Receiver::Ref => quote! { &self },
        };
        let ident = message.ident();
        let inputs = message.inputs();
        let output_arrow = message.output().map(|_| quote! { -> });
        let output = message.output();
        let statements = message.statements();
        quote_spanned!(span =>
            #( #attrs )*
            #vis fn #ident(#receiver, #( #inputs ),* ) #output_arrow #output {
                #( #statements )*
            }
        )
    }

    fn generate_inherent_item_impl(item_impl: &ir::ItemImpl) -> TokenStream2 {
        assert!(item_impl.trait_path().is_none());
        let span = item_impl.span();
        let attrs = item_impl.attrs();
        let messages = item_impl
            .iter_messages()
            .map(|cws| Self::generate_inherent_message(cws.callable()));
        let constructors = item_impl
            .iter_constructors()
            .map(|cws| Self::generate_inherent_constructor(cws.callable()));
        let other_items = item_impl
            .items()
            .iter()
            .filter_map(ir::ImplItem::filter_map_other_item)
            .map(ToTokens::to_token_stream);
        let self_type = item_impl.self_type();
        quote_spanned!(span =>
            #( #attrs )*
            impl #self_type {
                #( #constructors )*
                #( #messages )*
                #( #other_items )*
            }
        )
    }

    /// Generates code to guard against ink! implementations that have not been implemented
    /// for the ink! storage struct.
    fn generate_item_impl_self_ty_guard(&self, item_impl: &ir::ItemImpl) -> TokenStream2 {
        let self_ty = item_impl.self_type();
        let span = self_ty.span();
        let storage_ident = self.contract.module().storage().ident();
        quote_spanned!(span =>
            ::ink_lang::static_assertions::assert_type_eq_all!(
                #self_ty,
                #storage_ident,
            );
        )
    }

    /// Generates code for the given ink! implementation block.
    fn generate_item_impl(&self, item_impl: &ir::ItemImpl) -> TokenStream2 {
        let self_ty_guard = self.generate_item_impl_self_ty_guard(item_impl);
        let impl_block = match item_impl.trait_path() {
            Some(_) => Self::generate_trait_item_impl(item_impl),
            None => Self::generate_inherent_item_impl(item_impl),
        };
        quote! {
            #self_ty_guard
            #impl_block
        }
    }
}
