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
use super::Vec as StorageVec;
use crate::{
    collections::vec::IndexOutOfBounds,
    traits::{
        KeyPtr,
        PackedLayout,
        SpreadLayout,
    },
    Lazy,
};
use core::cmp::Ordering;
use ink_primitives::Key;

#[test]
fn new_vec_works() {
    // `StorageVec::new`
    let vec = <StorageVec<i32>>::new();
    assert!(vec.is_empty());
    assert_eq!(vec.len(), 0);
    assert_eq!(vec.get(0), None);
    assert!(vec.iter().next().is_none());
    // `StorageVec::default`
    let default = <StorageVec<i32> as Default>::default();
    assert!(default.is_empty());
    assert_eq!(default.len(), 0);
    assert_eq!(vec.get(0), None);
    assert!(default.iter().next().is_none());
    // `StorageVec::new` and `StorageVec::default` should be equal.
    assert_eq!(vec, default);
}

#[test]
fn from_iterator_works() {
    let some_primes = [1, 2, 3, 5, 7, 11, 13];
    assert_eq!(some_primes.iter().copied().collect::<StorageVec<_>>(), {
        let mut vec = StorageVec::new();
        for prime in &some_primes {
            vec.push(*prime)
        }
        vec
    });
}

#[test]
fn from_empty_iterator_works() {
    assert_eq!(
        [].iter().copied().collect::<StorageVec<i32>>(),
        StorageVec::new(),
    );
}

#[test]
fn first_last_of_empty() {
    let mut vec = <StorageVec<u8>>::new();
    assert_eq!(vec.first(), None);
    assert_eq!(vec.first_mut(), None);
    assert_eq!(vec.last(), None);
    assert_eq!(vec.last_mut(), None);
}

#[test]
fn push_pop_first_last_works() {
    /// Asserts conditions are met for the given storage vector.
    fn assert_vec<F, L>(vec: &StorageVec<u8>, len: u32, first: F, last: L)
    where
        F: Into<Option<u8>>,
        L: Into<Option<u8>>,
    {
        assert_eq!(vec.is_empty(), len == 0);
        assert_eq!(vec.len(), len);
        assert_eq!(vec.first().copied(), first.into());
        assert_eq!(vec.last().copied(), last.into());
    }

    let mut vec = StorageVec::new();
    assert_vec(&vec, 0, None, None);

    // Sequence of `push`
    vec.push(b'a');
    assert_vec(&vec, 1, b'a', b'a');
    vec.push(b'b');
    assert_vec(&vec, 2, b'a', b'b');
    vec.push(b'c');
    assert_vec(&vec, 3, b'a', b'c');
    vec.push(b'd');
    assert_vec(&vec, 4, b'a', b'd');

    // Sequence of `pop`
    assert_eq!(vec.pop(), Some(b'd'));
    assert_vec(&vec, 3, b'a', b'c');
    assert_eq!(vec.pop(), Some(b'c'));
    assert_vec(&vec, 2, b'a', b'b');
    assert_eq!(vec.pop(), Some(b'b'));
    assert_vec(&vec, 1, b'a', b'a');
    assert_eq!(vec.pop(), Some(b'a'));
    assert_vec(&vec, 0, None, None);

    // Pop from empty vector.
    assert_eq!(vec.pop(), None);
    assert_vec(&vec, 0, None, None);
}

#[test]
fn pop_drop_works() {
    let elems = [b'a', b'b', b'c', b'd'];
    let mut vec = vec_from_slice(&elems);
    assert_eq!(vec.pop_drop(), Some(()));
    assert_eq_slice(&vec, &elems[0..3]);
    assert_eq!(vec.pop_drop(), Some(()));
    assert_eq_slice(&vec, &elems[0..2]);
    assert_eq!(vec.pop_drop(), Some(()));
    assert_eq_slice(&vec, &elems[0..1]);
    assert_eq!(vec.pop_drop(), Some(()));
    assert_eq_slice(&vec, &[]);
    assert_eq!(vec.pop_drop(), None);
    assert_eq_slice(&vec, &[]);
}

