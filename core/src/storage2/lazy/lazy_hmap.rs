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

use super::{
    Entry,
    EntryState,
};
use crate::{
    hash::{
        hasher::Hasher,
        HashBuilder,
    },
    storage2::traits::{
        clear_packed_root,
        pull_packed_root_opt,
        KeyPtr,
        PackedLayout,
        SpreadLayout,
    },
};
use core::{
    borrow::Borrow,
    cell::{
        RefCell,
        UnsafeCell,
    },
    cmp::{
        Eq,
        Ord,
    },
    ptr::NonNull,
};
use ink_prelude::{
    borrow::ToOwned,
    boxed::Box,
    collections::BTreeMap,
    vec::Vec,
};
use ink_primitives::Key;

/// The map for the contract storage entries.
///
/// # Note
///
/// We keep the whole entry in a `Box<T>` in order to prevent pointer
/// invalidation upon updating the cache through `&self` methods as in
/// [`LazyMap::get`].
pub type EntryMap<K, V> = BTreeMap<K, Box<Entry<V>>>;

/// A lazy storage mapping that stores entries under their SCALE encoded key hashes.
///
/// # Note
///
/// This is mainly used as low-level storage primitives by other high-level
/// storage primitives in order to manage the contract storage for a whole
/// mapping of storage cells.
///
/// This storage data structure might store its entires anywhere in the contract
/// storage. It is the users responsibility to keep track of the entries if it
/// is necessary to do so.
#[derive(Debug)]
pub struct LazyHashMap<K, V, H> {
    /// The offset key for the storage mapping.
    ///
    /// This offsets the mapping for the entries stored in the contract storage
    /// so that all lazy hash map instances store equal entries at different
    /// locations of the contract storage and avoid collissions.
    key: Option<Key>,
    /// The currently cached entries of the lazy storage mapping.
    ///
    /// This normally only represents a subset of the total set of elements.
    /// An entry is cached as soon as it is loaded or written.
    cached_entries: UnsafeCell<EntryMap<K, V>>,
    /// The used hash builder.
    hash_builder: RefCell<HashBuilder<H, Vec<u8>>>,
}

impl<K, V, H, O> SpreadLayout for LazyHashMap<K, V, H>
where
    K: Ord + scale::Encode,
    V: PackedLayout,
    H: Hasher<Output = O>,
    O: Default,
    Key: From<O>,
{
    const FOOTPRINT: u64 = 1;

    fn pull_spread(ptr: &mut KeyPtr) -> Self {
        Self::lazy(ptr.next_for::<Self>())
    }

    fn push_spread(&self, ptr: &mut KeyPtr) {
        let offset_key = ptr.next_for::<Self>();
        for (index, entry) in self.entries().iter() {
            let root_key = self.to_offset_key(&offset_key, index);
            entry.push_packed_root(&root_key);
        }
    }

    #[inline]
    fn clear_spread(&self, _ptr: &mut KeyPtr) {
        // Low-level lazy abstractions won't perform automated clean-up since
        // they generally are not aware of their entire set of associated
        // elements. The high-level abstractions that build upon them are
        // responsible for cleaning up.
    }
}

// # Developer Note
//
// Even thought `LazyHashMap` would require storing just a single key a thus
// be a packable storage entity we cannot really make it one since this could
// allow for overlapping lazy hash map instances.
// An example for this would be a `Pack<(LazyHashMap, LazyHashMap)>` where
// both lazy hash maps would use the same underlying key and thus would apply
// the same underlying key mapping.

impl<K, V, H> Default for LazyHashMap<K, V, H>
where
    K: Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, H> LazyHashMap<K, V, H>
