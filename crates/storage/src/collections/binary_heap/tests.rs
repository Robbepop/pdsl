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
    BinaryHeap,
    PeekMut,
    Reverse,
};
use crate::traits::{
    KeyPtr,
    PackedLayout,
    SpreadLayout,
};
use ink_primitives::Key;

fn heap_from_slice<T>(slice: &[T]) -> BinaryHeap<T>
where
    T: Clone + PackedLayout + Ord,
{
    slice.iter().cloned().collect()
}

/// Creates a heap populated with `n` consecutive values.
fn heap_of_size(n: u32) -> BinaryHeap<u32> {
    std::iter::repeat(0u32)
        .take(n as usize)
        .enumerate()
        .map(|(i, _)| i as u32 + 1)
        .collect()
}

/// Returns the number of cells a binary tree of `heap_size` occupies
/// in the storage.
///
/// *Note*: `heap_size` must be even, if it is odd we cannot calculate
/// the number of cells with certainty, since for e.g. `heap_size = 5`
/// there might be two leaf cells with one element each or alternatively
/// one leaf with two elements.
fn get_count_cells(heap_size: u32) -> u32 {
    fn division_round_up(dividend: u32, divisor: u32) -> u32 {
        (dividend + divisor - 1) / divisor
    }
    assert!(heap_size % 2 == 0, "heap_size must be even");
    let rest = match heap_size {
        0 => 0,
        1 => 0,
        _ => division_round_up(heap_size, super::children::CHILDREN_PER_NODE),
    };
    rest + 1
}

#[test]
fn new_binary_heap_works() {
    // `BinaryHeap::new`
    let heap = <BinaryHeap<i32>>::new();
    assert!(heap.is_empty());
    assert_eq!(heap.len(), 0);
    // `BinaryHeap::default`
    let default = <BinaryHeap<i32> as Default>::default();
    assert!(default.is_empty());
    assert_eq!(default.len(), 0);
    // `BinaryHeap::new` and `BinaryHeap::default` should be equal.
    assert_eq!(heap, default);
}

#[test]
fn empty_pop_works() {
    let mut heap = BinaryHeap::<i32>::new();
    assert!(heap.pop().is_none());
}

#[test]
fn empty_peek_works() {
    let empty = BinaryHeap::<i32>::new();
    assert!(empty.peek().is_none());
}

#[test]
fn empty_peek_mut_works() {
    let mut empty = BinaryHeap::<i32>::new();
    assert!(empty.peek_mut().is_none());
}

#[test]
fn empty_iter_works() {
    let empty = BinaryHeap::<i32>::new();
    assert!(empty.iter().next().is_none());
}

#[test]
fn from_iterator_works() {
    let some_primes = [1, 2, 3, 5, 7, 11, 13];
    assert_eq!(some_primes.iter().copied().collect::<BinaryHeap<_>>(), {
        let mut vec = BinaryHeap::new();
        for prime in &some_primes {
            vec.push(*prime)
        }
        vec
    });
}

#[test]
fn from_empty_iterator_works() {
    assert_eq!(
        [].iter().copied().collect::<BinaryHeap<i32>>(),
        BinaryHeap::new(),
    );
}

#[test]
fn push_works() {
    let mut heap = heap_from_slice(&[2, 4, 9]);
    assert_eq!(heap.len(), 3);
    assert_eq!(*heap.peek().unwrap(), 9);
    heap.push(11);
    assert_eq!(heap.len(), 4);
    assert_eq!(*heap.peek().unwrap(), 11);
    heap.push(5);
    assert_eq!(heap.len(), 5);
    assert_eq!(*heap.peek().unwrap(), 11);
    heap.push(27);
    assert_eq!(heap.len(), 6);
    assert_eq!(*heap.peek().unwrap(), 27);
    heap.push(3);
    assert_eq!(heap.len(), 7);
    assert_eq!(*heap.peek().unwrap(), 27);
    heap.push(103);
    assert_eq!(heap.len(), 8);
    assert_eq!(*heap.peek().unwrap(), 103);
}

#[test]
fn peek_works() {
    let mut heap = <BinaryHeap<i32>>::new();
    heap.push(33);

    assert_eq!(heap.peek(), Some(&33));
}

#[test]
// Disabled do to possible UB: https://github.com/paritytech/ink/issues/850
#[cfg_attr(miri, ignore)]
fn peek_and_pop_works() {
    let data = vec![2, 4, 6, 2, 1, 8, 10, 3, 5, 7, 0, 9, 1];
    let mut sorted = data.clone();
    sorted.sort();
    let mut heap = heap_from_slice(&data);
    while !heap.is_empty() {
        assert_eq!(heap.peek().unwrap(), sorted.last().unwrap());
        assert_eq!(heap.pop().unwrap(), sorted.pop().unwrap());
    }
}

