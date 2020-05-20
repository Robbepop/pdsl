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

//! A storage hash map that allows to associate keys with values.

mod impls;
mod iter;
mod storage;

#[cfg(test)]
mod tests;

pub use self::iter::{
    Iter,
    IterMut,
    Keys,
    Values,
    ValuesMut,
};
use crate::{
    hash::hasher::{
        Blake2x256Hasher,
        Hasher,
    },
    storage2::{
        collections::Stash,
        lazy::LazyHashMap,
        traits::PackedLayout,
    },
};
use core::{
    borrow::Borrow,
    cmp::Eq,
};
use ink_prelude::borrow::ToOwned;
use ink_primitives::Key;

/// The index type within a hashmap.
///
/// # Note
///
/// Used for key indices internal to the hashmap.
type KeyIndex = u32;

/// A hash map operating on the contract storage.
///
/// Stores a mapping between keys and values.
///
/// # Note
///
/// Unlike Rust's standard `HashMap` that uses the [`core::hash::Hash`] trait
/// in order to hash its keys the storage hash map uses the [`scale::Encode`]
/// encoding in order to hash its keys using a built-in cryptographic
/// hash function provided by the chain runtime.
///
/// The main difference between the lower-level `LazyHashMap` and the
/// `storage::HashMap` is that the latter is aware of its associated keys and
/// values and operates on those instances directly as opposed to `Option`
/// instances of them. Also it provides a more high-level and user focused
/// API.
///
/// Users should generally prefer using this storage hash map over the low-level
/// `LazyHashMap` for direct usage in their smart contracts.
#[derive(Debug)]
pub struct HashMap<K, V, H = Blake2x256Hasher>
where
    K: Ord + Clone + PackedLayout,
    V: PackedLayout,
    H: Hasher,
    Key: From<<H as Hasher>::Output>,
{
    /// The keys of the storage hash map.
    keys: Stash<K>,
    /// The values of the storage hash map.
    values: LazyHashMap<K, ValueEntry<V>, H>,
}

/// An entry within the storage hash map.
///
/// Stores the value as well as the index to its associated key.
#[derive(Debug, scale::Encode, scale::Decode)]
struct ValueEntry<V> {
    /// The value stored in this entry.
    value: V,
    /// The index of the key associated with this value.
    key_index: KeyIndex,
}

impl<K, V, H> HashMap<K, V, H>
where
    K: Ord + Clone + PackedLayout,
    V: PackedLayout,
    H: Hasher,
    Key: From<<H as Hasher>::Output>,
{
    /// Creates a new empty storage hash map.
    pub fn new() -> Self {
        Self {
            keys: Stash::new(),
            values: LazyHashMap::new(),
        }
    }

    /// Returns the number of key- value pairs stored in the hash map.
    pub fn len(&self) -> u32 {
        self.keys.len()
    }

    /// Returns `true` if the hash map is empty.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Returns an iterator yielding shared references to all key/value pairs
    /// of the hash map.
    ///
    /// # Note
    ///
    /// - Avoid unbounded iteration over big storage hash maps.
    /// - Prefer using methods like `Iterator::take` in order to limit the number
    ///   of yielded elements.
    pub fn iter(&self) -> Iter<K, V, H> {
        Iter::new(self)
    }

    /// Returns an iterator yielding exclusive references to all key/value pairs
    /// of the hash map.
    ///
    /// # Note
    ///
    /// - Avoid unbounded iteration over big storage hash maps.
    /// - Prefer using methods like `Iterator::take` in order to limit the number
    ///   of yielded elements.
    pub fn iter_mut(&mut self) -> IterMut<K, V, H> {
        IterMut::new(self)
    }

    /// Returns an iterator yielding shared references to all values of the hash map.
    ///
    /// # Note
    ///
    /// - Avoid unbounded iteration over big storage hash maps.
    /// - Prefer using methods like `Iterator::take` in order to limit the number
    ///   of yielded elements.
    pub fn values(&self) -> Values<K, V, H> {
        Values::new(self)
    }

    /// Returns an iterator yielding shared references to all values of the hash map.
    ///
    /// # Note
    ///
    /// - Avoid unbounded iteration over big storage hash maps.
    /// - Prefer using methods like `Iterator::take` in order to limit the number
    ///   of yielded elements.
    pub fn values_mut(&mut self) -> ValuesMut<K, V, H> {
        ValuesMut::new(self)
    }

    /// Returns an iterator yielding shared references to all keys of the hash map.
    ///
    /// # Note
    ///
    /// - Avoid unbounded iteration over big storage hash maps.
    /// - Prefer using methods like `Iterator::take` in order to limit the number
    ///   of yielded elements.
    pub fn keys(&self) -> Keys<K> {
        Keys::new(self)
    }
}