#[test]
fn get_works() {
    let elems = [b'a', b'b', b'c', b'd'];
    let mut vec = vec_from_slice(&elems);
    for (n, mut expected) in elems.iter().copied().enumerate() {
        let n = n as u32;
        assert_eq!(vec.get(n), Some(&expected));
        assert_eq!(vec.get_mut(n), Some(&mut expected));
        assert_eq!(&vec[n], &expected);
        assert_eq!(&mut vec[n], &mut expected);
    }
    let len = vec.len();
    assert_eq!(vec.get(len), None);
    assert_eq!(vec.get_mut(len), None);
}

#[test]
#[should_panic(expected = "index out of bounds: the len is 3 but the index is 3")]
fn index_out_of_bounds_works() {
    let test_values = [b'a', b'b', b'c'];
    let vec = vec_from_slice(&test_values);
    let _ = &vec[test_values.len() as u32];
}

#[test]
#[should_panic(expected = "index out of bounds: the len is 3 but the index is 3")]
fn index_mut_out_of_bounds_works() {
    let test_values = [b'a', b'b', b'c'];
    let mut vec = vec_from_slice(&test_values);
    let _ = &mut vec[test_values.len() as u32];
}

#[test]
fn iter_next_works() {
    let elems = [b'a', b'b', b'c', b'd'];
    let vec = vec_from_slice(&elems);
    // Test iterator over `&T`:
    let mut iter = vec.iter();
    assert_eq!(iter.count(), 4);
    assert_eq!(iter.size_hint(), (4, Some(4)));
    assert_eq!(iter.next(), Some(&b'a'));
    assert_eq!(iter.size_hint(), (3, Some(3)));
    assert_eq!(iter.next(), Some(&b'b'));
    assert_eq!(iter.size_hint(), (2, Some(2)));
    assert_eq!(iter.count(), 2);
    assert_eq!(iter.next(), Some(&b'c'));
    assert_eq!(iter.size_hint(), (1, Some(1)));
    assert_eq!(iter.next(), Some(&b'd'));
    assert_eq!(iter.size_hint(), (0, Some(0)));
    assert_eq!(iter.count(), 0);
    assert_eq!(iter.next(), None);
    // Test iterator over `&mut T`:
    let mut vec = vec;
    let mut iter = vec.iter_mut();
    assert_eq!(iter.size_hint(), (4, Some(4)));
    assert_eq!(iter.next(), Some(&mut b'a'));
    assert_eq!(iter.size_hint(), (3, Some(3)));
    assert_eq!(iter.next(), Some(&mut b'b'));
    assert_eq!(iter.size_hint(), (2, Some(2)));
    assert_eq!(iter.next(), Some(&mut b'c'));
    assert_eq!(iter.size_hint(), (1, Some(1)));
    assert_eq!(iter.next(), Some(&mut b'd'));
    assert_eq!(iter.size_hint(), (0, Some(0)));
    assert_eq!(iter.next(), None);
    assert_eq!(iter.count(), 0);
}

#[test]
fn iter_nth_works() {
    let elems = [b'a', b'b', b'c', b'd'];
    let vec = vec_from_slice(&elems);
    // Test iterator over `&T`:
    let mut iter = vec.iter();
    assert_eq!(iter.count(), 4);
    assert_eq!(iter.size_hint(), (4, Some(4)));
    assert_eq!(iter.nth(1), Some(&b'b'));
    assert_eq!(iter.count(), 2);
    assert_eq!(iter.size_hint(), (2, Some(2)));
    assert_eq!(iter.nth(1), Some(&b'd'));
    assert_eq!(iter.size_hint(), (0, Some(0)));
    assert_eq!(iter.count(), 0);
    assert_eq!(iter.nth(1), None);
    // Test iterator over `&mut T`:
    let mut vec = vec;
    let mut iter = vec.iter_mut();
    assert_eq!(iter.size_hint(), (4, Some(4)));
    assert_eq!(iter.nth(1), Some(&mut b'b'));
    assert_eq!(iter.size_hint(), (2, Some(2)));
    assert_eq!(iter.nth(1), Some(&mut b'd'));
    assert_eq!(iter.size_hint(), (0, Some(0)));
    assert_eq!(iter.nth(1), None);
    assert_eq!(iter.count(), 0);
}

