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

use super::{
    CacheCell,
    EntryState,
    StorageEntry,
};
use crate::traits::{
    clear_packed_root,
    pull_packed_root_opt,
    ExtKeyPtr,
    KeyPtr,
    PackedLayout,
    SpreadLayout,
};
use core::{
    fmt,
    fmt::Debug,
    ptr::NonNull,
};
use ink_prelude::{
    boxed::Box,
    collections::BTreeMap,
};
use ink_primitives::Key;

/// The index type used in the lazy storage chunk.
pub type Index = u32;

/// A lazy storage chunk that spans over a whole chunk of storage cells.
///
/// # Note
///
/// This is mainly used as low-level storage primitives by other high-level
/// storage primitives in order to manage the contract storage for a whole
/// chunk of storage cells.
///
/// A chunk of storage cells is a contiguous range of `2^32` storage cells.
pub struct LazyIndexMap<V> {
    /// The offset key for the chunk of cells.
    ///
    /// If the lazy chunk has been initialized during contract initialization
    /// the key will be `None` since there won't be a storage region associated
    /// to the lazy chunk which prevents it from lazily loading elements. This,
    /// however, is only checked at contract runtime. We might incorporate
    /// compile-time checks for this particular use case later on.
    key: Option<Key>,
    /// The subset of currently cached entries of the lazy storage chunk.
    ///
    /// An entry is cached as soon as it is loaded or written.
    cached_entries: CacheCell<EntryMap<V>>,
}

struct DebugEntryMap<'a, V>(&'a CacheCell<EntryMap<V>>);

impl<'a, V> Debug for DebugEntryMap<'a, V>
where
    V: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.0.as_inner().iter()).finish()
    }
}

impl<V> Debug for LazyIndexMap<V>
where
    V: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("LazyIndexMap")
            .field("key", &self.key)
            .field("cached_entries", &DebugEntryMap(&self.cached_entries))
            .finish()
    }
}

#[test]
fn debug_impl_works() {
    let mut imap = <LazyIndexMap<i32>>::new();
    // Empty imap.
    assert_eq!(
        format!("{:?}", &imap),
        "LazyIndexMap { key: None, cached_entries: {} }",
    );
    // Filled imap.
    imap.put(0, Some(1));
    imap.put(42, Some(2));
    imap.put(999, None);
    assert_eq!(
        format!("{:?}", &imap),
        "LazyIndexMap { \
            key: None, \
            cached_entries: {\
                0: Entry { \
                    value: Some(1), \
                    state: Mutated \
                }, \
                42: Entry { \
                    value: Some(2), \
                    state: Mutated \
                }, \
                999: Entry { \
                    value: None, \
                    state: Mutated \
                }\
            } \
        }",
    );
}

impl<V> Default for LazyIndexMap<V> {
    fn default() -> Self {
        Self::new()
    }
}

/// The map for the contract storage entries.
///
/// # Note
///
/// We keep the whole entry in a `Box<T>` in order to prevent pointer
/// invalidation upon updating the cache through `&self` methods as in
/// [`LazyIndexMap::get`].
pub type EntryMap<V> = BTreeMap<Index, Box<StorageEntry<V>>>;

impl<V> LazyIndexMap<V> {
    /// Creates a new empty lazy map.
    ///
    /// # Note
    ///
    /// A lazy map created this way cannot be used to load from the contract storage.
    /// All operations that directly or indirectly load from storage will panic.
    pub fn new() -> Self {
        Self {
            key: None,
            cached_entries: CacheCell::new(EntryMap::new()),
        }
    }

