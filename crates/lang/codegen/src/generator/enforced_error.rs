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

use impl_serde::serialize as serde_hex;
use quote::format_ident;

/// Errors which may occur when forwarding a call is not allowed.
///
/// We insert markers for these errors in the generated contract code.
/// This is necessary since we can't check these errors at compile time
/// of the contract.
/// `cargo-contract` checks the contract code for these error markers
/// when building a contract and fails if it finds markers.
#[derive(scale::Encode, scale::Decode)]
pub enum EnforcedErrors {
    /// The below error represents calling a `&mut self` message in a context that
    /// only allows for `&self` messages. This may happen under certain circumstances
    /// when ink! trait implementations are involved with long-hand calling notation.
    #[codec(index = 1)]
    CannotCallTraitMessage {
        /// The trait that defines the called message.
        trait_ident: String,
        /// The name of the called message.
        message_ident: String,
        /// The selector of the called message.
        message_selector: [u8; 4],
        /// Is `true` if the `self` receiver of the ink! message is `&mut self`.
        message_is_mut: bool,
    },
    /// The below error represents calling a constructor in a context that does
    /// not allow calling it. This may happen when the constructor defined in a
    /// trait is cross-called in another contract.
    /// This is not allowed since the contract to which a call is forwarded must
    /// already exist at the point when the call to it is made.
    #[codec(index = 2)]
    CannotCallTraitConstructor {
        /// The trait that defines the called constructor.
        trait_ident: String,
        /// The name of the called constructor.
        constructor_ident: String,
        /// The selector of the called constructor.
        constructor_selector: [u8; 4],
    },
}

impl EnforcedErrors {
    /// Create the identifier of an enforced ink! compilation error.
    fn into_ident(self) -> syn::Ident {
        format_ident!(
            "__ink_enforce_error_{}",
            serde_hex::to_hex(&scale::Encode::encode(&self), false)
        )
    }

    /// Creates an enforced linker error to signal that an invalid
    /// implementation of an ink! trait message has been called.
    pub fn cannot_call_trait_message(
        trait_ident: &syn::Ident,
        message_ident: &syn::Ident,
        message_selector: ir::Selector,
        message_is_mut: bool,
    ) -> syn::Ident {
        Self::CannotCallTraitMessage {
            trait_ident: trait_ident.to_string(),
            message_ident: message_ident.to_string(),
            message_selector: *message_selector.as_bytes(),
            message_is_mut,
        }
        .into_ident()
    }
}
