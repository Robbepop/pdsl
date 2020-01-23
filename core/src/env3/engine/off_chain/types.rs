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

//! Emulated environmental types of the test environment.
//!
//! Due to technical constraints it is not possible to define the test
//! environment instance thread-locally and generically over the actual
//! environmental types.
//!
//! For that reason we store the type information of all the environmental
//! types at runtime to at least have some kind of type safety upon access.
//! This is done via the `TypedEncoded` abstraction that stores the
//! SCALE encoded bytes and also has a runtime type information marker
//! assigned upon initialization to check whether accesses to it are
//! type safe.

use super::TypedEncoded;

/// Type markers used in conjunction with `TypedEncoded`.
#[rustfmt::skip]
mod type_marker {
    /// Type marker representing an environmental `AccountId`.
    #[derive(Debug)] pub enum AccountId {}
    /// Type marker representing an environmental `Balance`.
    #[derive(Debug)] pub enum Balance {}
    /// Type marker representing an environmental `Hash`.
    #[derive(Debug)] pub enum Hash {}
    /// Type marker representing an environmental `Moment`.
    #[derive(Debug)] pub enum Moment {}
    /// Type marker representing an environmental `BlockNumber`.
    #[derive(Debug)] pub enum BlockNumber {}
    /// Type marker representing an environmental `Call`.
    #[derive(Debug)] pub enum Call {}
}

/// Off-chain environment account ID type.
pub type OffAccountId = TypedEncoded<type_marker::AccountId>;
/// Off-chain environment balance type.
pub type OffBalance = TypedEncoded<type_marker::Balance>;
/// Off-chain environment hash type.
pub type OffHash = TypedEncoded<type_marker::Hash>;
/// Off-chain environment moment (block time) type.
pub type OffMoment = TypedEncoded<type_marker::Moment>;
/// Off-chain environment block number type.
pub type OffBlockNumber = TypedEncoded<type_marker::BlockNumber>;
/// Off-chain environment call (runtime dispatch) type.
pub type OffCall = TypedEncoded<type_marker::Call>;
