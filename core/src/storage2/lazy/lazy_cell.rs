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
    CacheCell,
    Entry,
    EntryState,
};
use crate::storage2::traits::{
    clear_spread_root_opt,
    pull_spread_root_opt,
    ExtKeyPtr,
    KeyPtr,
    SpreadLayout,
};
use core::{
    fmt,
    fmt::Debug,
    ptr::NonNull,
};
use ink_primitives::Key;

/// A lazy storage entity.
///
/// This loads its value from storage upon first use.
///
/// # Note
///
/// Use this if the storage field doesn't need to be loaded in some or most cases.
pub struct LazyCell<T>
where
    T: SpreadLayout,
{
    /// The key to lazily load the value from.
    ///
    /// # Note
    ///
    /// This can be `None` on contract initialization where a `LazyCell` is
    /// normally initialized given a concrete value.
    key: Option<Key>,
    /// The low-level cache for the lazily loaded storage value.
    ///
    /// # Safety (Dev)
    ///
    /// We use `UnsafeCell` instead of `RefCell` because
    /// the intended use-case is to hand out references (`&` and `&mut`)
    /// to the callers of `Lazy`. This cannot be done without `unsafe`
    /// code even with `RefCell`. Also `RefCell` has a larger memory footprint
    /// and has additional overhead that we can avoid by the interface
    /// and the fact that ink! code is always run single-threaded.
    /// Being efficient is important here because this is intended to be
    /// a low-level primitive with lots of dependencies.
    cache: CacheCell<Option<Entry<T>>>,
}

impl<T> Debug for LazyCell<T>
where
    T: Debug + SpreadLayout,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("LazyCell")
            .field("key", &self.key)
            .field("cache", self.cache.as_inner())
            .finish()
    }
}

#[test]
fn debug_impl_works() {
    let c1 = <LazyCell<i32>>::new(None);
    assert_eq!(
        format!("{:?}", &c1),
        "LazyCell { key: None, cache: Some(Entry { value: None, state: Mutated }) }",
    );
    let c2 = <LazyCell<i32>>::new(Some(42));
    assert_eq!(
        format!("{:?}", &c2),
        "LazyCell { key: None, cache: Some(Entry { value: Some(42), state: Mutated }) }",
    );
    let c3 = <LazyCell<i32>>::lazy(Key::from([0x00; 32]));
    assert_eq!(
        format!("{:?}", &c3),
        "LazyCell { \
            key: Some(Key(0x_\
                0000000000000000_\
                0000000000000000_\
                0000000000000000_\
                0000000000000000)\
            ), \
            cache: None \
        }",
    );
}

impl<T> Drop for LazyCell<T>
where
    T: SpreadLayout,
{
    fn drop(&mut self) {
        if let Some(key) = self.key() {
            if let Some(entry) = self.entry() {
                clear_spread_root_opt::<T, _>(key, || entry.value().into())
            }
        }
    }
}

#[cfg(feature = "std")]
const _: () = {
    use crate::storage2::traits::StorageLayout;
    use ink_abi::layout2::Layout;

    impl<T> StorageLayout for LazyCell<T>
    where
        T: StorageLayout + SpreadLayout,
    {
        fn layout(key_ptr: &mut KeyPtr) -> Layout {
            <T as StorageLayout>::layout(key_ptr)
        }
    }
};

impl<T> SpreadLayout for LazyCell<T>
where
    T: SpreadLayout,
{
    const FOOTPRINT: u64 = <T as SpreadLayout>::FOOTPRINT;

    fn pull_spread(ptr: &mut KeyPtr) -> Self {
        Self::lazy(*KeyPtr::next_for::<T>(ptr))
    }

    fn push_spread(&self, ptr: &mut KeyPtr) {
        if let Some(entry) = self.entry() {
            SpreadLayout::push_spread(entry, ptr)
        }
    }

    fn clear_spread(&self, ptr: &mut KeyPtr) {
        if let Some(entry) = self.entry() {
            SpreadLayout::clear_spread(entry, ptr)
        }
    }
}

// # Developer Note
//
// Implementing PackedLayout for LazyCell is not useful since that would
// potentially allow overlapping distinct LazyCell instances by pulling
// from the same underlying storage cell.
//
// If a user wants a packed LazyCell they can instead pack its inner type.

