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
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

/// Generates `#[cfg(..)]` code to guard against compilation under `ink-as-dependency`.
#[derive(From)]
pub struct NotAsDependencyCfg<'a> {
    contract: &'a ir::Contract,
}

impl GenerateCode for NotAsDependencyCfg<'_> {
    fn generate_code(&self) -> TokenStream2 {
        if self.contract.config().is_compile_as_dependency_enabled() {
            // We use `__ink_DO_NOT_COMPILE` in order to craft a `cfg` that
            // never evaluates to `true` and therefore is always disabled.
            return quote! { #[cfg(feature = "__ink_DO_NOT_COMPILE")] }
        }
        quote! { #[cfg(not(feature = "ink-as-dependency"))] }
    }
}

/// Generates `#[cfg(..)]` code to only allow compilation when `ink-as-dependency` is enabled.
///
/// The `ink-as-dependency` can be enabled mainly by 2 different ways:
///
/// - Enabling it in the associated `Cargo.toml` as crate feature.
///     - Note: This can be enabled by dependencies of an ink! smart contract.
/// - Enabling it in the configuration header with `#[ink::contract(compile_as_dependency = true)]`.
///     - If set here the contract will always be compiled as it is was a dependency.
#[derive(From)]
pub struct OnlyAsDependencyCfg<'a> {
    contract: &'a ir::Contract,
}

impl GenerateCode for OnlyAsDependencyCfg<'_> {
    fn generate_code(&self) -> TokenStream2 {
        if self.contract.config().is_compile_as_dependency_enabled() {
            // We return no code since no code is required to disable compilation.
            return quote! {}
        }
        quote! {
            #[cfg(feature = "ink-as-dependency")]
        }
    }
}