#[test]
fn iter_next_back_works() {
    let elems = [b'a', b'b', b'c', b'd'];
    let vec = vec_from_slice(&elems);
    // Test iterator over `&T`:
    let mut iter = vec.iter().rev();
    assert_eq!(iter.clone().count(), 4);
    assert_eq!(iter.next(), Some(&b'd'));
    assert_eq!(iter.next(), Some(&b'c'));
    assert_eq!(iter.clone().count(), 2);
    assert_eq!(iter.next(), Some(&b'b'));
    assert_eq!(iter.next(), Some(&b'a'));
    assert_eq!(iter.clone().count(), 0);
    assert_eq!(iter.next(), None);
    // Test iterator over `&mut T`:
    let mut vec = vec;
    let mut iter = vec.iter_mut().rev();
    assert_eq!(iter.next(), Some(&mut b'd'));
    assert_eq!(iter.next(), Some(&mut b'c'));
    assert_eq!(iter.next(), Some(&mut b'b'));
    assert_eq!(iter.next(), Some(&mut b'a'));
    assert_eq!(iter.next(), None);
    assert_eq!(iter.count(), 0);
}

#[test]
fn iter_nth_back_works() {
    let elems = [b'a', b'b', b'c', b'd'];
    let vec = vec_from_slice(&elems);
    // Test iterator over `&T`:
    let mut iter = vec.iter().rev();
    assert_eq!(iter.clone().count(), 4);
    assert_eq!(iter.nth(1), Some(&b'c'));
    assert_eq!(iter.clone().count(), 2);
    assert_eq!(iter.nth(1), Some(&b'a'));
    assert_eq!(iter.clone().count(), 0);
    assert_eq!(iter.nth(1), None);
    // Test iterator over `&mut T`:
    let mut vec = vec;
    let mut iter = vec.iter_mut().rev();
    assert_eq!(iter.nth(1), Some(&mut b'c'));
    assert_eq!(iter.nth(1), Some(&mut b'a'));
    assert_eq!(iter.nth(1), None);
    assert_eq!(iter.count(), 0);
}

/// Asserts that the given ordered storage vector elements are equal to the
/// ordered elements of the given slice.
fn assert_eq_slice(vec: &StorageVec<u8>, slice: &[u8]) {
    assert_eq!(vec.len() as usize, slice.len());
    assert!(vec.iter().zip(slice.iter()).all(|(lhs, rhs)| *lhs == *rhs))
}

/// Creates a storage vector from the given slice.
fn vec_from_slice<T: Copy + PackedLayout>(slice: &[T]) -> StorageVec<T> {
    slice.iter().copied().collect::<StorageVec<T>>()
}

#[test]
fn swap_works() {
    let elems = [b'a', b'b', b'c', b'd'];
    let mut vec = vec_from_slice(&elems);

    // Swap at same position is a no-op.
    for index in 0..elems.len() as u32 {
        vec.swap(index, index);
        assert_eq_slice(&vec, &elems);
    }

    // Swap first and second
    vec.swap(0, 1);
    assert_eq_slice(&vec, &[b'b', b'a', b'c', b'd']);
    // Swap third and last
    vec.swap(2, 3);
    assert_eq_slice(&vec, &[b'b', b'a', b'd', b'c']);
    // Swap first and last
    vec.swap(0, 3);
    assert_eq_slice(&vec, &[b'c', b'a', b'd', b'b']);
}

#[test]
#[should_panic]
fn swap_one_invalid_index() {
    let mut vec = vec_from_slice(&[b'a', b'b', b'c', b'd']);
    vec.swap(0, vec.len());
}

#[test]
#[should_panic]
fn swap_both_invalid_indices() {
    let mut vec = vec_from_slice(&[b'a', b'b', b'c', b'd']);
    vec.swap(vec.len(), vec.len());
}