impl<T> From<T> for LazyCell<T>
where
    T: SpreadLayout,
{
    fn from(value: T) -> Self {
        Self::new(Some(value))
    }
}

impl<T> Default for LazyCell<T>
where
    T: Default + SpreadLayout,
{
    fn default() -> Self {
        Self::new(Some(Default::default()))
    }
}

impl<T> LazyCell<T>
where
    T: SpreadLayout,
{
    /// Creates an already populated lazy storage cell.
    ///
    /// # Note
    ///
    /// Since this already has a value it will never actually load from
    /// the contract storage.
    #[must_use]
    pub fn new(value: Option<T>) -> Self {
        Self {
            key: None,
            cache: CacheCell::new(Some(Entry::new(value, EntryState::Mutated))),
        }
    }

    /// Creates a lazy storage cell for the given key.
    ///
    /// # Note
    ///
    /// This will actually lazily load from the associated storage cell
    /// upon access.
    #[must_use]
    pub fn lazy(key: Key) -> Self {
        Self {
            key: Some(key),
            cache: CacheCell::new(None),
        }
    }

    /// Returns the lazy key if any.
    ///
    /// # Note
    ///
    /// The key is `None` if the `LazyCell` has been initialized as a value.
    /// This generally only happens in ink! constructors.
    fn key(&self) -> Option<&Key> {
        self.key.as_ref()
    }

    /// Returns the cached entry.
    fn entry(&self) -> Option<&Entry<T>> {
        self.cache.as_inner().as_ref()
    }
}

