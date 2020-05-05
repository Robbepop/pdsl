// Copyright 2019-2020 Parity Technologies (UK) Ltd.
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

use super::SmallVec;
use crate::storage2::{
    lazy::{
        LazyArray,
        LazyArrayLength,
    },
    traits2::{
        KeyPtr as KeyPtr2,
        PackedLayout,
        SpreadLayout,
    },
    ClearForward,
    KeyPtr,
    PullForward,
    PushForward,
    StorageFootprint,
};
use generic_array::typenum::Unsigned;

impl<T, N> SpreadLayout for SmallVec<T, N>
where
    T: PackedLayout,
    T: StorageFootprint + PullForward,
    N: LazyArrayLength<T>,
{
    const FOOTPRINT: u64 = 1 + <N as Unsigned>::U64;

    fn pull_spread(ptr: &mut KeyPtr2) -> Self {
        Self {
            len: SpreadLayout::pull_spread(ptr),
            elems: SpreadLayout::pull_spread(ptr),
        }
    }

    fn push_spread(&self, ptr: &mut KeyPtr2) {
        SpreadLayout::push_spread(&self.len, ptr);
        SpreadLayout::push_spread(&self.elems, ptr);
    }

    fn clear_spread(&self, ptr: &mut KeyPtr2) {
        self.clear_cells();
        SpreadLayout::clear_spread(&self.len, ptr);
        SpreadLayout::clear_spread(&self.elems, ptr);
    }
}

impl<T, N> StorageFootprint for SmallVec<T, N>
where
    T: StorageFootprint + PullForward,
    N: LazyArrayLength<T>,
{
    const VALUE: u64 = 1 + <N as Unsigned>::U64;
}

impl<T, N> PullForward for SmallVec<T, N>
where
    N: LazyArrayLength<T>,
    LazyArray<T, N>: PullForward,
{
    fn pull_forward(ptr: &mut KeyPtr) -> Self {
        Self {
            len: PullForward::pull_forward(ptr),
            elems: PullForward::pull_forward(ptr),
        }
    }
}

impl<T, N> PushForward for SmallVec<T, N>
where
    LazyArray<T, N>: PushForward,
    N: LazyArrayLength<T>,
{
    fn push_forward(&self, ptr: &mut KeyPtr) {
        PushForward::push_forward(&self.len, ptr);
        PushForward::push_forward(&self.elems, ptr);
    }
}

impl<T, N> ClearForward for SmallVec<T, N>
where
    T: StorageFootprint + ClearForward + PullForward,
    N: LazyArrayLength<T>,
{
    fn clear_forward(&self, ptr: &mut KeyPtr) {
        ClearForward::clear_forward(&self.len, ptr);
        // ClearForward::clear_forward(&self.elems, ptr);
        if self.elems.key().is_none() {
            return
        }
        for (index, elem) in self.iter().enumerate() {
            <T as ClearForward>::clear_forward(
                elem,
                &mut KeyPtr::from(
                    self.elems
                        .key_at(index as u32)
                        .expect("expected a key mapping since self.elems.key() is some"),
                ),
            )
        }
    }
}
