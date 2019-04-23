// Copyright 2018-2019 Parity Technologies (UK) Ltd.
// This file is part of ink!.
//
// ink! is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// ink! is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with ink!.  If not, see <http://www.gnu.org/licenses/>.

use super::*;
use crate::storage::{
    self,
    alloc::{
        AllocateUsing,
        Initialize,
    },
    Allocator,
    Flush,
    Key,
};

/// Allocator for dynamic contract storage.
///
/// Uses storage effective bit vectors for free list representation.
/// Searches for free cells and chunks via first-fit approach which
/// can be slow (more than 2 reads) for more than 3000 dynamic allocations
/// at the same time. This is subject to change in the future if
/// experiments show that this is a bottle neck.
#[derive(Debug)]
pub struct DynAlloc {
    /// Bitmap indicating free cell slots.
    free_cells: storage::BitVec,
    /// Bitmap indicating free chunk slots.
    free_chunks: storage::BitVec,
    /// Offset origin key for all cells.
    cells_origin: Key,
    /// Offset origin key for all chunks.
    chunks_origin: Key,
}

impl AllocateUsing for DynAlloc {
    unsafe fn allocate_using<A>(alloc: &mut A) -> Self
    where
        A: Allocate,
    {
        Self {
            free_cells: AllocateUsing::allocate_using(alloc),
            free_chunks: AllocateUsing::allocate_using(alloc),
            cells_origin: alloc.alloc(u32::max_value().into()),
            chunks_origin: alloc.alloc(u32::max_value().into()),
        }
    }
}

impl Initialize for DynAlloc {
    type Args = ();

    fn initialize(&mut self, _args: Self::Args) {
        self.free_cells.initialize(());
        self.free_chunks.initialize(());
    }
}

impl Flush for DynAlloc {
    fn flush(&mut self) {
        self.free_cells.flush();
        self.free_chunks.flush();
    }
}

#[cfg(test)]
impl DynAlloc {
    pub(crate) fn cells_origin(&self) -> Key {
        self.cells_origin
    }

    pub(crate) fn chunks_origin(&self) -> Key {
        self.chunks_origin
    }
}

impl DynAlloc {
    /// Allocates another cell and returns its key.
    fn alloc_cell(&mut self) -> Key {
        let offset = if let Some(free) = self.free_cells.first_set_position() {
            self.free_cells.set(free, false);
            free
        } else {
            let len = self.free_cells.len();
            self.free_cells.push(false);
            len
        };
        self.cells_origin + offset
    }

    /// Allocates another chunk and returns its key.
    fn alloc_chunk(&mut self) -> Key {
        let offset = if let Some(free) = self.free_chunks.first_set_position() {
            self.free_chunks.set(free, false);
            free
        } else {
            let len = self.free_chunks.len();
            self.free_chunks.push(false);
            len
        };
        self.chunks_origin + ((1 << 32) * u64::from(offset))
    }

    /// Deallocates the cell key.
    ///
    /// # Note
    ///
    /// This just frees the associated slot for future allocations.
    fn dealloc_cell(&mut self, key: Key) {
        debug_assert!(key >= self.cells_origin);
        debug_assert!(key < self.cells_origin + self.free_cells.len());
        let position = self.key_to_cell_position(key);
        self.free_cells.set(position, true);
    }

    /// Deallocates the chunk key.
    ///
    /// # Note
    ///
    /// This just frees the associated slot for future allocations.
    fn dealloc_chunk(&mut self, key: Key) {
        debug_assert!(key >= self.chunks_origin);
        debug_assert!(
            key < self.chunks_origin + ((1 << 32) * u64::from(self.free_chunks.len()))
        );
        let position = self.key_to_chunk_position(key);
        self.free_chunks.set(position, true);
    }

    /// Converts a key previously allocated as cell key
    /// back into its offset position.
    fn key_to_cell_position(&self, key: Key) -> u32 {
        let diff = key - self.cells_origin;
        diff.try_to_u32().expect(
            "if allocated by this allocator the key difference can
				 never be greater than u32::MAX; qed",
        )
    }

    /// Converts a key previously allocated as chunk key
    /// back into its offset position.
    fn key_to_chunk_position(&self, key: Key) -> u32 {
        let diff = key - self.chunks_origin;
        let position = diff.try_to_u64().expect(
            "if allocated by this allocator the key difference can
				 never be greater than u64::MAX; qed",
        );
        // Since chunks are always of size 2^32 we need to
        // shift in order to receive the true chunk position.
        (position >> 32) as u32
    }
}

impl Allocate for DynAlloc {
    /// Can only allocate sizes of up to `u32::MAX`.
    fn alloc(&mut self, size: u64) -> Key {
        assert!(size <= u32::max_value().into());
        assert!(size != 0);
        if size == 1 {
            self.alloc_cell()
        } else {
            self.alloc_chunk()
        }
    }
}

impl Allocator for DynAlloc {
    fn dealloc(&mut self, key: Key) {
        // This condition requires cells offset key
        // to be always smaller than chunks offset key.
        //
        // This must either be an invariant or we need
        // another more safe condition in the future.
        if key < self.chunks_origin {
            // The key was allocated as a cell
            self.dealloc_cell(key)
        } else {
            // The key was allocated as a chunk
            self.dealloc_chunk(key)
        }
    }
}