impl<T> LazyCell<T>
where
    T: SpreadLayout,
{
    /// Loads the storage entry.
    ///
    /// Tries to load the entry from cache and falls back to lazily load the
    /// entry from the contract storage.
    unsafe fn load_through_cache(&self) -> NonNull<Entry<T>> {
        // SAFETY: This is critical because we mutably access the entry.
        //         However, we mutate the entry only if it is vacant.
        //         If the entry is occupied by a value we return early.
        //         This way we do not invalidate pointers to this value.
        let cache = &mut *self.cache.get_ptr().as_ptr();
        if cache.is_none() {
            // Load value from storage and then return the cached entry.
            let value = self
                .key
                .map(|key| pull_spread_root_opt::<T>(&key))
                .unwrap_or(None);
            *cache = Some(Entry::new(value, EntryState::Preserved));
        }
        debug_assert!(cache.is_some());
        NonNull::from(cache.as_mut().expect("unpopulated cache entry"))
    }

    /// Returns a shared reference to the entry.
    fn load_entry(&self) -> &Entry<T> {
        // SAFETY: We load the entry either from cache of from contract storage.
        //
        //         This is safe because we are just returning a shared reference
        //         from within a `&self` method. This also cannot change the
        //         loaded value and thus cannot change the `mutate` flag of the
        //         entry. Aliases using this method are safe since ink! is
        //         single-threaded.
        unsafe { &*self.load_through_cache().as_ptr() }
    }

    /// Returns an exclusive reference to the entry.
    fn load_entry_mut(&mut self) -> &mut Entry<T> {
        // SAFETY: We load the entry either from cache of from contract storage.
        //
        //         This is safe because we are just returning an exclusive reference
        //         from within a `&mut self` method. This may change the
        //         loaded value and thus the `mutate` flag of the entry is set.
        //         Aliases cannot happen through this method since ink! is
        //         single-threaded.
        let entry = unsafe { &mut *self.load_through_cache().as_ptr() };
        entry.replace_state(EntryState::Mutated);
        entry
    }

    /// Returns a shared reference to the value.
    ///
    /// # Note
    ///
    /// This eventually lazily loads the value from the contract storage.
    ///
    /// # Panics
    ///
    /// If decoding the loaded value to `T` failed.
    #[must_use]
    pub fn get(&self) -> Option<&T> {
        self.load_entry().value().into()
    }

    /// Returns an exclusive reference to the value.
    ///
    /// # Note
    ///
    /// This eventually lazily loads the value from the contract storage.
    ///
    /// # Panics
    ///
    /// If decoding the loaded value to `T` failed.
    #[must_use]
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.load_entry_mut().value_mut().into()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Entry,
        EntryState,
        LazyCell,
    };
    use crate::{
        env,
        env::test::run_test,
        storage2::traits::{
            KeyPtr,
            SpreadLayout,
        },
    };
    use ink_primitives::Key;

    #[test]
    fn new_works() {
        // Initialized via some value:
        let mut a = <LazyCell<u8>>::new(Some(b'A'));
        assert_eq!(a.key(), None);
        assert_eq!(
            a.entry(),
            Some(&Entry::new(Some(b'A'), EntryState::Mutated))
        );
        assert_eq!(a.get(), Some(&b'A'));
        assert_eq!(a.get_mut(), Some(&mut b'A'));
        // Initialized as none:
        let mut b = <LazyCell<u8>>::new(None);
        assert_eq!(b.key(), None);
        assert_eq!(b.entry(), Some(&Entry::new(None, EntryState::Mutated)));
        assert_eq!(b.get(), None);
        assert_eq!(b.get_mut(), None);
        // Same as default or from:
        let default_lc = <LazyCell<u8>>::default();
        let from_lc = LazyCell::from(u8::default());
        let new_lc = LazyCell::new(Some(u8::default()));
        assert_eq!(default_lc.get(), from_lc.get());
        assert_eq!(from_lc.get(), new_lc.get());
        assert_eq!(new_lc.get(), Some(&u8::default()));
    }

    #[test]
    fn lazy_works() {
        let root_key = Key::from([0x42; 32]);
        let cell = <LazyCell<u8>>::lazy(root_key);
        assert_eq!(cell.key(), Some(&root_key));
    }

    #[test]
    fn lazy_get_works() -> env::Result<()> {
        run_test::<env::DefaultEnvTypes, _>(|_| {
            let cell = <LazyCell<u8>>::lazy(Key::from([0x42; 32]));
            let value = cell.get();
            // We do the normally unreachable check in order to have an easier
            // time finding the issue if the above execution did not panic.
            assert_eq!(value, None);
            Ok(())
        })
    }

    #[test]
    fn get_mut_works() {
        let mut cell = <LazyCell<i32>>::new(Some(1));
        assert_eq!(cell.get(), Some(&1));
        *cell.get_mut().unwrap() += 1;
        assert_eq!(cell.get(), Some(&2));
    }

    #[test]
    fn spread_layout_works() -> env::Result<()> {
        run_test::<env::DefaultEnvTypes, _>(|_| {
            let cell_a0 = <LazyCell<u8>>::new(Some(b'A'));
            assert_eq!(cell_a0.get(), Some(&b'A'));
            // Push `cell_a0` to the contract storage.
            // Then, pull `cell_a1` from the contract storage and check if it is
            // equal to `cell_a0`.
            let root_key = Key::from([0x42; 32]);
            SpreadLayout::push_spread(&cell_a0, &mut KeyPtr::from(root_key));
            let cell_a1 =
                <LazyCell<u8> as SpreadLayout>::pull_spread(&mut KeyPtr::from(root_key));
            assert_eq!(cell_a1.get(), cell_a0.get());
            assert_eq!(cell_a1.get(), Some(&b'A'));
            assert_eq!(
                cell_a1.entry(),
                Some(&Entry::new(Some(b'A'), EntryState::Preserved))
            );
            // Also test if a lazily instantiated cell works:
            let cell_a2 = <LazyCell<u8>>::lazy(root_key);
            assert_eq!(cell_a2.get(), cell_a0.get());
            assert_eq!(cell_a2.get(), Some(&b'A'));
            assert_eq!(
                cell_a2.entry(),
                Some(&Entry::new(Some(b'A'), EntryState::Preserved))
            );
            // Test if clearing works:
            SpreadLayout::clear_spread(&cell_a1, &mut KeyPtr::from(root_key));
            let cell_a3 = <LazyCell<u8>>::lazy(root_key);
            assert_eq!(cell_a3.get(), None);
            assert_eq!(
                cell_a3.entry(),
                Some(&Entry::new(None, EntryState::Preserved))
            );
            Ok(())
        })
    }
}