#[test]
// Disabled do to possible UB: https://github.com/paritytech/ink/issues/850
#[cfg_attr(miri, ignore)]
fn peek_mut_works() {
    let data = vec![2, 4, 6, 2, 1, 8, 10, 3, 5, 7, 0, 9, 1];
    let mut heap = heap_from_slice(&data);
    assert_eq!(heap.peek(), Some(&10));
    {
        let mut top = heap.peek_mut().unwrap();
        *top -= 2;
    }
    assert_eq!(heap.peek(), Some(&9));
}

#[test]
// Disabled do to possible UB: https://github.com/paritytech/ink/issues/850
#[cfg_attr(miri, ignore)]
fn peek_mut_pop_works() {
    let data = vec![2, 4, 6, 2, 1, 8, 10, 3, 5, 7, 0, 9, 1];
    let mut heap = heap_from_slice(&data);
    assert_eq!(heap.peek(), Some(&10));
    {
        let mut top = heap.peek_mut().unwrap();
        *top -= 2;
        assert_eq!(PeekMut::pop(top), 8);
    }
    assert_eq!(heap.peek(), Some(&9));
}

#[test]
// Disabled do to possible UB: https://github.com/paritytech/ink/issues/850
#[cfg_attr(miri, ignore)]
fn min_heap_works() {
    let data = vec![2, 4, 6, 2, 1, 8, 10, 3, 5, 7, 0, 9, 1]
        .iter()
        .map(|x| Reverse::new(*x))
        .collect::<Vec<_>>();
    let mut sorted = data.clone();
    sorted.sort();
    let mut heap = heap_from_slice(&data);
    while !heap.is_empty() {
        assert_eq!(heap.peek().unwrap(), sorted.last().unwrap());
        assert_eq!(heap.pop().unwrap(), sorted.pop().unwrap());
    }
}

#[test]
fn spread_layout_push_pull_works() -> ink_env::Result<()> {
    ink_env::test::run_test::<ink_env::DefaultEnvironment, _>(|_| {
        let heap1 = heap_from_slice(&[b'a', b'b', b'c', b'd']);
        let root_key = Key::from([0x42; 32]);
        SpreadLayout::push_spread(&heap1, &mut KeyPtr::from(root_key));
        // Load the pushed binary heap into another instance and check that
        // both instances are equal:
        let heap2 =
            <BinaryHeap<u8> as SpreadLayout>::pull_spread(&mut KeyPtr::from(root_key));
        assert_eq!(heap1, heap2);
        Ok(())
    })
}

#[test]
#[should_panic(expected = "encountered empty storage cell")]
fn spread_layout_clear_works() {
    ink_env::test::run_test::<ink_env::DefaultEnvironment, _>(|_| {
        let heap1 = heap_from_slice(&[b'a', b'b', b'c', b'd']);
        let root_key = Key::from([0x42; 32]);
        SpreadLayout::push_spread(&heap1, &mut KeyPtr::from(root_key));
        // It has already been asserted that a valid instance can be pulled
        // from contract storage after a push to the same storage region.
        //
        // Now clear the associated storage from `heap1` and check whether
        // loading another instance from this storage will panic since the
        // heap's length property cannot read a value:
        SpreadLayout::clear_spread(&heap1, &mut KeyPtr::from(root_key));
        let _ =
            <BinaryHeap<u8> as SpreadLayout>::pull_spread(&mut KeyPtr::from(root_key));
        Ok(())
    })
    .unwrap()
}

#[test]
#[should_panic(expected = "encountered empty storage cell")]
#[cfg(not(feature = "ink-experimental-engine"))]
fn drop_works() {
    ink_env::test::run_test::<ink_env::DefaultEnvironment, _>(|_| {
        let root_key = Key::from([0x42; 32]);

        // if the setup panics it should not cause the test to pass
        let setup_result = std::panic::catch_unwind(|| {
            let heap = heap_from_slice(&[23, 25, 65]);
            SpreadLayout::push_spread(&heap, &mut KeyPtr::from(root_key));

            let _ = <BinaryHeap<u8> as SpreadLayout>::pull_spread(&mut KeyPtr::from(
                root_key,
            ));
            // heap is dropped which should clear the cells
        });

        assert!(setup_result.is_ok(), "setup should not panic");

        let contract_id = ink_env::test::get_current_contract_account_id::<
            ink_env::DefaultEnvironment,
        >()
        .expect("Cannot get contract id");
        let used_cells = ink_env::test::count_used_storage_cells::<
            ink_env::DefaultEnvironment,
        >(&contract_id)
        .expect("Used cells must be returned");
        assert_eq!(used_cells, 0);

        let _ =
            <BinaryHeap<u8> as SpreadLayout>::pull_spread(&mut KeyPtr::from(root_key));
        Ok(())
    })
    .unwrap()
}

