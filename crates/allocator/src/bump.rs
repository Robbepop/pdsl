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
//! Its goal to have a much smaller footprint than the admittedly more full-featured `wee_alloc`
//! allocator which is currently being used by ink! smart contracts.
//!
//! The heap which is used by this allocator is built from pages of Wasm memory (each page is `64KiB`).
//! We will request new pages of memory as needed until we run out of memory, at which point we
//! will crash with an `OOM` error instead of freeing any memory.

use core::alloc::{
    GlobalAlloc,
    Layout,
};

/// A page in Wasm is `64KiB`
const PAGE_SIZE: usize = 64 * 1024;

static mut INNER: InnerAlloc = InnerAlloc::new();

/// A bump allocator suitable for use in a Wasm environment.
pub struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match INNER.alloc(layout) {
            Some(start) => start as *mut u8,
            None => core::ptr::null_mut(),
        }
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // A new page in Wasm is guaranteed to already be zero initialized, so we can just use our
        // regular `alloc` call here and save a bit of work.
        //
        // See: https://webassembly.github.io/spec/core/exec/modules.html#growing-memories
        self.alloc(layout)
    }

    #[inline]
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[cfg_attr(feature = "std", derive(Debug, Copy, Clone))]
struct InnerAlloc {
    /// Points to the start of the next available allocation.
    next: usize,

    /// The address of the upper limit of our heap.
    upper_limit: usize,
}

impl InnerAlloc {
    const fn new() -> Self {
        Self {
            next: 0,
            upper_limit: 0,
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(test)] {
            /// Request a `pages` number of page sized sections of Wasm memory. Each page is `64KiB` in size.
            ///
            /// Returns `None` if a page is not available.
            ///
            /// This implementation is only meant to be used for testing, since we cannot (easily)
            /// test the `wasm32` implementation.
            fn request_pages(&mut self, _pages: usize) -> Option<usize> {
                Some(self.upper_limit)
            }
        } else if #[cfg(feature = "std")] {
            fn request_pages(&mut self, _pages: usize) -> Option<usize> {
                unreachable!(
                    "This branch is only used to keep the compiler happy when building tests, and
                     should never actually be called outside of a test run."
                )
            }
        } else if #[cfg(target_arch = "wasm32")] {
            /// Request a `pages` number of pages of Wasm memory. Each page is `64KiB` in size.
            ///
            /// Returns `None` if a page is not available.
            fn request_pages(&mut self, pages: usize) -> Option<usize> {
                let prev_page = core::arch::wasm32::memory_grow(0, pages);
                if prev_page == usize::MAX {
                    return None;
                }

                prev_page.checked_mul(PAGE_SIZE)
            }
        } else {
            compile_error! {
                "ink! only supports compilation as `std` or `no_std` + `wasm32-unknown`"
            }
        }
    }

    /// Tries to allocate enough memory on the heap for the given `Layout`. If there is not enough
    /// room on the heap it'll try and grow it by a page.
    ///
    /// Note: This implementation results in internal fragmentation when allocating across pages.
    fn alloc(&mut self, layout: Layout) -> Option<usize> {
        let alloc_start = self.next;

        let aligned_size = layout.pad_to_align().size();
        let alloc_end = alloc_start.checked_add(aligned_size)?;

        if alloc_end > self.upper_limit {
            let required_pages = required_pages(aligned_size)?;
            let page_start = self.request_pages(required_pages)?;

            self.upper_limit = required_pages
                .checked_mul(PAGE_SIZE)
                .and_then(|pages| page_start.checked_add(pages))?;
            self.next = page_start.checked_add(aligned_size)?;

            Some(page_start)
        } else {
            self.next = alloc_end;
            Some(alloc_start)
        }
    }
}