impl<K, V, H> HashMap<K, V, H>
where
    K: Ord + Clone + PackedLayout,
    V: PackedLayout,
    H: Hasher,
    Key: From<<H as Hasher>::Output>,
{
    fn clear_cells(&self) {
        if self.values.key().is_none() {
            // We won't clear any storage if we are in lazy state since there
            // probably has not been any state written to storage, yet.
            return
        }
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
    }
}

impl<K, V, H> HashMap<K, V, H>
where
    K: Ord + Eq + Clone + PackedLayout,
    V: PackedLayout,
    H: Hasher,
    Key: From<H::Output>,
{
    /// Inserts a key-value pair into the map.
    ///
    /// Returns the previous value associated with the same key if any.
    /// If the map did not have this key present, `None` is returned.
    ///
    /// # Note
    ///
    /// - If the map did have this key present, the value is updated,
    ///   and the old value is returned. The key is not updated, though;
    ///   this matters for types that can be `==` without being identical.
    pub fn insert(&mut self, key: K, new_value: V) -> Option<V> {
        if let Some(occupied) = self.values.get_mut(&key) {
            // Update value, don't update key.
            let old_value = core::mem::replace(&mut occupied.value, new_value);
            return Some(old_value)
        }
        // At this point we know that `key` does not yet exist in the map.
        let key_index = self.keys.put(key.to_owned());
        self.values.put(
            key,
            Some(ValueEntry {
                value: new_value,
                key_index,
            }),
        );
        None
    }

    /// Removes the key/value pair from the map associated with the given key.
    ///
    /// - Returns the removed value if any.
    ///
    /// # Note
    ///
    /// The key may be any borrowed form of the map's key type,
    /// but `Hash` and `Eq` on the borrowed form must match those for the key type.
    pub fn take<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + scale::Encode + ToOwned<Owned = K>,
    {
        let entry = self.values.put_get(key, None)?;
        self.keys
            .take(entry.key_index)
            .expect("`key_index` must point to a valid key entry");
        Some(entry.value)
    }

    /// Returns a shared reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type,
    /// but `Hash` and `Eq` on the borrowed form must match those for the key type.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + scale::Encode + ToOwned<Owned = K>,
    {
        self.values.get(key).map(|entry| &entry.value)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type,
    /// but `Hash` and `Eq` on the borrowed form must match those for the key type.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Ord + scale::Encode + ToOwned<Owned = K>,
    {
        self.values.get_mut(key).map(|entry| &mut entry.value)
    }

    /// Returns `true` if there is an entry corresponding to the key in the map.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + PartialEq<K> + Eq + scale::Encode + ToOwned<Owned = K>,
    {
        self.values
            .get(key)
            .map(|entry| entry.key_index)
            .and_then(|key_index| {
                self.keys.get(key_index).map(|stored_key| key == stored_key)
            })
            .unwrap_or(false)
    }

    /// Defragments storage used by the storage hash map.
    ///
    /// Returns the number of storage cells freed this way.
    ///
    /// A `max_iterations` parameter of `None` means that there is no limit
    /// to the number of iterations performed. This is generally not advised.
    ///
    /// # Note
    ///
    /// This frees storage that is held but not necessary for the hash map to hold.
    /// This operation might be expensive, especially for big `max_iteration`
    /// parameters. The `max_iterations` parameter can be used to limit the
    /// expensiveness for this operation and instead free up storage incrementally.
    pub fn defrag(&mut self, max_iterations: Option<u32>) -> u32 {
        // This method just defrags the underlying `storage::Stash` used to
        // store the keys as it can sometimes take a lot of unused storage
        // if many keys have been removed at some point. Some hash map
        // implementations might even prefer to perform this operation with a
        // limit set to 1 after every successful removal.
        let len_vacant = self.keys.capacity() - self.keys.len();
        let max_iterations = max_iterations.unwrap_or(len_vacant);
        let values = &mut self.values;
        let callback = |old_index, new_index, key: &K| {
            let value_entry = values.get_mut(key).expect("key must be valid");
            debug_assert_eq!(value_entry.key_index, old_index);
            value_entry.key_index = new_index;
        };
        self.keys.defrag(Some(max_iterations), callback)
    }
}