#[test]
fn swap_remove_works() {
    let mut vec = vec_from_slice(&[b'a', b'b', b'c', b'd']);

    // Swap remove first element.
    assert_eq!(vec.swap_remove(0), Some(b'a'));
    assert_eq_slice(&vec, &[b'd', b'b', b'c']);
    // Swap remove middle element.
    assert_eq!(vec.swap_remove(1), Some(b'b'));
    assert_eq_slice(&vec, &[b'd', b'c']);
    // Swap remove last element.
    assert_eq!(vec.swap_remove(1), Some(b'c'));
    assert_eq_slice(&vec, &[b'd']);
    // Swap remove only element.
    assert_eq!(vec.swap_remove(0), Some(b'd'));
    assert_eq_slice(&vec, &[]);
    // Swap remove from empty vector.
    assert_eq!(vec.swap_remove(0), None);
    assert_eq_slice(&vec, &[]);
}

#[test]
fn swap_remove_drop_works() {
    let mut vec = vec_from_slice(&[b'a', b'b', b'c', b'd']);

    // Swap remove first element.
    assert_eq!(vec.swap_remove_drop(0), Some(()));
    assert_eq_slice(&vec, &[b'd', b'b', b'c']);
    // Swap remove middle element.
    assert_eq!(vec.swap_remove_drop(1), Some(()));
    assert_eq_slice(&vec, &[b'd', b'c']);
    // Swap remove last element.
    assert_eq!(vec.swap_remove_drop(1), Some(()));
    assert_eq_slice(&vec, &[b'd']);
    // Swap remove only element.
    assert_eq!(vec.swap_remove_drop(0), Some(()));
    assert_eq_slice(&vec, &[]);
    // Swap remove from empty vector.
    assert_eq!(vec.swap_remove_drop(0), None);
    assert_eq_slice(&vec, &[]);
}

#[test]
fn spread_layout_push_pull_works() -> ink_env::Result<()> {
    ink_env::test::run_test::<ink_env::DefaultEnvironment, _>(|_| {
        let vec1 = vec_from_slice(&[b'a', b'b', b'c', b'd']);
        let root_key = Key::from([0x42; 32]);
        SpreadLayout::push_spread(&vec1, &mut KeyPtr::from(root_key));
        // Load the pushed storage vector into another instance and check that
        // both instances are equal:
        let vec2 =
            <StorageVec<u8> as SpreadLayout>::pull_spread(&mut KeyPtr::from(root_key));
        assert_eq!(vec1, vec2);
        Ok(())
    })
}

#[test]
#[should_panic(expected = "encountered empty storage cell")]
fn spread_layout_clear_works() {
    ink_env::test::run_test::<ink_env::DefaultEnvironment, _>(|_| {
        let vec1 = vec_from_slice(&[b'a', b'b', b'c', b'd']);
        let root_key = Key::from([0x42; 32]);
        SpreadLayout::push_spread(&vec1, &mut KeyPtr::from(root_key));
        // It has already been asserted that a valid instance can be pulled
        // from contract storage after a push to the same storage region.
        //
        // Now clear the associated storage from `vec1` and check whether
        // loading another instance from this storage will panic since the
        // vector's length property cannot read a value:
        SpreadLayout::clear_spread(&vec1, &mut KeyPtr::from(root_key));
        let _ =
            <StorageVec<u8> as SpreadLayout>::pull_spread(&mut KeyPtr::from(root_key));
        Ok(())
    })
    .unwrap()
}

#[test]
fn set_works() {
    ink_env::test::run_test::<ink_env::DefaultEnvironment, _>(|_| {
        let mut vec = vec_from_slice(&[b'a', b'b', b'c', b'd']);
        let _ = vec.set(0, b'x').unwrap();
        let expected = vec_from_slice(&[b'x', b'b', b'c', b'd']);
        assert_eq!(vec, expected);
        Ok(())
    })
    .unwrap()
}

#[test]
fn set_fails_when_index_oob() {
    let mut vec = vec_from_slice(&[b'a']);
    let res = vec.set(1, b'x');
    assert_eq!(res, Err(IndexOutOfBounds));
}

#[test]
fn clear_works_on_filled_vec() {
    let mut vec = vec_from_slice(&[b'a', b'b', b'c', b'd']);
    vec.clear();
    assert!(vec.is_empty());
}

#[test]
fn clear_works_on_empty_vec() {
    let mut vec: StorageVec<()> = vec_from_slice(&[]);
    vec.clear();
    assert!(vec.is_empty());
}

