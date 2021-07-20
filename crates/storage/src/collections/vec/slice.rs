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

use crate::{
    lazy::LazyIndexMap,
    traits::PackedLayout,
};
use core::ops::{
    Index,
    IndexMut,
    Range,
};

/// A view into a storage `Vec`.
#[derive(Clone, Debug)]
pub struct Slice<'a, T> {
    /// The start and end indices inside the `index_map`. Indexing the slice using `n` means that we
    /// access `n + range.start`.
    range: Range<u32>,
    index_map: &'a LazyIndexMap<T>,
}

impl<'a, T: PackedLayout> Slice<'a, T> {
    pub fn new_unchecked(
        range: Range<u32>,
        index_map: &'a LazyIndexMap<T>,
    ) -> Slice<'a, T> {
        Slice { range, index_map }
    }

    pub fn get(&self, index: u32) -> Option<&T> {
        let index = self.range.start + index;
        if !self.range.contains(&index) {
            return None
        }
        self.index_map.get(index)
    }
}

impl<'a, T> Index<u32> for Slice<'a, T>
where
    T: PackedLayout,
{
    type Output = T;

    fn index(&self, index: u32) -> &Self::Output {
        self.get(index).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct SliceMut<'a, T> {
    /// The start and end indices inside the `index_map`. Indexing the slice using `n` means that we
    /// access `n + range.start`.
    range: Range<u32>,
    index_map: &'a LazyIndexMap<T>,
}

impl<'a, T: PackedLayout> SliceMut<'a, T> {
    /// Creates a slice.
    ///
    /// # SAFETY
    /// The caller must ensure that no other slices exist that have overlapping indices with the
    /// LazyIndexMap.
    pub unsafe fn new_unchecked(
        range: Range<u32>,
        index_map: &'a LazyIndexMap<T>,
    ) -> SliceMut<'a, T> {
        SliceMut { range, index_map }
    }

    pub fn get(&self, index: u32) -> Option<&T> {
        let index = self.range.start + index;
        if !self.range.contains(&index) {
            return None
        }
        self.index_map.get(index)
    }

    pub fn get_mut(&self, index: u32) -> Option<&mut T> {
        let index = self.range.start + index;
        if !self.range.contains(&index) {
            return None
        }
        // SAFETY:
        //  - lazily_load requires that there is exclusive access to the T. Vec::split_at_mut
        //    guarantees this by ensuring that slices do not overlap.
        //  - lazily_load always returns a valid pointer.
        unsafe {
            self.index_map
                .lazily_load(index)
                .as_mut()
                .value_mut()
                .as_mut()
        }
    }

    pub fn as_slice(&self) -> Slice<T> {
        Slice::new_unchecked(self.range.clone(), self.index_map)
    }
}

impl<'a, T> Index<u32> for SliceMut<'a, T>
where
    T: PackedLayout,
{
    type Output = T;

    fn index(&self, index: u32) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<'a, T> IndexMut<u32> for SliceMut<'a, T>
where
    T: PackedLayout,
{
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}