    /// Creates a new empty lazy map positioned at the given key.
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
            cached_entries: CacheCell::new(EntryMap::new()),
        }
    }

    /// Returns the offset key of the lazy map if any.
    pub fn key(&self) -> Option<&Key> {
        self.key.as_ref()
    }

    /// Returns a shared reference to the underlying entries.
    fn entries(&self) -> &EntryMap<V> {
        self.cached_entries.as_inner()
    }

    /// Returns an exclusive reference to the underlying entries.
    fn entries_mut(&mut self) -> &mut EntryMap<V> {
        self.cached_entries.as_inner_mut()
    }

    /// Puts the new value at the given index.
    ///
    /// # Note
    ///
    /// - Use [`LazyIndexMap::put`]`(None)` in order to remove an element.
    /// - Prefer this method over [`LazyIndexMap::put_get`] if you are not interested
    ///   in the old value of the same cell index.
    ///
    /// # Panics
    ///
    /// - If the lazy chunk is in an invalid state that forbids interaction.
    /// - If the decoding of the old element at the given index failed.
    pub fn put(&mut self, index: Index, new_value: Option<V>) {
        use ink_prelude::collections::btree_map::Entry as BTreeMapEntry;
        match self.entries_mut().entry(index) {
            BTreeMapEntry::Occupied(mut occupied) => {
                // We can re-use the already existing boxed `Entry` and simply
                // swap the underlying values.
                occupied.get_mut().put(new_value);
            }
            BTreeMapEntry::Vacant(vacant) => {
                vacant
                    .insert(Box::new(StorageEntry::new(new_value, EntryState::Mutated)));
            }
        }
    }
}

#[cfg(feature = "std")]
const _: () = {
    use crate::traits::StorageLayout;
    use ink_metadata::layout::{
        ArrayLayout,
        CellLayout,
        Layout,
        LayoutKey,
    };
    use scale_info::TypeInfo;

    impl<T> StorageLayout for LazyIndexMap<T>
    where
        T: TypeInfo + 'static,
    {
        fn layout(key_ptr: &mut KeyPtr) -> Layout {
            let capacity = u32::MAX;
            Layout::Array(ArrayLayout::new(
                LayoutKey::from(key_ptr.advance_by(capacity as u64)),
                capacity,
                1,
                Layout::Cell(CellLayout::new::<T>(LayoutKey::from(
                    key_ptr.advance_by(0),
                ))),
            ))
        }
    }
};