where
    K: Ord,
{
    /// Creates a new empty lazy hash map.
    ///
    /// # Note
    ///
    /// A lazy map created this way cannot be used to load from the contract storage.
    /// All operations that directly or indirectly load from storage will panic.
    pub fn new() -> Self {
        Self {
            key: None,
            cached_entries: UnsafeCell::new(EntryMap::new()),
            hash_builder: RefCell::new(HashBuilder::from(Vec::new())),
        }
    }

    /// Creates a new empty lazy hash map positioned at the given key.
    ///
    /// # Note
    ///
    /// This constructor is private and should never need to be called from
    /// outside this module. It is used to construct a lazy index map from a
    /// key that is only useful upon a contract call. Use [`LazyIndexMap::new`]
    /// for construction during contract initialization.
    fn lazy(key: Key) -> Self {
        Self {
            key: Some(key),
            cached_entries: UnsafeCell::new(EntryMap::new()),
            hash_builder: RefCell::new(HashBuilder::from(Vec::new())),
        }
    }

    /// Returns the offset key of the lazy map if any.
    pub fn key(&self) -> Option<&Key> {
        self.key.as_ref()
    }

    /// Returns a shared reference to the underlying entries.
    fn entries(&self) -> &EntryMap<K, V> {
        // SAFETY: It is safe to return a `&` reference from a `&self` receiver.
        unsafe { &*self.cached_entries.get() }
    }

    /// Returns an exclusive reference to the underlying entries.
    fn entries_mut(&mut self) -> &mut EntryMap<K, V> {
        // SAFETY: It is safe to return a `&mut` reference from a `&mut self` receiver.
        unsafe { &mut *self.cached_entries.get() }
    }

    /// Puts the new value under the given key.
    ///
    /// # Note
    ///
    /// - Use [`LazyHashMap::put`]`(None)` in order to remove an element.
    /// - Prefer this method over [`LazyHashMap::put_get`] if you are not interested
    ///   in the old value of the same cell index.
    ///
    /// # Panics
    ///
    /// - If the lazy hash map is in an invalid state that forbids interaction
    ///   with the underlying contract storage.
    /// - If the decoding of the old element at the given index failed.
    pub fn put(&mut self, key: K, new_value: Option<V>) {
        self.entries_mut()
            .insert(key, Box::new(Entry::new(new_value, EntryState::Mutated)));
    }
}

impl<K, V, H, O> LazyHashMap<K, V, H>
where
    K: Ord + scale::Encode,
    H: Hasher<Output = O>,
    O: Default,
    Key: From<O>,
{
    /// Returns an offset key for the given key pair.
    fn to_offset_key<Q>(&self, storage_key: &Key, key: &Q) -> Key
    where
        K: Borrow<Q>,
        Q: scale::Encode,
    {
        #[derive(scale::Encode)]
        struct KeyPair<'a, Q> {
            storage_key: &'a Key,
            value_key: &'a Q,
        }
        let key_pair = KeyPair {
            storage_key,
            value_key: key,
        };
        self.hash_builder
            .borrow_mut()
            .hash_encoded(&key_pair)
            .into()
    }

    /// Returns an offset key for the given key.
    fn key_at<Q>(&self, key: &Q) -> Option<Key>
    where
        K: Borrow<Q>,
        Q: scale::Encode,
    {
        self.key
            .map(|storage_key| self.to_offset_key(&storage_key, key))
    }
}

