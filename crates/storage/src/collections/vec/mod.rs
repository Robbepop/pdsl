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

//! A storage vector used to store elements in a contiguous sequenced order.
//!
//! This is by default the go-to collection for most smart contracts if there
//! are no special requirements to the storage data structure.

mod impls;
mod iter;
mod slice;
mod storage;

#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "ink-fuzz-tests"))]
mod fuzz_tests;

pub use self::iter::{
    Iter,
    IterMut,
};
use crate::{
    collections::vec::slice::{
        Slice,
        SliceMut,
    },
    lazy::{
        Lazy,
        LazyIndexMap,
    },
    traits::PackedLayout,
};

/// A contiguous growable array type, written `Vec<T>` but pronounced "vector".
///
/// # Note
///
/// Despite the similarity to Rust's `Vec` type this storage `Vec` has many
/// differences in its internal data layout. While it stores its data in contiguous
/// storage slots this does not mean that the data is actually densely stored
/// in memory.
///
/// Also its technical performance characteristics may be different from Rust's
/// `Vec` due to the differences stated above.
///
/// Allows to store up to `2^32` elements and is guaranteed to not reallocate
/// upon pushing new elements to it.
#[derive(Debug)]
pub struct Vec<T>
where
    T: PackedLayout,
{
    /// The length of the vector.
    len: Lazy<u32>,
    /// The synchronized cells to operate on the contract storage.
    elems: LazyIndexMap<T>,
}

/// The index is out of the bounds of this vector.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IndexOutOfBounds;

impl<T> Default for Vec<T>
where
    T: PackedLayout,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Vec<T>
where
    T: PackedLayout,
{
    /// Creates a new empty storage vector.
    pub fn new() -> Self {
        Self {
            len: Lazy::new(0),
            elems: LazyIndexMap::new(),
        }
    }

    /// Returns the number of elements in the vector, also referred to as its length.
    pub fn len(&self) -> u32 {
        *self.len
    }

    /// Returns `true` if the vector contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T> Vec<T>
where
    T: PackedLayout,
{
    /// Clears the underlying storage cells of the storage vector.
    ///
    /// # Note
    ///
    /// This completely invalidates the storage vector's invariants about
    /// the contents of its associated storage region.
    ///
    /// This API is used for the `Drop` implementation of [`Vec`] as well as
    /// for the [`SpreadLayout::clear_spread`][`crate::traits::SpreadLayout::clear_spread`]
    /// trait implementation.
    fn clear_cells(&self) {
        if self.elems.key().is_none() {
            // We won't clear any storage if we are in lazy state since there
            // probably has not been any state written to storage, yet.
            return
        }
        for index in 0..self.len() {
            self.elems.clear_packed_at(index);
        }
    }
}

impl<T> Vec<T>
where
    T: PackedLayout,
{
    /// Returns an iterator yielding shared references to all elements of the vector.
    ///
    /// # Note
    ///
    /// Avoid unbounded iteration over big storage vectors.
    /// Prefer using methods like `Iterator::take` in order to limit the number
    /// of yielded elements.
    pub fn iter(&self) -> Iter<T> {
        Iter::new(self)
    }

    /// Returns an iterator yielding exclusive references to all elements of the vector.
    ///
    /// # Note
    ///
    /// Avoid unbounded iteration over big storage vectors.
    /// Prefer using methods like `Iterator::take` in order to limit the number
    /// of yielded elements.
    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut::new(self)
    }

    /// Returns the index if it is within bounds or `None` otherwise.
    fn within_bounds(&self, index: u32) -> Option<u32> {
        if index < self.len() {
            return Some(index)
        }
        None
    }

    /// Returns a shared reference to the first element if any.
    pub fn first(&self) -> Option<&T> {
        if self.is_empty() {
            return None
        }
        self.get(0)
    }

    /// Returns a shared reference to the last element if any.
    pub fn last(&self) -> Option<&T> {
        if self.is_empty() {
            return None
        }
        let last_index = self.len() - 1;
        self.get(last_index)
    }

    /// Returns a shared reference to the indexed element.
    ///
    /// Returns `None` if `index` is out of bounds.
    pub fn get(&self, index: u32) -> Option<&T> {
        self.within_bounds(index)
            .and_then(|index| self.elems.get(index))
    }
}

