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

use super::blake2::blake2b_256;
use crate::Namespace;

/// A function selector.
///
/// # Note
///
/// This is equal to the first four bytes of the SHA-3 hash of a function's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Selector {
    bytes: [u8; 4],
}

/// The trait prefix to compute a composed selector for trait implementation blocks.
#[derive(Debug, Copy, Clone)]
pub struct TraitPrefix<'a> {
    /// The namespace of the ink! trait definition.
    ///
    /// By default this is equal to the `module_path!` at the ink! trait definition site.
    /// It can be customized by the ink! trait definition author using `#[ink(namespace = N)]`
    /// ink! attribute.
    namespace: &'a Namespace,
    /// The Rust identifier of the ink! trait definition.
    trait_ident: &'a syn::Ident,
}

impl<'a> TraitPrefix<'a> {
    /// Creates a new trait prefix.
    pub fn new(trait_ident: &'a syn::Ident, namespace: &'a Namespace) -> Self {
        Self {
            namespace,
            trait_ident,
        }
    }

    /// Returns a shared slice over the bytes of the namespace.
    pub fn namespace_bytes(&self) -> &[u8] {
        self.namespace.as_bytes()
    }

    /// Returns a shared reference to the Rust identifier of the trait.
    pub fn trait_ident(&self) -> &'a syn::Ident {
        &self.trait_ident
    }
}

impl Selector {
    /// Creates a new selector from the given raw bytes.
    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        Self { bytes }
    }

    /// Computes the BLAKE-2 256-bit based selector from the given input bytes.
    pub fn new(input: &[u8]) -> Self {
        let mut output = [0; 32];
        blake2b_256(input, &mut output);
        Self::from_bytes([output[0], output[1], output[2], output[3]])
    }

    /// # Note
    ///
    /// - `trait_prefix` is `None` when computing the selector of ink! constructors
    ///   and messages in inherent implementation blocks.
    /// - `trait_prefix` is `Some` when computing the selector of ink! constructors
    ///   and messages in trait implementation blocks. In this case the `namespace`
    ///   is either the full path of the trait definition gained by Rust's
    ///   `module_path!` macro by default or it is customized by manual application
    ///   of the `#[ink(namespace = "my_namespace")]` ink! attribute. In the
    ///   example `my_namespace` concatenated with `::` and the identifier of the
    ///   trait definition would then be part of the provided `trait_prefix` parameter.
    /// - `fn_ident` refers to the ink! constructor or message identifier.
    ///
    /// # Inherent Implementation Blocks
    ///
    /// For inherent implementation blocks, when `trait_prefix` is `None` the composed
    /// selector is computed as follows:
    ///
    /// 1. Apply `BLAKE2` 256-bit hash `H` on the bytes of the ascii representation of
    ///   the `fn_ident` identifier.
    /// 1. The first 4 bytes of `H` make up the selector.
    ///
    /// # Trait Implementation Blocks
    ///
    /// For trait implementation blocks, when `trait_prefix` is
    /// `Some((namespace, trait_ident))` the composed selector is computed as follows:
    ///
    /// 1. Compute the ascii byte representation of `fn_ident` and call it `F`.
    /// 1. Compute the ascii byte representation of `namespace` and call it `N`.
    /// 1. Compute the ascii byte representation of `trait_ident` and call it `T`.
    /// 1. Concatenate `N`, `T` and `F` using `::` as separator and call it `C`.
    /// 1. Apply the `BLAKE2` 256-bit hash `H` of `C`.
    /// 1. The first 4 bytes of `H` make up the selector.
    pub fn compose<'a, T>(trait_prefix: T, fn_ident: &syn::Ident) -> Self
    where
        T: Into<Option<TraitPrefix<'a>>>,
    {
        let fn_ident = fn_ident.to_string().into_bytes();
        let input_bytes: Vec<u8> = match trait_prefix.into() {
            Some(trait_prefix) => {
                let namespace = trait_prefix.namespace_bytes();
                let trait_ident = trait_prefix.trait_ident().to_string().into_bytes();
                let separator = &b"::"[..];
                [namespace, &trait_ident, &fn_ident].join(separator)
            }
            None => fn_ident.to_vec(),
        };
        Self::new(&input_bytes)
    }

    /// Returns the underlying four bytes.
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.bytes
    }

    /// Returns a unique identifier as `usize`.
    pub fn unique_id(self) -> usize {
        u32::from_le_bytes(self.bytes) as usize
    }
}

impl From<[u8; 4]> for Selector {
    fn from(bytes: [u8; 4]) -> Self {
        Self::from_bytes(bytes)
    }
}