#[test]
fn test_binary_search() {
    let b: StorageVec<i32> = StorageVec::new();
    assert_eq!(b.binary_search(&5), Err(0));

    let b = vec_from_slice(&[4]);
    assert_eq!(b.binary_search(&3), Err(0));
    assert_eq!(b.binary_search(&4), Ok(0));
    assert_eq!(b.binary_search(&5), Err(1));

    let b = vec_from_slice(&[1, 2, 4, 6, 8, 9]);
    assert_eq!(b.binary_search(&5), Err(3));
    assert_eq!(b.binary_search(&6), Ok(3));
    assert_eq!(b.binary_search(&7), Err(4));
    assert_eq!(b.binary_search(&8), Ok(4));

    let b = vec_from_slice(&[1, 2, 4, 5, 6, 8]);
    assert_eq!(b.binary_search(&9), Err(6));

    let b = vec_from_slice(&[1, 2, 4, 6, 7, 8, 9]);
    assert_eq!(b.binary_search(&6), Ok(3));
    assert_eq!(b.binary_search(&5), Err(3));
    assert_eq!(b.binary_search(&8), Ok(5));

    let b = vec_from_slice(&[1, 2, 4, 5, 6, 8, 9]);
    assert_eq!(b.binary_search(&7), Err(5));
    assert_eq!(b.binary_search(&0), Err(0));

    let b = vec_from_slice(&[1, 3, 3, 3, 7]);
    assert_eq!(b.binary_search(&0), Err(0));
    assert_eq!(b.binary_search(&1), Ok(0));
    assert_eq!(b.binary_search(&2), Err(1));
    assert!(match b.binary_search(&3) {
        Ok(1..=3) => true,
        _ => false,
    });
    assert!(match b.binary_search(&3) {
        Ok(1..=3) => true,
        _ => false,
    });
    assert_eq!(b.binary_search(&4), Err(4));
    assert_eq!(b.binary_search(&5), Err(4));
    assert_eq!(b.binary_search(&6), Err(4));
    assert_eq!(b.binary_search(&7), Ok(4));
    assert_eq!(b.binary_search(&8), Err(5));

    let b = vec_from_slice(&[(); u8::MAX as usize]);
    assert_eq!(b.binary_search(&()), Ok(u8::MAX as u32 / 2));
}

#[test]
fn test_binary_search_by_overflow() {
    let b = vec_from_slice(&[(); u8::MAX as usize]);
    assert_eq!(
        b.binary_search_by(|_| Ordering::Equal),
        Ok(u8::MAX as u32 / 2)
    );
    assert_eq!(b.binary_search_by(|_| Ordering::Greater), Err(0));
    assert_eq!(b.binary_search_by(|_| Ordering::Less), Err(u8::MAX as u32));
}

#[test]
// Test implementation specific behavior when finding equivalent elements.
fn test_binary_search_implementation_details() {
    let b = vec_from_slice(&[1, 1, 2, 2, 3, 3, 3]);
    assert_eq!(b.binary_search(&1), Ok(1));
    assert_eq!(b.binary_search(&2), Ok(3));
    assert_eq!(b.binary_search(&3), Ok(5));
    let b = vec_from_slice(&[1, 1, 1, 1, 1, 3, 3, 3, 3]);
    assert_eq!(b.binary_search(&1), Ok(4));
    assert_eq!(b.binary_search(&3), Ok(7));
    let b = vec_from_slice(&[1, 1, 1, 1, 3, 3, 3, 3, 3]);
    assert_eq!(b.binary_search(&1), Ok(2));
    assert_eq!(b.binary_search(&3), Ok(4));
}

#[test]
fn test_split_at_mut() {
    let mut b = vec_from_slice(&[1, 1, 2, 2, 3, 3, 3]);
    let (mut a, mut c) = b.split_at_mut(3);
    a[2] += 1;
    c[0] += 1;
}