impl<K, V, H, O> LazyHashMap<K, V, H>
where
    K: Ord + Eq + Clone + scale::Encode,
    V: PackedLayout,
    H: Hasher<Output = O>,
    O: Default,
    Key: From<O>,
{
    /// Lazily loads the value at the given index.
    ///
    /// # Note
    ///
    /// Only loads a value if `key` is set and if the value has not been loaded yet.
    /// Returns the freshly loaded or already loaded entry of the value.
    ///
    /// # Safety
    ///
    /// This function has a `&self` receiver while returning an `Option<*mut T>`
    /// which is unsafe in isolation. The caller has to determine how to forward
    /// the returned `*mut T`.
    ///
    /// # Panics
    ///
    /// - If the lazy chunk is in an invalid state that forbids interaction.
    /// - If the lazy chunk is not in a state that allows lazy loading.
    ///
    /// # Safety
    ///
    /// This is an `unsafe` operation because it has a `&self` receiver but returns
    /// a `*mut Entry<T>` pointer that allows for exclusive access. This is safe
    /// within internal use only and should never be given outside of the lazy
    /// entity for public `&self` methods.
    unsafe fn lazily_load<Q>(&self, key: &Q) -> NonNull<Entry<V>>
    where
        K: Borrow<Q>,
        Q: Ord + scale::Encode + ToOwned<Owned = K>,
    {
        // SAFETY: We have put the whole `cached_entries` mapping into an
        //         `UnsafeCell` because of this caching functionality. The
        //         trick here is that due to using `Box<T>` internally
        //         we are able to return references to the cached entries
        //         while maintaining the invariant that mutating the caching
        //         `BTreeMap` will never invalidate those references.
        //         By returning a raw pointer we enforce an `unsafe` block at
        //         the caller site to underline that guarantees are given by the
        //         caller.
        #[allow(unused_unsafe)]
        let cached_entries = unsafe { &mut *self.cached_entries.get() };
        use ink_prelude::collections::btree_map::Entry as BTreeMapEntry;
        // We have to clone the key here because we do not have access to the unsafe
        // raw entry API for Rust hash maps, yet since it is unstable. We can remove
        // the contraints on `K: Clone` once we have access to this API.
        // Read more about the issue here: https://github.com/rust-lang/rust/issues/56167
        match cached_entries.entry(key.to_owned()) {
            BTreeMapEntry::Occupied(occupied) => {
                NonNull::from(&mut **occupied.into_mut())
            }
            BTreeMapEntry::Vacant(vacant) => {
                let offset_key = self
                    .key_at(key)
                    .expect("cannot load lazily in the current state");
                let value = pull_packed_root_opt::<V>(&offset_key);
                NonNull::from(
                    &mut **vacant
                        .insert(Box::new(Entry::new(value, EntryState::Preserved))),
                )
            }
        }
    }

    /// Lazily loads the value associated with the given key.
    ///
    /// # Note
    ///
    /// Only loads a value if `key` is set and if the value has not been loaded yet.
    /// Returns a pointer to the freshly loaded or already loaded entry of the value.
    ///
    /// # Panics
    ///
    /// - If the lazy chunk is in an invalid state that forbids interaction.
    /// - If the lazy chunk is not in a state that allows lazy loading.
    fn lazily_load_mut<Q>(&mut self, index: &Q) -> &mut Entry<V>
    where
        K: Borrow<Q>,
        Q: Ord + scale::Encode + ToOwned<Owned = K>,
    {
        // SAFETY:
        // - Returning a `&mut Entry<T>` is safe because entities inside the
        //   cache are stored within a `Box` to not invalidate references into
        //   them upon operating on the outer cache.
        unsafe { &mut *self.lazily_load(index).as_ptr() }
    }

    /// Clears the underlying storage of the entry at the given index.
    ///
    /// # Safety
    ///
    /// For performance reasons this does not synchronize the lazy index map's
    /// memory-side cache which invalidates future accesses the cleared entry.
    /// Care should be taken when using this API.
    ///
    /// The general use of this API is to streamline `Drop` implementations of
    /// high-level abstractions that build upon this low-level data strcuture.
    pub fn clear_packed_at<Q>(&self, index: &Q)
    where
        K: Borrow<Q>,
        V: PackedLayout,
        Q: Ord + scale::Encode + ToOwned<Owned = K>,
    {
        let root_key = self.key_at(index).expect("cannot clear in lazy state");
        // We need to load the entity before we remove its associated contract storage
        // because it might be a type that propagates clearing to its fields,
        // for example in the case of `T` being a `storage::Box`. However,
        // since in other cases this load is not required but implies a lot of
        // overhead we need to find a way to avoid it in those cases.
        let entity = self.get(index).expect("cannot clear a non existing entity");
        clear_packed_root::<V>(&entity, &root_key);
    }

    /// Returns a shared reference to the value associated with the given key if any.
    ///
    /// # Panics
    ///
    /// - If the lazy chunk is in an invalid state that forbids interaction.
    /// - If the decoding of the element at the given index failed.
    pub fn get<Q>(&self, index: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + scale::Encode + ToOwned<Owned = K>,
    {
        // SAFETY: Dereferencing the `*mut T` pointer into a `&T` is safe
        //         since this method's receiver is `&self` so we do not
        //         leak non-shared references to the outside.
        unsafe { &*self.lazily_load(index).as_ptr() }.value().into()
    }

    /// Returns an exclusive reference to the value associated with the given key if any.
    ///
    /// # Panics
    ///
    /// - If the lazy chunk is in an invalid state that forbids interaction.
    /// - If the decoding of the element at the given index failed.
    pub fn get_mut<Q>(&mut self, index: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Ord + scale::Encode + ToOwned<Owned = K>,
    {
        self.lazily_load_mut(index).value_mut().into()
    }

    /// Takes and returns the value associated with the given key if any.
    ///
    /// # Note
    ///
    /// This removes the value associated with the given key from the storage.
    ///
    /// # Panics
    ///
    /// - If the lazy chunk is in an invalid state that forbids interaction.
    /// - If the decoding of the element at the given index failed.
    pub fn take<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + scale::Encode + ToOwned<Owned = K>,
    {
        self.lazily_load_mut(key).take_value()
    }

    /// Puts the new value under the given key and returns the old value if any.
    ///
    /// # Note
    ///
    /// - Use [`LazyHashMap::put_get`]`(None)` in order to remove an element
    ///   and retrieve the old element back.
    ///
    /// # Panics
    ///
    /// - If the lazy hashmap is in an invalid state that forbids interaction.
    /// - If the decoding of the old element at the given index failed.
    pub fn put_get<Q>(&mut self, key: &Q, new_value: Option<V>) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + scale::Encode + ToOwned<Owned = K>,
    {
        self.lazily_load_mut(key).put(new_value)
    }

    /// Swaps the values at entries with associated keys `x` and `y`.
    ///
    /// This operation tries to be as efficient as possible and reuse allocations.
    ///
    /// # Panics
    ///
    /// - If the lazy hashmap is in an invalid state that forbids interaction.
    /// - If the decoding of one of the elements failed.
    pub fn swap<Q1, Q2>(&mut self, x: &Q1, y: &Q2)
    where
        K: Borrow<Q1> + Borrow<Q2>,
        Q1: Ord + PartialEq<Q2> + scale::Encode + ToOwned<Owned = K>,
        Q2: Ord + PartialEq<Q1> + scale::Encode + ToOwned<Owned = K>,
    {
        if x == y {
            // Bail out early if both indices are the same.
            return
        }
        let (loaded_x, loaded_y) =
            // SAFETY: The loaded `x` and `y` entries are distinct from each
            //         other guaranteed by the previous check. Also `lazily_load`
            //         guarantees to return a pointer to a pinned entity
            //         so that the returned references do not conflict with
            //         each other.
            unsafe { (
                &mut *self.lazily_load(x).as_ptr(),
                &mut *self.lazily_load(y).as_ptr(),
            ) };
        if loaded_x.value().is_none() && loaded_y.value().is_none() {
            // Bail out since nothing has to be swapped if both values are `None`.
            return
        }
        // Set the `mutate` flag since at this point at least one of the loaded
        // values is guaranteed to be `Some`.
        loaded_x.set_state(EntryState::Mutated);
        loaded_y.set_state(EntryState::Mutated);
        core::mem::swap(loaded_x.value_mut(), loaded_y.value_mut());
    }
}