#[test]
#[should_panic(expected = "encountered empty storage cell")]
#[cfg(feature = "ink-experimental-engine")]
fn drop_works() {
    ink_env::test::run_test::<ink_env::DefaultEnvironment, _>(|_| {
        let root_key = Key::from([0x42; 32]);

        // if the setup panics it should not cause the test to pass
        let setup_result = std::panic::catch_unwind(|| {
            let heap = heap_from_slice(&[23, 25, 65]);
            SpreadLayout::push_spread(&heap, &mut KeyPtr::from(root_key));

            let _ = <BinaryHeap<u8> as SpreadLayout>::pull_spread(&mut KeyPtr::from(
                root_key,
            ));
            // heap is dropped which should clear the cells
        });

        assert!(setup_result.is_ok(), "setup should not panic");

        let contract_id = ink_env::test::callee::<ink_env::DefaultEnvironment>();
        let used_cells = ink_env::test::count_used_storage_cells::<
            ink_env::DefaultEnvironment,
        >(&contract_id)
        .expect("Used cells must be returned");
        assert_eq!(used_cells, 0);

        let _ =
            <BinaryHeap<u8> as SpreadLayout>::pull_spread(&mut KeyPtr::from(root_key));
        Ok(())
    })
    .unwrap()
}

#[test]
fn clear_works_on_filled_heap() {
    let mut heap = heap_from_slice(&[b'a', b'b', b'c', b'd']);
    heap.clear();
    assert!(heap.is_empty());
}

#[test]
fn clear_works_on_empty_heap() {
    let mut heap = BinaryHeap::<u8>::default();
    heap.clear();
    assert!(heap.is_empty());
}

#[cfg(not(feature = "ink-experimental-engine"))]
fn check_complexity_read_writes<F>(
    heap_size: u32,
    heap_op: F,
    expected_net_reads: usize,
    expected_net_writes: usize,
) -> ink_env::Result<()>
where
    F: FnOnce(&mut BinaryHeap<u32>),
{
    ink_env::test::run_test::<ink_env::DefaultEnvironment, _>(|_| {
        let heap1 = heap_of_size(heap_size);
        let root_key = Key::from([0x42; 32]);
        SpreadLayout::push_spread(&heap1, &mut KeyPtr::from(root_key));

        let contract_account = ink_env::test::get_current_contract_account_id::<
            ink_env::DefaultEnvironment,
        >()
        .expect("Cannot get contract id");

        let mut lazy_heap =
            <BinaryHeap<u32> as SpreadLayout>::pull_spread(&mut KeyPtr::from(root_key));

        let (base_reads, base_writes) = ink_env::test::get_contract_storage_rw::<
            ink_env::DefaultEnvironment,
        >(&contract_account)?;

        // elements.len + vec.len
        const CONST_WRITES: u32 = 2;
        assert_eq!(
            (base_reads as u32, base_writes as u32),
            (0, CONST_WRITES + get_count_cells(heap_size))
        );

        heap_op(&mut lazy_heap);

        // write back to storage so we can see how many writes required
        SpreadLayout::push_spread(&lazy_heap, &mut KeyPtr::from(root_key));

        let (reads, writes) = ink_env::test::get_contract_storage_rw::<
            ink_env::DefaultEnvironment,
        >(&contract_account)?;

        let net_reads = reads - base_reads;
        let net_writes = writes - base_writes;

        assert_eq!(
            net_reads, expected_net_reads,
            "size {}: storage reads",
            heap_size
        );
        assert_eq!(
            net_writes, expected_net_writes,
            "size {}: storage writes",
            heap_size
        );

        Ok(())
    })
}

