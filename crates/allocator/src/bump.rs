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

//! A simple bump allocator.
//!
//! It's goal to have a much smaller footprint than the admittedly more full-featured `wee_alloc`
//! allocator which is currently being used by ink! smart contracts.
//!
//! The heap which will be used by this allocator is a single page of memory, which in Wasm is
//! 64KiB. We do not expect contracts to use more memory than this (for now), so we will throw an
//! OOM error instead of requesting more memory.

use core::alloc::{
    GlobalAlloc,
    Layout,
};

/// A page in Wasm is 64KiB
const PAGE_SIZE: usize = 64 * 1024;

static mut INNER: InnerAlloc = InnerAlloc::new();

pub struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        INNER.alloc(layout)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

struct InnerAlloc {
    /// Points to the start of the next available allocation.
    ///
    /// If the heap hasn't been initialized yet this value will be `None`.
    next: Option<usize>,
}

impl InnerAlloc {
    pub const fn new() -> Self {
        Self { next: None }
    }

    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        // TODO: Figure out how to properly initalize the heap
        let alloc_start = if let Some(start) = self.next {
            start;
        } else {
            let prev_page = core::arch::wasm32::memory_grow(0, 1);
            if prev_page == usize::max_value() {
                panic!("OOM")
            }
            prev_page.checked_mul(PAGE_SIZE).expect("OOM")
        };

        let aligned_layout = layout.pad_to_align();
        let alloc_end = match alloc_start.checked_add(aligned_layout.size()) {
            Some(end) => end,
            None => return core::ptr::null_mut(),
        };

        // Since we're using a single page as our entire heap if we exceed it we're effectively
        // out-of-memory.
        if alloc_end > PAGE_SIZE {
            return core::ptr::null_mut()
        }

        self.next = Some(alloc_end);
        alloc_start as *mut u8
    }
}