#[inline]
fn required_pages(size: usize) -> Option<usize> {
    size.checked_add(PAGE_SIZE - 1)
        .and_then(|num| num.checked_div(PAGE_SIZE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_alloc_no_bytes() {
        let mut inner = InnerAlloc::new();

        let layout = Layout::new::<()>();
        assert_eq!(inner.alloc(layout), Some(0));

        let expected_limit =
            PAGE_SIZE * required_pages(layout.pad_to_align().size()).unwrap();
        assert_eq!(inner.upper_limit, expected_limit);

        let expected_alloc_start = std::mem::size_of::<()>();
        assert_eq!(inner.next, expected_alloc_start);
    }

    #[test]
    fn can_alloc_a_byte() {
        let mut inner = InnerAlloc::new();

        let layout = Layout::new::<u8>();
        assert_eq!(inner.alloc(layout), Some(0));

        let expected_limit =
            PAGE_SIZE * required_pages(layout.pad_to_align().size()).unwrap();
        assert_eq!(inner.upper_limit, expected_limit);

        let expected_alloc_start = std::mem::size_of::<u8>();
        assert_eq!(inner.next, expected_alloc_start);
    }

    #[test]
    fn can_alloc_a_foobarbaz() {
        let mut inner = InnerAlloc::new();

        struct FooBarBaz {
            _foo: u32,
            _bar: u128,
            _baz: (u16, bool),
        }

        let layout = Layout::new::<FooBarBaz>();
        let mut total_size = 0;

        let allocations = 3;
        for _ in 0..allocations {
            assert!(inner.alloc(layout).is_some());
            total_size += layout.pad_to_align().size();
        }

        let expected_limit = PAGE_SIZE * required_pages(total_size).unwrap();
        assert_eq!(inner.upper_limit, expected_limit);

        let expected_alloc_start = allocations * std::mem::size_of::<FooBarBaz>();
        assert_eq!(inner.next, expected_alloc_start);
    }

    #[test]
    fn can_alloc_across_pages() {
        let mut inner = InnerAlloc::new();

        struct Foo {
            _foo: [u8; PAGE_SIZE - 1],
        }

        // First, let's allocate a struct which is _almost_ a full page
        let layout = Layout::new::<Foo>();
        assert_eq!(inner.alloc(layout), Some(0));

        let expected_limit =
            PAGE_SIZE * required_pages(layout.pad_to_align().size()).unwrap();
        assert_eq!(inner.upper_limit, expected_limit);

        let expected_alloc_start = std::mem::size_of::<Foo>();
        assert_eq!(inner.next, expected_alloc_start);

        // Now we'll allocate two bytes which will push us over to the next page
        let layout = Layout::new::<u16>();
        assert_eq!(inner.alloc(layout), Some(PAGE_SIZE));

        let expected_limit = 2 * PAGE_SIZE;
        assert_eq!(inner.upper_limit, expected_limit);

        // Notice that we start the allocation on the second page, instead of making use of the
        // remaining byte on the first page
        let expected_alloc_start = PAGE_SIZE + std::mem::size_of::<u16>();
        assert_eq!(inner.next, expected_alloc_start);
    }

    #[test]
    fn can_alloc_multiple_pages() {
        let mut inner = InnerAlloc::new();

        struct Foo {
            _foo: [u8; 2 * PAGE_SIZE],
        }

        let layout = Layout::new::<Foo>();
        assert_eq!(inner.alloc(layout), Some(0));

        let expected_limit =
            PAGE_SIZE * required_pages(layout.pad_to_align().size()).unwrap();
        assert_eq!(inner.upper_limit, expected_limit);

        let expected_alloc_start = std::mem::size_of::<Foo>();
        assert_eq!(inner.next, expected_alloc_start);

        // Now we want to make sure that the state of our allocator is correct for any subsequent
        // allocations
        let layout = Layout::new::<u8>();
        assert_eq!(inner.alloc(layout), Some(2 * PAGE_SIZE));

        let expected_limit = 3 * PAGE_SIZE;
        assert_eq!(inner.upper_limit, expected_limit);

        let expected_alloc_start = 2 * PAGE_SIZE + std::mem::size_of::<u8>();
        assert_eq!(inner.next, expected_alloc_start);
    }

    // TODO: What I want to end up doing is turning this into a `quickcheck` test such that the
    // random sized bytes and the number of allocations comes from `quickcheck`
    #[test]
    fn can_alloc_many_different_sized_chunks() {
        let mut inner = InnerAlloc::new();

        // let v = vec![16, 15, 3, 10, 10, 10, 5, 65];
        let v = vec![32, 30, 6, 20, 20, 20, 10, 130];

        let mut total_bytes_requested = 0;
        let mut expected_alloc_start = 0;
        let mut total_bytes_fragmented = 0;

        for alloc in v {
            let n = alloc * 1024;

            let layout =
                Layout::from_size_align(n, std::mem::size_of::<usize>()).unwrap();
            let size = layout.pad_to_align().size();

            let current_page_limit = required_pages(inner.next).unwrap() * PAGE_SIZE;
            let is_too_big_for_current_page = inner.next + size > current_page_limit;

            if is_too_big_for_current_page && inner.next != 0 {
                let fragmented_in_current_page = current_page_limit % inner.next;
                total_bytes_fragmented += fragmented_in_current_page;

                // We expect our next allocation to be aligned to the start of the next page
                // boundary
                expected_alloc_start = inner.upper_limit;
            }

            assert_eq!(inner.alloc(layout), Some(expected_alloc_start));
            total_bytes_requested += size;

            let pages_required =
                required_pages(total_bytes_requested + total_bytes_fragmented).unwrap();
            let expected_limit = pages_required * PAGE_SIZE;
            assert_eq!(inner.upper_limit, expected_limit);

            expected_alloc_start = total_bytes_requested + total_bytes_fragmented;
            assert_eq!(inner.next, expected_alloc_start);
        }
    }
}

#[cfg(test)]
mod fuzz_tests {
    use super::*;
    use quickcheck::{
        quickcheck,
        TestResult,
    };

    // #[ignore]
    #[quickcheck]
    fn fuzz_should_allocate_random_bytes_that_do_not_overflow(n: usize) -> TestResult {
        // If `n` is going to overflow we don't want to check it here (we'll check the overflow
        // case in another test)
        if n.checked_add(PAGE_SIZE - 1).is_none() {
            return TestResult::discard()
        }

        let mut inner = InnerAlloc::new();

        let layout = Layout::from_size_align(n, std::mem::size_of::<usize>()).unwrap();
        let size = layout.pad_to_align().size();
        assert_eq!(inner.alloc(layout), Some(0));

        let expected_limit = PAGE_SIZE * required_pages(size).unwrap();
        assert_eq!(inner.upper_limit, expected_limit);

        let expected_alloc_start = size;
        assert_eq!(inner.next, expected_alloc_start);

        TestResult::passed()
    }

    // #[ignore]
    #[quickcheck]
    fn fuzz_should_not_allocate_if_it_overflows(n: usize) -> TestResult {
        // In the last test we ignored the overflow case, now we ignore the valid cases
        if n.checked_add(PAGE_SIZE - 1).is_some() {
            return TestResult::discard()
        }

        if let Ok(layout) = Layout::from_size_align(n, std::mem::size_of::<usize>()) {
            let mut inner = InnerAlloc::new();
            assert_eq!(inner.alloc(layout), None);

            TestResult::passed()
        } else {
            // We only want to test cases which can create a valid `Layout`
            TestResult::discard()
        }
    }
}
