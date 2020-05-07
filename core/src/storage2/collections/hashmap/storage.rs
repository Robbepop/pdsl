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

//! Implementation of ink! storage traits.

use super::{
    HashMap as StorageHashMap,
    ValueEntry,
};
use crate::{
    hash::hasher::Hasher,
    storage2::{
        collections::Stash as StorageStash,
        traits::{
            forward_clear_packed,
            forward_pull_packed,
            forward_push_packed,
            KeyPtr,
            PackedLayout,
            SpreadLayout,
        },
    },
};
use ink_primitives::Key;

impl<T> SpreadLayout for ValueEntry<T>
where
    T: PackedLayout,
{
    const FOOTPRINT: u64 = 1;

    fn pull_spread(ptr: &mut KeyPtr) -> Self {
        forward_pull_packed::<Self>(ptr)
    }

    fn push_spread(&self, ptr: &mut KeyPtr) {
        forward_push_packed::<Self>(self, ptr)
    }

    fn clear_spread(&self, ptr: &mut KeyPtr) {
        forward_clear_packed::<Self>(self, ptr)
    }
}

impl<T> PackedLayout for ValueEntry<T>
where
    T: PackedLayout,
{
    fn pull_packed(&mut self, at: &Key) {
        <T as PackedLayout>::pull_packed(&mut self.value, at)
    }

    fn push_packed(&self, at: &Key) {
        <T as PackedLayout>::push_packed(&self.value, at)
    }

    fn clear_packed(&self, at: &Key) {
        <T as PackedLayout>::clear_packed(&self.value, at)
    }
}

impl<K, V, H, O> SpreadLayout for StorageHashMap<K, V, H>
where
    K: Ord + Clone + PackedLayout,
    V: PackedLayout,
    H: Hasher<Output = O>,
    O: Default,
    Key: From<O>,
{
    const FOOTPRINT: u64 = 1 + <StorageStash<K> as SpreadLayout>::FOOTPRINT;

    fn pull_spread(ptr: &mut KeyPtr) -> Self {
        Self {
            keys: SpreadLayout::pull_spread(ptr),
            values: SpreadLayout::pull_spread(ptr),
        }
    }

    fn push_spread(&self, ptr: &mut KeyPtr) {
        SpreadLayout::push_spread(&self.keys, ptr);
        SpreadLayout::push_spread(&self.values, ptr);
    }

    fn clear_spread(&self, ptr: &mut KeyPtr) {
        for key in self.keys() {
            // It might seem wasteful to clear all entries instead of just
            // the occupied ones. However this spares us from having one extra
            // read for every element in the storage stash to filter out vacant
            // entries. So this is actually a trade-off and at the time of this
            // implementation it is unclear which path is more efficient.
            //
            // The bet is that clearing a storage cell is cheaper than reading one.
            self.values.clear_packed_at(key);
        }
        SpreadLayout::clear_spread(&self.keys, ptr);
        SpreadLayout::clear_spread(&self.values, ptr);
    }
}
