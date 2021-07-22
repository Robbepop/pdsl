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
use ir::Callable as _;
use proc_macro2::TokenStream as TokenStream2;
use quote::{
    quote,
    quote_spanned,
};
use syn::spanned::Spanned as _;

/// Generates code to generate the metadata of the contract.
#[derive(From)]
pub struct Metadata<'a> {
    /// The contract to generate code for.
    contract: &'a ir::Contract,
}

impl Metadata<'_> {
    fn generate_cgf(&self) -> TokenStream2 {
        if self.contract.config().is_compile_as_dependency_enabled() {
            return quote! { #[cfg(feature = "__ink_DO_NOT_COMPILE")] }
        }
        quote! { #[cfg(not(feature = "ink-as-dependency"))] }
    }
}

impl GenerateCode for Metadata<'_> {
    fn generate_code(&self) -> TokenStream2 {
        let contract = self.generate_contract();
        let layout = self.generate_layout();
        let no_cross_calling_cfg = self.generate_cgf();

        quote! {
            #[cfg(feature = "std")]
            #no_cross_calling_cfg
            const _: () = {
                #[no_mangle]
                pub fn __ink_generate_metadata() -> ::ink_metadata::InkProject  {
                    let contract: ::ink_metadata::ContractSpec = {
                        #contract
                    };
                    let layout: ::ink_metadata::layout::Layout = {
                        #layout
                    };
                    ::ink_metadata::InkProject::new(layout, contract)
                }
            };
        }
    }
}