impl<V> SpreadLayout for LazyIndexMap<V>
where
    V: PackedLayout,
{
    const FOOTPRINT: u64 = 1_u64 << 32;

    fn pull_spread(ptr: &mut KeyPtr) -> Self {
        Self::lazy(*ExtKeyPtr::next_for::<Self>(ptr))
    }

    fn push_spread(&self, ptr: &mut KeyPtr) {
        let offset_key = ExtKeyPtr::next_for::<Self>(ptr);
        for (&index, entry) in self.entries().iter() {
            let root_key = offset_key + (index as u64);
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

impl<V> LazyIndexMap<V>
where
    V: PackedLayout,
{
    /// Clears the underlying storage of the entry at the given index.
    ///
    /// # Safety
    ///
    /// For performance reasons this does not synchronize the lazy index map's
    /// memory-side cache which invalidates future accesses the cleared entry.
    /// Care should be taken when using this API.
    ///
    /// The general use of this API is to streamline `Drop` implementations of
    /// high-level abstractions that build upon this low-level data structure.
    pub fn clear_packed_at(&self, index: Index) {
        let root_key = self.key_at(index).expect("cannot clear in lazy state");
        if <V as SpreadLayout>::REQUIRES_DEEP_CLEAN_UP {
            // We need to load the entity before we remove its associated contract storage
            // because it requires a deep clean-up which propagates clearing to its fields,
            // for example in the case of `T` being a `storage::Box`.
            let entity = self.get(index).expect("cannot clear a non existing entity");
            clear_packed_root::<V>(entity, &root_key);
        } else {
            // The type does not require deep clean-up so we can simply clean-up
            // its associated storage cell and be done without having to load it first.
            ink_env::clear_contract_storage(&root_key);
        }
    }
}

impl<V> LazyIndexMap<V>
where
    V: PackedLayout,
{
    /// Returns an offset key for the given index.
    pub fn key_at(&self, index: Index) -> Option<Key> {
        let key = self.key?;
        let offset_key = key + index as u64;
        Some(offset_key)
    }

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
    /// # Safety
    ///
    /// This is an `unsafe` operation because it has a `&self` receiver but returns
    /// a `*mut Entry<T>` pointer that allows for exclusive access. This is safe
    /// within internal use only and should never be given outside the lazy entity
    /// for public `&self` methods.
    pub(crate) unsafe fn lazily_load(&self, index: Index) -> NonNull<StorageEntry<V>> {
        // SAFETY: We have put the whole `cached_entries` mapping into an
        //         `UnsafeCell` because of this caching functionality. The
        //         trick here is that due to using `Box<T>` internally
        //         we are able to return references to the cached entries
        //         while maintaining the invariant that mutating the caching
        //         `BTreeMap` will never invalidate those references.
        //         By returning a raw pointer we enforce an `unsafe` block at
        //         the caller site to underline that guarantees are given by the
        //         caller.
        let cached_entries = &mut *self.cached_entries.get_ptr().as_ptr();
        use ink_prelude::collections::btree_map::Entry as BTreeMapEntry;
        match cached_entries.entry(index) {
            BTreeMapEntry::Occupied(occupied) => {
                NonNull::from(&mut **occupied.into_mut())
            }
            BTreeMapEntry::Vacant(vacant) => {
                let value = self
                    .key_at(index)
                    .map(|key| pull_packed_root_opt::<V>(&key))
                    .unwrap_or(None);
                NonNull::from(
                    &mut **vacant.insert(Box::new(StorageEntry::new(
                        value,
                        EntryState::Preserved,
                    ))),
                )
            }
        }
    }

    /// Lazily loads the value at the given index.
    ///
    /// # Note
    ///
    /// Only loads a value if `key` is set and if the value has not been loaded yet.
    /// Returns the freshly loaded or already loaded entry of the value.
    ///
    /// # Panics
    ///
    /// - If the lazy chunk is in an invalid state that forbids interaction.
    /// - If the lazy chunk is not in a state that allows lazy loading.
    fn lazily_load_mut(&mut self, index: Index) -> &mut StorageEntry<V> {
        // SAFETY:
        // - Returning a `&mut Entry<T>` is safe because entities inside the
        //   cache are stored within a `Box` to not invalidate references into
        //   them upon operating on the outer cache.
        unsafe { &mut *self.lazily_load(index).as_ptr() }
    }

    /// Returns a shared reference to the element at the given index if any.
    ///
    /// # Panics
    ///
    /// - If the lazy chunk is in an invalid state that forbids interaction.
    /// - If the decoding of the element at the given index failed.
    pub fn get(&self, index: Index) -> Option<&V> {
        // SAFETY: Dereferencing the `*mut T` pointer into a `&T` is safe
        //         since this method's receiver is `&self` so we do not
        //         leak non-shared references to the outside.
        unsafe { &*self.lazily_load(index).as_ptr() }.value().into()
    }

    /// Returns an exclusive reference to the element at the given index if any.
    ///
    /// # Panics
    ///
    /// - If the lazy chunk is in an invalid state that forbids interaction.
    /// - If the decoding of the element at the given index failed.
    pub fn get_mut(&mut self, index: Index) -> Option<&mut V> {
        self.lazily_load_mut(index).value_mut().into()
    }

    /// Puts the new value at the given index and returns the old value if any.
    ///
    /// # Note
    ///
    /// - Use [`LazyIndexMap::put_get`]`(None)` in order to remove an element
    ///   and retrieve the old element back.
    ///
    /// # Panics
    ///
    /// - If the lazy chunk is in an invalid state that forbids interaction.
    /// - If the decoding of the old element at the given index failed.
    pub fn put_get(&mut self, index: Index, new_value: Option<V>) -> Option<V> {
        self.lazily_load_mut(index).put(new_value)
    }

    /// Swaps the values at indices `x` and `y`.
    ///
    /// This operation tries to be as efficient as possible and reuse allocations.
    ///
    /// # Panics
    ///
    /// - If the lazy chunk is in an invalid state that forbids interaction.
    /// - If the decoding of one of the elements failed.
    pub fn swap(&mut self, x: Index, y: Index) {
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
        loaded_x.replace_state(EntryState::Mutated);
        loaded_y.replace_state(EntryState::Mutated);
        core::mem::swap(loaded_x.value_mut(), loaded_y.value_mut());
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{
            EntryState,
            StorageEntry,
        },
        Index,
        LazyIndexMap,
    };
    use crate::traits::{
        KeyPtr,
        SpreadLayout,
    };
    use ink_primitives::Key;

    /// Asserts that the cached entries of the given `imap` is equal to the `expected` slice.
    fn assert_cached_entries(
        imap: &LazyIndexMap<u8>,
        expected: &[(Index, StorageEntry<u8>)],
    ) {
        assert_eq!(imap.entries().len(), expected.len());
        for (given, expected) in imap
            .entries()
            .iter()
            .map(|(index, boxed_entry)| (*index, &**boxed_entry))
            .zip(expected.iter().map(|(index, entry)| (*index, entry)))
        {
            assert_eq!(given, expected);
        }
    }

    #[test]
    fn new_works() {
        let imap = <LazyIndexMap<u8>>::new();
        // Key must be none.
        assert_eq!(imap.key(), None);
        assert_eq!(imap.key_at(0), None);
        // Cached elements must be empty.
        assert_cached_entries(&imap, &[]);
        // Same as default:
        let default_imap = <LazyIndexMap<u8>>::default();
        assert_eq!(imap.key(), default_imap.key());
        assert_eq!(imap.entries(), default_imap.entries());
    }

    #[test]
    fn lazy_works() {
        let key = Key::from([0x42; 32]);
        let imap = <LazyIndexMap<u8>>::lazy(key);
        // Key must be none.
        assert_eq!(imap.key(), Some(&key));
        assert_eq!(imap.key_at(0), Some(key));
        assert_eq!(imap.key_at(1), Some(key + 1u64));
        // Cached elements must be empty.
        assert_cached_entries(&imap, &[]);
    }

    #[test]
    fn put_get_works() {
        let mut imap = <LazyIndexMap<u8>>::new();
        // Put some values.
        assert_eq!(imap.put_get(1, Some(b'A')), None);
        assert_eq!(imap.put_get(2, Some(b'B')), None);
        assert_eq!(imap.put_get(4, Some(b'C')), None);
        assert_cached_entries(
            &imap,
            &[
                (1, StorageEntry::new(Some(b'A'), EntryState::Mutated)),
                (2, StorageEntry::new(Some(b'B'), EntryState::Mutated)),
                (4, StorageEntry::new(Some(b'C'), EntryState::Mutated)),
            ],
        );
        // Put none values.
        assert_eq!(imap.put_get(3, None), None);
        assert_eq!(imap.put_get(5, None), None);
        assert_cached_entries(
            &imap,
            &[
                (1, StorageEntry::new(Some(b'A'), EntryState::Mutated)),
                (2, StorageEntry::new(Some(b'B'), EntryState::Mutated)),
                (3, StorageEntry::new(None, EntryState::Preserved)),
                (4, StorageEntry::new(Some(b'C'), EntryState::Mutated)),
                (5, StorageEntry::new(None, EntryState::Preserved)),
            ],
        );
        // Override some values with none.
        assert_eq!(imap.put_get(2, None), Some(b'B'));
        assert_eq!(imap.put_get(4, None), Some(b'C'));
        assert_cached_entries(
            &imap,
            &[
                (1, StorageEntry::new(Some(b'A'), EntryState::Mutated)),
                (2, StorageEntry::new(None, EntryState::Mutated)),
                (3, StorageEntry::new(None, EntryState::Preserved)),
                (4, StorageEntry::new(None, EntryState::Mutated)),
                (5, StorageEntry::new(None, EntryState::Preserved)),
            ],
        );
        // Override none values with some.
        assert_eq!(imap.put_get(3, Some(b'X')), None);
        assert_eq!(imap.put_get(5, Some(b'Y')), None);
        assert_cached_entries(
            &imap,
            &[
                (1, StorageEntry::new(Some(b'A'), EntryState::Mutated)),
                (2, StorageEntry::new(None, EntryState::Mutated)),
                (3, StorageEntry::new(Some(b'X'), EntryState::Mutated)),
                (4, StorageEntry::new(None, EntryState::Mutated)),
                (5, StorageEntry::new(Some(b'Y'), EntryState::Mutated)),
            ],
        );
    }

    #[test]
    fn get_works() {
        let mut imap = <LazyIndexMap<u8>>::new();
        let nothing_changed = &[
            (1, StorageEntry::new(None, EntryState::Preserved)),
            (2, StorageEntry::new(Some(b'B'), EntryState::Mutated)),
            (3, StorageEntry::new(None, EntryState::Preserved)),
            (4, StorageEntry::new(Some(b'D'), EntryState::Mutated)),
        ];
        // Put some values.
        assert_eq!(imap.put_get(1, None), None);
        assert_eq!(imap.put_get(2, Some(b'B')), None);
        assert_eq!(imap.put_get(3, None), None);
        assert_eq!(imap.put_get(4, Some(b'D')), None);
        assert_cached_entries(&imap, nothing_changed);
        // `get` works:
        assert_eq!(imap.get(1), None);
        assert_eq!(imap.get(2), Some(&b'B'));
        assert_eq!(imap.get(3), None);
        assert_eq!(imap.get(4), Some(&b'D'));
        assert_cached_entries(&imap, nothing_changed);
        // `get_mut` works:
        assert_eq!(imap.get_mut(1), None);
        assert_eq!(imap.get_mut(2), Some(&mut b'B'));
        assert_eq!(imap.get_mut(3), None);
        assert_eq!(imap.get_mut(4), Some(&mut b'D'));
        assert_cached_entries(&imap, nothing_changed);
        // `get` or `get_mut` without cache:
        assert_eq!(imap.get(5), None);
        assert_eq!(imap.get_mut(5), None);
    }

    #[test]
    fn put_works() {
        let mut imap = <LazyIndexMap<u8>>::new();
        // Put some values.
        imap.put(1, None);
        imap.put(2, Some(b'B'));
        imap.put(4, None);
        // The main difference between `put` and `put_get` is that `put` never
        // loads from storage which also has one drawback: Putting a `None`
        // value always ends-up in `Mutated` state for the entry even if the
        // entry is already `None`.
        assert_cached_entries(
            &imap,
            &[
                (1, StorageEntry::new(None, EntryState::Mutated)),
                (2, StorageEntry::new(Some(b'B'), EntryState::Mutated)),
                (4, StorageEntry::new(None, EntryState::Mutated)),
            ],
        );
        // Overwrite entries:
        imap.put(1, Some(b'A'));
        imap.put(2, None);
        assert_cached_entries(
            &imap,
            &[
                (1, StorageEntry::new(Some(b'A'), EntryState::Mutated)),
                (2, StorageEntry::new(None, EntryState::Mutated)),
                (4, StorageEntry::new(None, EntryState::Mutated)),
            ],
        );
    }

    #[test]
    fn swap_works() {
        let mut imap = <LazyIndexMap<u8>>::new();
        let nothing_changed = &[
            (1, StorageEntry::new(Some(b'A'), EntryState::Mutated)),
            (2, StorageEntry::new(Some(b'B'), EntryState::Mutated)),
            (3, StorageEntry::new(None, EntryState::Preserved)),
            (4, StorageEntry::new(None, EntryState::Preserved)),
        ];
        // Put some values.
        assert_eq!(imap.put_get(1, Some(b'A')), None);
        assert_eq!(imap.put_get(2, Some(b'B')), None);
        assert_eq!(imap.put_get(3, None), None);
        assert_eq!(imap.put_get(4, None), None);
        assert_cached_entries(&imap, nothing_changed);
        // Swap same indices: Check that nothing has changed.
        for i in 0..4 {
            imap.swap(i, i);
        }
        assert_cached_entries(&imap, nothing_changed);
        // Swap `None` values: Check that nothing has changed.
        imap.swap(3, 4);
        imap.swap(4, 3);
        assert_cached_entries(&imap, nothing_changed);
        // Swap `Some` and `None`:
        imap.swap(1, 3);
        assert_cached_entries(
            &imap,
            &[
                (1, StorageEntry::new(None, EntryState::Mutated)),
                (2, StorageEntry::new(Some(b'B'), EntryState::Mutated)),
                (3, StorageEntry::new(Some(b'A'), EntryState::Mutated)),
                (4, StorageEntry::new(None, EntryState::Preserved)),
            ],
        );
        // Swap `Some` and `Some`:
        imap.swap(2, 3);
        assert_cached_entries(
            &imap,
            &[
                (1, StorageEntry::new(None, EntryState::Mutated)),
                (2, StorageEntry::new(Some(b'A'), EntryState::Mutated)),
                (3, StorageEntry::new(Some(b'B'), EntryState::Mutated)),
                (4, StorageEntry::new(None, EntryState::Preserved)),
            ],
        );
        // Swap out of bounds: `None` and `None`
        imap.swap(4, 5);
        assert_cached_entries(
            &imap,
            &[
                (1, StorageEntry::new(None, EntryState::Mutated)),
                (2, StorageEntry::new(Some(b'A'), EntryState::Mutated)),
                (3, StorageEntry::new(Some(b'B'), EntryState::Mutated)),
                (4, StorageEntry::new(None, EntryState::Preserved)),
                (5, StorageEntry::new(None, EntryState::Preserved)),
            ],
        );
        // Swap out of bounds: `Some` and `None`
        imap.swap(3, 6);
        assert_cached_entries(
            &imap,
            &[
                (1, StorageEntry::new(None, EntryState::Mutated)),
                (2, StorageEntry::new(Some(b'A'), EntryState::Mutated)),
                (3, StorageEntry::new(None, EntryState::Mutated)),
                (4, StorageEntry::new(None, EntryState::Preserved)),
                (5, StorageEntry::new(None, EntryState::Preserved)),
                (6, StorageEntry::new(Some(b'B'), EntryState::Mutated)),
            ],
        );
    }

    #[test]
    fn spread_layout_works() -> ink_env::Result<()> {
        ink_env::test::run_test::<ink_env::DefaultEnvironment, _>(|_| {
            let mut imap = <LazyIndexMap<u8>>::new();
            let nothing_changed = &[
                (1, StorageEntry::new(Some(b'A'), EntryState::Mutated)),
                (2, StorageEntry::new(Some(b'B'), EntryState::Mutated)),
                (3, StorageEntry::new(None, EntryState::Preserved)),
                (4, StorageEntry::new(None, EntryState::Preserved)),
            ];
            // Put some values.
            assert_eq!(imap.put_get(1, Some(b'A')), None);
            assert_eq!(imap.put_get(2, Some(b'B')), None);
            assert_eq!(imap.put_get(3, None), None);
            assert_eq!(imap.put_get(4, None), None);
            assert_cached_entries(&imap, nothing_changed);
            // Push the lazy index map onto the contract storage and then load
            // another instance of it from the contract stoarge.
            // Then: Compare both instances to be equal.
            let root_key = Key::from([0x42; 32]);
            SpreadLayout::push_spread(&imap, &mut KeyPtr::from(root_key));
            let imap2 = <LazyIndexMap<u8> as SpreadLayout>::pull_spread(
                &mut KeyPtr::from(root_key),
            );
            assert_cached_entries(&imap2, &[]);
            assert_eq!(imap2.get(1), Some(&b'A'));
            assert_eq!(imap2.get(2), Some(&b'B'));
            assert_eq!(imap2.get(3), None);
            assert_eq!(imap2.get(4), None);
            assert_cached_entries(
                &imap2,
                &[
                    (1, StorageEntry::new(Some(b'A'), EntryState::Preserved)),
                    (2, StorageEntry::new(Some(b'B'), EntryState::Preserved)),
                    (3, StorageEntry::new(None, EntryState::Preserved)),
                    (4, StorageEntry::new(None, EntryState::Preserved)),
                ],
            );
            // Clear the first lazy index map instance and reload another instance
            // to check whether the associated storage has actually been freed
            // again:
            SpreadLayout::clear_spread(&imap2, &mut KeyPtr::from(root_key));
            // The above `clear_spread` call is a no-op since lazy index map is
            // generally not aware of its associated elements. So we have to
            // manually clear them from the contract storage which is what the
            // high-level data structures like `storage::Vec` would command:
            imap2.clear_packed_at(1);
            imap2.clear_packed_at(2);
            imap2.clear_packed_at(3); // Not really needed here.
            imap2.clear_packed_at(4); // Not really needed here.
            let imap3 = <LazyIndexMap<u8> as SpreadLayout>::pull_spread(
                &mut KeyPtr::from(root_key),
            );
            assert_cached_entries(&imap3, &[]);
            assert_eq!(imap3.get(1), None);
            assert_eq!(imap3.get(2), None);
            assert_eq!(imap3.get(3), None);
            assert_eq!(imap3.get(4), None);
            assert_cached_entries(
                &imap3,
                &[
                    (1, StorageEntry::new(None, EntryState::Preserved)),
                    (2, StorageEntry::new(None, EntryState::Preserved)),
                    (3, StorageEntry::new(None, EntryState::Preserved)),
                    (4, StorageEntry::new(None, EntryState::Preserved)),
                ],
            );
            Ok(())
        })
    }
}