#[cfg(feature = "ink-experimental-engine")]
fn check_complexity_read_writes<F>(
    heap_size: u32,
    heap_op: F,
    expected_net_reads: usize,
    expected_net_writes: usize,
) -> ink_env::Result<()>
where
    F: FnOnce(&mut BinaryHeap<u32>),
{
    ink_env::test::run_test::<ink_env::DefaultEnvironment, _>(|_| {
        let heap1 = heap_of_size(heap_size);
        let root_key = Key::from([0x42; 32]);
        SpreadLayout::push_spread(&heap1, &mut KeyPtr::from(root_key));

        let contract_account = ink_env::test::callee::<ink_env::DefaultEnvironment>();

        let mut lazy_heap =
            <BinaryHeap<u32> as SpreadLayout>::pull_spread(&mut KeyPtr::from(root_key));

        let (base_reads, base_writes) = ink_env::test::get_contract_storage_rw::<
            ink_env::DefaultEnvironment,
        >(&contract_account);

        // elements.len + vec.len
        const CONST_WRITES: u32 = 2;
        assert_eq!(
            (base_reads as u32, base_writes as u32),
            (0, CONST_WRITES + get_count_cells(heap_size))
        );

        heap_op(&mut lazy_heap);

        // write back to storage so we can see how many writes required
        SpreadLayout::push_spread(&lazy_heap, &mut KeyPtr::from(root_key));

        let (reads, writes) = ink_env::test::get_contract_storage_rw::<
            ink_env::DefaultEnvironment,
        >(&contract_account);

        let net_reads = reads - base_reads;
        let net_writes = writes - base_writes;

        assert_eq!(
            net_reads, expected_net_reads,
            "size {}: storage reads",
            heap_size
        );
        assert_eq!(
            net_writes, expected_net_writes,
            "size {}: storage writes",
            heap_size
        );

        Ok(())
    })
}

#[test]
fn push_largest_value_complexity_big_o_log_n() -> ink_env::Result<()> {
    // 1 elements overhead (#508) + 1 elements.len + 1 heap overhead (#508) + 1 heap.len + 1 cell
    const CONST_READS: usize = 5;

    // 1 elements.len + 1 cell which was pushed to
    // vec.len does not get larger because no cell is added
    const CONST_WRITES: usize = 2;

    for (n, log_n) in &[(2, 1), (4, 2), (8, 3), (16, 4), (32, 5), (64, 6)] {
        let largest_value = n + 1;
        let expected_reads = log_n + CONST_READS;
        let expected_writes = log_n + CONST_WRITES;
        check_complexity_read_writes(
            *n,
            |heap| heap.push(largest_value),
            expected_reads,
            expected_writes,
        )?;
    }
    Ok(())
}

#[test]
fn push_smallest_value_complexity_big_o_1() -> ink_env::Result<()> {
    const SMALLEST_VALUE: u32 = 0;

    // 1 elements overhead (#508) + 1 elements.len + 1 vec overhead (#508) +
    // 1 vec.len + 1 vec.cell in which to insert + 1 parent cell during `sift_up`
    const EXPECTED_READS: usize = 6;

    // binary heap len + one cell
    // vec.len does not get larger because no cell is added
    const EXPECTED_WRITES: usize = 2;

    for n in &[2, 4, 8, 16, 32, 64] {
        check_complexity_read_writes(
            *n,
            |heap| {
                heap.push(SMALLEST_VALUE);
            },
            EXPECTED_READS,
            EXPECTED_WRITES,
        )?;
    }
    Ok(())
}

#[test]
// Disabled do to possible UB: https://github.com/paritytech/ink/issues/850
#[cfg_attr(miri, ignore)]
fn pop_complexity_big_o_log_n() -> ink_env::Result<()> {
    // 1 elements overhead (#508) + elements.len + 1 vec overhead (#508) +
    // 1 vec.len + 1 vec.cell from which to pop
    const CONST_READS: usize = 5;

    // 1 elements.len + 1 vec.len + cell which was modified
    const CONST_WRITES: usize = 3;

    for (n, log_n) in &[(2, 1), (4, 2), (8, 3), (16, 4), (32, 5), (64, 6)] {
        let expected_reads = log_n + CONST_READS;
        let expected_writes = log_n + CONST_WRITES;

        check_complexity_read_writes(
            *n,
            |heap| {
                heap.pop();
            },
            expected_reads,
            expected_writes,
        )?;
    }
    Ok(())
}

#[cfg(feature = "ink-fuzz-tests")]
#[quickcheck]
fn fuzz_pop_always_returns_largest_element(xs: Vec<i32>) {
    ink_env::test::run_test::<ink_env::DefaultEnvironment, _>(|_| {
        let mut sorted = xs.clone();
        sorted.sort();
        let mut heap = heap_from_slice(&xs);

        for x in sorted.iter().rev() {
            assert_eq!(Some(*x), heap.pop())
        }

        assert_eq!(heap.len(), 0);

        // all elements must have been removed as well
        assert_eq!(heap.elements.children_count(), 0);

        Ok(())
    })
    .unwrap()
}