impl Metadata<'_> {
    fn generate_layout(&self) -> TokenStream2 {
        let contract_ident = self.contract.module().storage().ident();
        quote! {
            <#contract_ident as ::ink_storage::traits::StorageLayout>::layout(
                &mut ::ink_primitives::KeyPtr::from(::ink_primitives::Key::from([0x00; 32]))
            )
        }
    }

    fn generate_contract(&self) -> TokenStream2 {
        let constructors = self.generate_constructors();
        let messages = self.generate_messages();
        let events = self.generate_events();
        let docs = self.generate_docs();

        quote! {
            ::ink_metadata::ContractSpec::new()
                .constructors(vec![
                    #(#constructors ,)*
                ])
                .messages(vec![
                    #(#messages ,)*
                ])
                .events(vec![
                    #(#events ,)*
                ])
                .docs(vec![
                    #(#docs ,)*
                ])
                .done()
        }
    }

    /// Extracts the doc strings from the given slice of attributes.
    fn extract_doc_comments(
        attributes: &[syn::Attribute],
    ) -> impl Iterator<Item = String> + '_ {
        attributes
            .iter()
            .filter_map(|attribute| {
                match attribute.parse_meta() {
                    Ok(syn::Meta::NameValue(name_value)) => Some(name_value),
                    Ok(_) | Err(_) => None,
                }
            })
            .filter(|name_value| name_value.path.is_ident("doc"))
            .filter_map(|name_value| {
                match name_value.lit {
                    syn::Lit::Str(lit_str) => Some(lit_str.value()),
                    _ => None,
                }
            })
    }

    /// Generates ink! metadata for all contract constructors.
    fn generate_constructors(&self) -> impl Iterator<Item = TokenStream2> + '_ {
        self.contract
            .module()
            .impls()
            .flat_map(|impl_block| {
                let trait_ident = impl_block
                    .trait_path()
                    .map(|path| path.segments.last().map(|seg| &seg.ident))
                    .flatten();
                impl_block
                    .iter_constructors()
                    .map(move |constructor| (trait_ident, constructor))
            })
            .map(|(trait_ident, constructor)| {
                let span = constructor.span();
                let attrs = constructor.attrs();
                let docs = Self::extract_doc_comments(attrs);
                let selector = constructor.composed_selector();
                let selector_bytes = selector.as_bytes();
                let constructor = constructor.callable();
                let ident_lit = constructor.metadata_name();
                let args = constructor
                    .inputs()
                    .map(|arg| Self::generate_message_param(arg));
                let constr = match trait_ident {
                    Some(trait_ident) => {
                        let trait_ident_lit = trait_ident.to_string();
                        quote_spanned!(span => from_trait_and_name(#trait_ident_lit, #ident_lit))
                    }
                    None => {
                        quote_spanned!(span => from_name(#ident_lit))
                    }
                };
                quote_spanned!(span =>
                    ::ink_metadata::ConstructorSpec::#constr
                        .selector([#(#selector_bytes),*])
                        .args(vec![
                            #(#args ,)*
                        ])
                        .docs(vec![
                            #(#docs ,)*
                        ])
                        .done()
                )
            })
    }

    /// Generates the ink! metadata for the given parameter and parameter type.
    fn generate_message_param(pat_type: &syn::PatType) -> TokenStream2 {
        let ident = match &*pat_type.pat {
            syn::Pat::Ident(ident) => &ident.ident,
            _ => unreachable!("encountered unexpected non identifier in ink! parameter"),
        };
        let ident_lit = ident.to_string();
        let type_spec = Self::generate_type_spec(&pat_type.ty);
        quote! {
            ::ink_metadata::MessageParamSpec::new(#ident_lit)
                .of_type(#type_spec)
                .done()
        }
    }

    /// Generates the ink! metadata for the given type.
    fn generate_type_spec(ty: &syn::Type) -> TokenStream2 {
        fn without_display_name(ty: &syn::Type) -> TokenStream2 {
            quote! { ::ink_metadata::TypeSpec::new::<#ty>() }
        }
        if let syn::Type::Path(type_path) = ty {
            if type_path.qself.is_some() {
                return without_display_name(ty)
            }
            let path = &type_path.path;
            if path.segments.is_empty() {
                return without_display_name(ty)
            }
            let segs = path
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect::<Vec<_>>();
            quote! {
                ::ink_metadata::TypeSpec::with_name_segs::<#ty, _>(
                    vec![#(#segs),*].into_iter().map(AsRef::as_ref)
                )
            }
        } else {
            without_display_name(ty)
        }
    }

    fn generate_messages(&self) -> impl Iterator<Item = TokenStream2> + '_ {
        self.contract
            .module()
            .impls()
            .flat_map(|impl_block| {
                impl_block
                    .iter_messages()
                    .map(move |message| (impl_block.trait_metadata_name(), message))
            })
            .map(|(trait_name, message)| {
                let span = message.span();
                let attrs = message.attrs();
                let docs = Self::extract_doc_comments(attrs);
                let selector = message.composed_selector();
                let selector_bytes = selector.as_bytes();
                let is_payable = message.is_payable();
                let message = message.callable();
                let mutates = message.receiver().is_ref_mut();
                let ident_lit = message.metadata_name();
                let args = message
                    .inputs()
                    .map(|arg| Self::generate_message_param(arg));
                let ret_ty = Self::generate_return_type(message.output());
                let constr = match trait_name {
                    Some(trait_name_string) => {
                        quote_spanned!(span => from_trait_and_name(#trait_name_string, #ident_lit))
                    }
                    None => {
                        quote_spanned!(span => from_name(#ident_lit))
                    }
                };
                quote_spanned!(span =>
                    ::ink_metadata::MessageSpec::#constr
                        .selector([#(#selector_bytes),*])
                        .args(vec![
                            #(#args ,)*
                        ])
                        .returns(#ret_ty)
                        .mutates(#mutates)
                        .payable(#is_payable)
                        .docs(vec![
                            #(#docs ,)*
                        ])
                        .done()
                )
            })
    }

    /// Generates ink! metadata for the given return type.
    fn generate_return_type(ret_ty: Option<&syn::Type>) -> TokenStream2 {
        match ret_ty {
            None => {
                quote! {
                    ::ink_metadata::ReturnTypeSpec::new(None)
                }
            }
            Some(ty) => {
                let type_spec = Self::generate_type_spec(ty);
                quote! {
                    ::ink_metadata::ReturnTypeSpec::new(#type_spec)
                }
            }
        }
    }

    /// Generates ink! metadata for all user provided ink! event definitions.
    fn generate_events(&self) -> impl Iterator<Item = TokenStream2> + '_ {
        self.contract.module().events().map(|event| {
            let span = event.span();
            let ident = event.ident();
            let ident_lit = ident.to_string();
            let docs = Self::extract_doc_comments(event.attrs());
            let args = Self::generate_event_args(event);
            quote_spanned!(span =>
                ::ink_metadata::EventSpec::new(#ident_lit)
                    .args(vec![
                        #( #args, )*
                    ])
                    .docs(vec![
                        #( #docs, )*
                    ])
                    .done()
            )
        })
    }

    /// Generate ink! metadata for a single argument of an ink! event definition.
    fn generate_event_args(event: &ir::Event) -> impl Iterator<Item = TokenStream2> + '_ {
        event.fields().map(|event_field| {
            let span = event_field.span();
            let ident = event_field.ident();
            let ident_lit = ident.map(ToString::to_string);
            let is_topic = event_field.is_topic;
            let attrs = event_field.attrs();
            let docs = Self::extract_doc_comments(&attrs);
            let ty = Self::generate_type_spec(event_field.ty());
            quote_spanned!(span =>
                ::ink_metadata::EventParamSpec::new(#ident_lit)
                    .of_type(#ty)
                    .indexed(#is_topic)
                    .docs(vec![
                        #( #docs, )*
                    ])
                    .done()
            )
        })
    }

    /// Generates the documentation for the contract module.
    fn generate_docs(&self) -> impl Iterator<Item = String> + '_ {
        Self::extract_doc_comments(self.contract.module().attrs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_doc_comments_works() {
        assert_eq!(
            Metadata::extract_doc_comments(&[syn::parse_quote!( #[doc = r"content"] )])
                .collect::<Vec<_>>(),
            vec!["content".to_string()],
        );
        assert_eq!(
            Metadata::extract_doc_comments(&[syn::parse_quote!(
                /// content
            )])
            .collect::<Vec<_>>(),
            vec![" content".to_string()],
        );
        assert_eq!(
            Metadata::extract_doc_comments(&[syn::parse_quote!(
                /**
                 * Multi-line comments ...
                 * May span many lines
                 */
            )])
            .collect::<Vec<_>>(),
            vec![r"
                 * Multi-line comments ...
                 * May span many lines
                 "
            .to_string()],
        );
        assert_eq!(
            Metadata::extract_doc_comments(&[
                syn::parse_quote!(
                    /// multiple
                ),
                syn::parse_quote!(
                    /// single
                ),
                syn::parse_quote!(
                    /// line
                ),
                syn::parse_quote!(
                    /// comments
                ),
            ])
            .collect::<Vec<_>>(),
            vec![
                " multiple".to_string(),
                " single".to_string(),
                " line".to_string(),
                " comments".to_string(),
            ],
        );
        assert_eq!(
            Metadata::extract_doc_comments(&[
                syn::parse_quote!( #[doc = r"a"] ),
                syn::parse_quote!( #[non_doc] ),
                syn::parse_quote!( #[doc = r"b"] ),
                syn::parse_quote!( #[derive(NonDoc)] ),
                syn::parse_quote!( #[doc = r"c"] ),
                syn::parse_quote!( #[docker = false] ),
                syn::parse_quote!( #[doc = r"d"] ),
                syn::parse_quote!( #[doc(Nope)] ),
                syn::parse_quote!( #[doc = r"e"] ),
            ])
            .collect::<Vec<_>>(),
            vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
                "e".to_string(),
            ],
        )
    }
}