impl<T> Vec<T>
where
    T: PackedLayout,
{
    /// Appends an element to the back of the vector.
    pub fn push(&mut self, value: T) {
        assert!(
            self.len() < core::u32::MAX,
            "cannot push more elements into the storage vector"
        );
        let last_index = self.len();
        *self.len += 1;
        self.elems.put(last_index, Some(value));
    }

    /// Binary searches this sorted vector for a given element.
    ///
    /// If the value is found then [`Result::Ok`] is returned, containing the
    /// index of the matching element. If there are multiple matches, then any
    /// one of the matches could be returned. If the value is not found then
    /// [`Result::Err`] is returned, containing the index where a matching
    /// element could be inserted while maintaining sorted order.
    ///
    /// See also [`binary_search_by`], [`binary_search_by_key`].
    ///
    /// [`binary_search_by`]: Vec::binary_search_by
    /// [`binary_search_by_key`]: Vec::binary_search_by_key
    ///
    /// # Examples
    ///
    /// Looks up a series of four elements. The first is found, with a
    /// uniquely determined position; the second and third are not
    /// found; the fourth could match any position in `[1, 4]`.
    ///
    /// ```
    /// use ink_storage::Vec as StorageVec;
    ///
    /// let s: StorageVec<i32> = [0, 1, 1, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55]
    ///     .into_iter()
    ///     .copied()
    ///     .collect();
    ///
    /// assert_eq!(s.binary_search(&13),  Ok(9));
    /// assert_eq!(s.binary_search(&4),   Err(7));
    /// assert_eq!(s.binary_search(&100), Err(13));
    /// let r = s.binary_search(&1);
    /// assert!(match r { Ok(1..=4) => true, _ => false, });
    /// ```
    #[inline]
    pub fn binary_search(&self, x: &T) -> Result<u32, u32>
    where
        T: Ord,
    {
        self.binary_search_by(|p| p.cmp(x))
    }

    /// Binary searches this sorted vector with a comparator function.
    ///
    /// The comparator function should implement an order consistent
    /// with the sort order of the underlying vector, returning an
    /// order code that indicates whether its argument is `Less`,
    /// `Equal` or `Greater` the desired target.
    ///
    /// If the value is found then [`Result::Ok`] is returned, containing the
    /// index of the matching element. If there are multiple matches, then any
    /// one of the matches could be returned. If the value is not found then
    /// [`Result::Err`] is returned, containing the index where a matching
    /// element could be inserted while maintaining sorted order.
    ///
    /// See also [`binary_search`], [`binary_search_by_key`].
    ///
    /// [`binary_search`]: Vec::binary_search
    /// [`binary_search_by_key`]: Vec::binary_search_by_key
    ///
    /// # Examples
    ///
    /// Looks up a series of four elements. The first is found, with a
    /// uniquely determined position; the second and third are not
    /// found; the fourth could match any position in `[1, 4]`.
    ///
    /// ```
    /// use ink_storage::Vec as StorageVec;
    ///
    /// let s: StorageVec<i32> = [0, 1, 1, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55]
    ///     .into_iter()
    ///     .copied()
    ///     .collect();
    ///
    /// let seek = 13;
    /// assert_eq!(s.binary_search_by(|probe| probe.cmp(&seek)), Ok(9));
    /// let seek = 4;
    /// assert_eq!(s.binary_search_by(|probe| probe.cmp(&seek)), Err(7));
    /// let seek = 100;
    /// assert_eq!(s.binary_search_by(|probe| probe.cmp(&seek)), Err(13));
    /// let seek = 1;
    /// let r = s.binary_search_by(|probe| probe.cmp(&seek));
    /// assert!(match r { Ok(1..=4) => true, _ => false, });
    /// ```
    // The binary_search implementation is ported from
    // https://github.com/rust-lang/rust/blob/c5e344f7747dbd7e7d4b209e3c480deb5979a56f/library/core/src/slice/mod.rs#L2191
    // and attempts to remain as close to the source as possible.
    #[inline]
    pub fn binary_search_by<'a, F>(&'a self, mut f: F) -> Result<u32, u32>
    where
        F: FnMut(&'a T) -> core::cmp::Ordering,
    {
        use core::cmp::Ordering::*;

        let mut size = self.len();
        let mut left = 0;
        let mut right = size;
        while left < right {
            let mid = left + size / 2;

            // the call is made safe by the following invariants:
            // - `mid >= 0`
            // - `mid < size`: `mid` is limited by `[left; right)` bound.
            let cmp = f(&self[mid]);

            // The reason why we use if/else control flow rather than match
            // is because match reorders comparison operations, which is perf sensitive.
            if cmp == Less {
                left = mid + 1;
            } else if cmp == Greater {
                right = mid;
            } else {
                return Ok(mid)
            }

            size = right - left;
        }
        Err(left)
    }

    /// Binary searches this sorted vector with a key extraction function.
    ///
    /// If the value is found then [`Result::Ok`] is returned, containing the
    /// index of the matching element. If there are multiple matches, then any
    /// one of the matches could be returned. If the value is not found then
    /// [`Result::Err`] is returned, containing the index where a matching
    /// element could be inserted while maintaining sorted order.
    ///
    /// See also [`binary_search`], [`binary_search_by`].
    ///
    /// [`binary_search`]: Vec::binary_search
    /// [`binary_search_by`]: Vec::binary_search_by
    ///
    /// # Examples
    ///
    /// Looks up a series of four elements in a vector of pairs sorted by
    /// their second elements. The first is found, with a uniquely
    /// determined position; the second and third are not found; the
    /// fourth could match any position in `[1, 4]`.
    ///
    /// ```
    /// use ink_storage::Vec as StorageVec;
    ///
    /// let s: StorageVec<(i32, i32)> = [
    ///     (0, 0),
    ///     (2, 1),
    ///     (4, 1),
    ///     (5, 1),
    ///     (3, 1),
    ///     (1, 2),
    ///     (2, 3),
    ///     (4, 5),
    ///     (5, 8),
    ///     (3, 13),
    ///     (1, 21),
    ///     (2, 34),
    ///     (4, 55),
    /// ]
    /// .into_iter()
    /// .copied()
    /// .collect();
    ///
    /// assert_eq!(s.binary_search_by_key(&13, |&(a, b)| b),  Ok(9));
    /// assert_eq!(s.binary_search_by_key(&4, |&(a, b)| b),   Err(7));
    /// assert_eq!(s.binary_search_by_key(&100, |&(a, b)| b), Err(13));
    /// let r = s.binary_search_by_key(&1, |&(a, b)| b);
    /// assert!(match r { Ok(1..=4) => true, _ => false, });
    /// ```
    #[inline]
    pub fn binary_search_by_key<'a, B, F>(&'a self, b: &B, mut f: F) -> Result<u32, u32>
    where
        F: FnMut(&'a T) -> B,
        B: Ord,
    {
        self.binary_search_by(|k| f(k).cmp(b))
    }
}

