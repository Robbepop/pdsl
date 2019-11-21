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

//! A binary heap collection.
//! The heap depends on `Ord` and is a max-heap by default. In order to
//! make it a min-heap implement the `Ord` trait explicitly on the type
//! which is stored in the heap.
//!
//! Provides `O(log(n))` push and pop operations.

#[cfg(all(test, feature = "test-env"))]
mod tests;

mod duplex_sync_chunk;
mod impls;

pub use self::impls::{
    BinaryHeap,
    Iter,
    Values,
};