#[test]
#[should_panic(expected = "encountered empty storage cell")]
#[cfg(not(feature = "ink-experimental-engine"))]
fn storage_is_cleared_completely_after_pull_lazy() {
    ink_env::test::run_test::<ink_env::DefaultEnvironment, _>(|_| {
        // given
        let root_key = Key::from([0x42; 32]);
        let mut lazy_vec: Lazy<StorageVec<u32>> = Lazy::new(StorageVec::new());
        lazy_vec.push(13u32);
        lazy_vec.push(13u32);
        SpreadLayout::push_spread(&lazy_vec, &mut KeyPtr::from(root_key));
        let pulled_vec = <Lazy<StorageVec<u32>> as SpreadLayout>::pull_spread(
            &mut KeyPtr::from(root_key),
        );

        // when
        SpreadLayout::clear_spread(&pulled_vec, &mut KeyPtr::from(root_key));

        // then
        let contract_id = ink_env::test::get_current_contract_account_id::<
            ink_env::DefaultEnvironment,
        >()
        .expect("Cannot get contract id");
        let used_cells = ink_env::test::count_used_storage_cells::<
            ink_env::DefaultEnvironment,
        >(&contract_id)
        .expect("used cells must be returned");
        assert_eq!(used_cells, 0);
        let _ =
            *<Lazy<Lazy<u32>> as SpreadLayout>::pull_spread(&mut KeyPtr::from(root_key));

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
            let vec = vec_from_slice(&[b'a', b'b', b'c', b'd']);
            SpreadLayout::push_spread(&vec, &mut KeyPtr::from(root_key));
            let _ = <StorageVec<u8> as SpreadLayout>::pull_spread(&mut KeyPtr::from(
                root_key,
            ));
            // vec is dropped which should clear the cells
        });
        assert!(setup_result.is_ok(), "setup should not panic");

        let contract_id = ink_env::test::get_current_contract_account_id::<
            ink_env::DefaultEnvironment,
        >()
        .expect("Cannot get contract id");
        let used_cells = ink_env::test::count_used_storage_cells::<
            ink_env::DefaultEnvironment,
        >(&contract_id)
        .expect("used cells must be returned");
        assert_eq!(used_cells, 0);

        let _ =
            <StorageVec<u8> as SpreadLayout>::pull_spread(&mut KeyPtr::from(root_key));
        Ok(())
    })
    .unwrap()
}

#[test]
#[should_panic(expected = "encountered empty storage cell")]
#[cfg(feature = "ink-experimental-engine")]
fn storage_is_cleared_completely_after_pull_lazy() {
    ink_env::test::run_test::<ink_env::DefaultEnvironment, _>(|_| {
        // given
        let root_key = Key::from([0x42; 32]);
        let mut lazy_vec: Lazy<StorageVec<u32>> = Lazy::new(StorageVec::new());
        lazy_vec.push(13u32);
        lazy_vec.push(13u32);
        SpreadLayout::push_spread(&lazy_vec, &mut KeyPtr::from(root_key));
        let pulled_vec = <Lazy<StorageVec<u32>> as SpreadLayout>::pull_spread(
            &mut KeyPtr::from(root_key),
        );

        // when
        SpreadLayout::clear_spread(&pulled_vec, &mut KeyPtr::from(root_key));

        // then
        let contract_id = ink_env::test::callee::<ink_env::DefaultEnvironment>();
        let used_cells = ink_env::test::count_used_storage_cells::<
            ink_env::DefaultEnvironment,
        >(&contract_id)
        .expect("used cells must be returned");
        assert_eq!(used_cells, 0);
        let _ =
            *<Lazy<Lazy<u32>> as SpreadLayout>::pull_spread(&mut KeyPtr::from(root_key));

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
            let vec = vec_from_slice(&[b'a', b'b', b'c', b'd']);
            SpreadLayout::push_spread(&vec, &mut KeyPtr::from(root_key));
            let _ = <StorageVec<u8> as SpreadLayout>::pull_spread(&mut KeyPtr::from(
                root_key,
            ));
            // vec is dropped which should clear the cells
        });
        assert!(setup_result.is_ok(), "setup should not panic");

        let contract_id = ink_env::test::callee::<ink_env::DefaultEnvironment>();
        let used_cells = ink_env::test::count_used_storage_cells::<
            ink_env::DefaultEnvironment,
        >(&contract_id)
        .expect("used cells must be returned");
        assert_eq!(used_cells, 0);

        let _ =
            <StorageVec<u8> as SpreadLayout>::pull_spread(&mut KeyPtr::from(root_key));
        Ok(())
    })
    .unwrap()
}