impl<T> Vec<T>
where
    T: PackedLayout,
{
    /// Pops the last element from the vector and returns it.
    //
    /// Returns `None` if the vector is empty.
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None
        }
        let last_index = self.len() - 1;
        *self.len = last_index;
        self.elems.put_get(last_index, None)
    }

    /// Pops the last element from the vector and immediately drops it.
    ///
    /// Returns `Some(())` if an element has been removed and `None` otherwise.
    ///
    /// # Note
    ///
    /// This operation is a bit more efficient than [`Vec::pop`]
    /// since it avoids reading from contract storage in some use cases.
    pub fn pop_drop(&mut self) -> Option<()> {
        if self.is_empty() {
            return None
        }
        let last_index = self.len() - 1;
        *self.len = last_index;
        self.elems.put(last_index, None);
        Some(())
    }

    /// Returns an exclusive reference to the first element if any.
    pub fn first_mut(&mut self) -> Option<&mut T> {
        if self.is_empty() {
            return None
        }
        self.get_mut(0)
    }

    /// Returns an exclusive reference to the last element if any.
    pub fn last_mut(&mut self) -> Option<&mut T> {
        if self.is_empty() {
            return None
        }
        let last_index = self.len() - 1;
        self.get_mut(last_index)
    }

    /// Returns an exclusive reference to the indexed element.
    ///
    /// Returns `None` if `index` is out of bounds.
    pub fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        self.within_bounds(index)
            .and_then(move |index| self.elems.get_mut(index))
    }

    /// Swaps the elements at the given indices.
    ///
    /// # Panics
    ///
    /// If one or both indices are out of bounds.
    pub fn swap(&mut self, a: u32, b: u32) {
        assert!(
            a < self.len() && b < self.len(),
            "indices are out of bounds"
        );
        self.elems.swap(a, b)
    }

    /// Removes the indexed element from the vector and returns it.
    ///
    /// The last element of the vector is put into the indexed slot.
    /// Returns `None` and does not mutate the vector if the index is out of bounds.
    ///
    /// # Note
    ///
    /// This operation does not preserve ordering but is constant time.
    pub fn swap_remove(&mut self, n: u32) -> Option<T> {
        if self.is_empty() {
            return None
        }
        self.elems.swap(n, self.len() - 1);
        self.pop()
    }

    /// Removes the indexed element from the vector.
    ///
    /// The last element of the vector is put into the indexed slot.
    /// Returns `Some(())` if an element has been removed and `None` otherwise.
    ///
    /// # Note
    ///
    /// This operation should be preferred over [`Vec::swap_remove`] if there is
    /// no need to return the removed element since it avoids a contract storage
    /// read for some use cases.
    pub fn swap_remove_drop(&mut self, n: u32) -> Option<()> {
        if self.is_empty() {
            return None
        }
        self.elems.put(n, None);
        let last_index = self.len() - 1;
        let last = self.elems.put_get(last_index, None);
        self.elems.put(n, last);
        *self.len = last_index;
        Some(())
    }

    /// Sets the elements at the given index to the new value.
    ///
    /// Won't return the old element back to the caller.
    /// Prefer this operation over other method of overriding an element
    /// in the storage vector since this is more efficient.
    #[inline]
    pub fn set(&mut self, index: u32, new_value: T) -> Result<(), IndexOutOfBounds> {
        if self.within_bounds(index).is_none() {
            return Err(IndexOutOfBounds)
        }
        self.elems.put(index, Some(new_value));
        Ok(())
    }

    /// Removes all elements from this vector.
    ///
    /// # Note
    ///
    /// Use this method to clear the vector instead of e.g. iterative `pop()`.
    /// This method performs significantly better and does not actually read
    /// any of the elements (whereas `pop()` does).
    pub fn clear(&mut self) {
        if self.is_empty() {
            return
        }
        for index in 0..self.len() {
            self.elems.put(index, None);
        }
        *self.len = 0;
    }


    #[inline]
    pub fn split_at(&self, mid: u32) -> (Slice<T>, Slice<T>) {
        assert!(mid <= self.len());
        (
            Slice::new_unchecked(0..mid, &self.elems),
            Slice::new_unchecked(mid..self.len(), &self.elems),
        )
    }

    #[inline]
    pub fn split_at_mut(&mut self, mid: u32) -> (SliceMut<T>, SliceMut<T>) {
        assert!(mid <= *self.len);

        // SAFETY: Slice::new_unchecked requires that the ranges do not overlap.
        unsafe {
            (
                SliceMut::new_unchecked(0..mid, &self.elems),
                SliceMut::new_unchecked(mid..(*self.len), &self.elems),
            )
        }
    }
}
