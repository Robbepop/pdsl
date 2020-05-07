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
    collections::extend_lifetime,
    lazy::LazyArrayLength,
    traits::PackedLayout,
};

/// An iterator over shared references to the elements of a small storage vector.
#[derive(Debug, Clone, Copy)]
pub struct Iter<'a, T, N>
where
    N: LazyArrayLength<T>,
{
    /// The storage vector to iterate over.
    vec: &'a SmallVec<T, N>,
    /// The current begin of the iteration.
    begin: u32,
    /// The current end of the iteration.
    end: u32,
}

impl<'a, T, N> Iter<'a, T, N>
where
    N: LazyArrayLength<T>,
{
    /// Creates a new iterator for the given storage vector.
    pub(crate) fn new(vec: &'a SmallVec<T, N>) -> Self {
        Self {
            vec,
            begin: 0,
            end: vec.len(),
        }
    }

    /// Returns the amount of remaining elements to yield by the iterator.
    fn remaining(&self) -> u32 {
        self.end - self.begin
    }
}

impl<'a, T, N> Iterator for Iter<'a, T, N>
where
    T: PackedLayout,
    N: LazyArrayLength<T>,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        <Self as Iterator>::nth(self, 0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining() as usize;
        (remaining, Some(remaining))
    }

    fn count(self) -> usize {
        self.remaining() as usize
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        debug_assert!(self.begin <= self.end);
        let n = n as u32;
        if self.begin + n >= self.end {
            return None
        }
        let cur = self.begin + n;
        self.begin += 1 + n;
        self.vec.get(cur).expect("access is within bounds").into()
    }
}

impl<'a, T, N> ExactSizeIterator for Iter<'a, T, N>
where
    T: PackedLayout,
    N: LazyArrayLength<T>,
{
}

impl<'a, T, N> DoubleEndedIterator for Iter<'a, T, N>
where
    T: PackedLayout,
    N: LazyArrayLength<T>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        <Self as DoubleEndedIterator>::nth_back(self, 0)
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        debug_assert!(self.begin <= self.end);
        let n = n as u32;
        if self.begin >= self.end.saturating_sub(n) {
            return None
        }
        self.end -= 1 + n;
        self.vec
            .get(self.end)
            .expect("access is within bounds")
            .into()
    }
}

/// An iterator over exclusive references to the elements of a small storage vector.
#[derive(Debug)]
pub struct IterMut<'a, T, N>
where
    N: LazyArrayLength<T>,
{
    /// The storage vector to iterate over.
    vec: &'a mut SmallVec<T, N>,
    /// The current begin of the iteration.
    begin: u32,
    /// The current end of the iteration.
    end: u32,
}

impl<'a, T, N> IterMut<'a, T, N>
where
    N: LazyArrayLength<T>,
{
    /// Creates a new iterator for the given storage vector.
    pub(crate) fn new(vec: &'a mut SmallVec<T, N>) -> Self {
        let len = vec.len();
        Self {
            vec,
            begin: 0,
            end: len,
        }
    }

    /// Returns the amount of remaining elements to yield by the iterator.
    fn remaining(&self) -> u32 {
        self.end - self.begin
    }
}

impl<'a, T, N> IterMut<'a, T, N>
where
    T: PackedLayout,
    N: LazyArrayLength<T>,
{
    fn get_mut<'b>(&'b mut self, at: u32) -> Option<&'a mut T> {
        self.vec.get_mut(at).map(|value| {
            // SAFETY: We extend the lifetime of the reference here.
            //
            //         This is safe because the iterator yields an exclusive
            //         reference to every element in the iterated vector
            //         just once and also there can be only one such iterator
            //         for the same vector at the same time which is
            //         guaranteed by the constructor of the iterator.
            unsafe { extend_lifetime::<'b, 'a, T>(value) }
        })
    }
}

impl<'a, T, N> Iterator for IterMut<'a, T, N>
where
    T: PackedLayout,
    N: LazyArrayLength<T>,
{
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        <Self as Iterator>::nth(self, 0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining() as usize;
        (remaining, Some(remaining))
    }

    fn count(self) -> usize {
        self.remaining() as usize
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        debug_assert!(self.begin <= self.end);
        let n = n as u32;
        if self.begin + n >= self.end {
            return None
        }
        let cur = self.begin + n;
        self.begin += 1 + n;
        self.get_mut(cur).expect("access is within bounds").into()
    }
}

impl<'a, T, N> ExactSizeIterator for IterMut<'a, T, N>
where
    T: PackedLayout,
    N: LazyArrayLength<T>,
{
}

impl<'a, T, N> DoubleEndedIterator for IterMut<'a, T, N>
where
    T: PackedLayout,
    N: LazyArrayLength<T>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        <Self as DoubleEndedIterator>::nth_back(self, 0)
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        debug_assert!(self.begin <= self.end);
        let n = n as u32;
        if self.begin >= self.end.saturating_sub(n) {
            return None
        }
        self.end -= 1 + n;
        self.get_mut(self.end)
            .expect("access is within bounds")
            .into()
    }
}
