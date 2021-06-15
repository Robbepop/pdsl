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
use quote::quote;

/// Generates code for the entirety of the ink! contract.
#[derive(From)]
pub struct Contract<'a> {
    /// The contract to generate code for.
    contract: &'a ir::Contract,
}

impl AsRef<ir::Contract> for Contract<'_> {
    fn as_ref(&self) -> &ir::Contract {
        self.contract
    }
}

impl GenerateCode for Contract<'_> {
    /// Generates ink! contract code.
    fn generate_code(&self) -> TokenStream2 {
        let module = self.contract.module();
        let ident = module.ident();
        let attrs = module.attrs();
        let vis = module.vis();
        let env = self.generate_code_using::<generator::Env>();
        let storage = self.generate_code_using::<generator::Storage>();
        let events = self.generate_code_using::<generator::Events>();
        let dispatch = self.generate_code_using::<generator::Dispatch>();
        let item_impls = self.generate_code_using::<generator::ItemImpls>();
        let cross_calling = self.generate_code_using::<generator::CrossCalling>();
        let metadata = self.generate_code_using::<generator::Metadata>();
        let call_builder = self.generate_code_using::<generator::CallBuilder>();
        let non_ink_items = self
            .contract
            .module()
            .items()
            .iter()
            .filter_map(ir::Item::map_rust_item);
        quote! {
            #( #attrs )*
            #vis mod #ident {
                #env
                #storage
                #events
                #dispatch
                #item_impls
                #cross_calling
                #call_builder
                #metadata
                #( #non_ink_items )*
            }
        }
    }
}
