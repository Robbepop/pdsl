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

/// Types that are able to flush their state into the contract storage.
///
/// # Note
///
/// Many types support caching of their state into memory to avoid costly
/// contract storage reads or writes. When execution of a contract is finished
/// or interrupted (e.g. due to calling a remote contract) we have to flush
/// all cached state into the contract storage.
///
/// # Implementation Hints
///
/// Caching types provided by pDSL are `SyncCell` for caching of a single data
/// and `SyncChunk` for caching an array of data.
///
/// All abstractions built upon them that do not have their own caching mechanism
/// shall simply forward flushing to their interiors. Examples for this are
/// `storage::Vec` or `storage::Value`.

use parity_codec::Encode;
use crate::storage::key::Key;

pub trait Flush {
    /// Flushes the cached state back to the contract storage, if any.
    fn flush(&mut self) {
        unreachable!();
    }

    /// Default implementation which forwards to flush.
    /// This realizes recursive behavior for e.g. nested vectors.
    fn flush_at(&mut self, _at: Key) {
        self.flush();
    }
}

impl Flush for i8 where Self: Encode {
    fn flush_at(&mut self, at: Key) {
        unsafe {
            crate::env::store(at, &self.encode()[..]);
        }
    }
}

impl Flush for i16 where Self: Encode {
    fn flush_at(&mut self, at: Key) {
        unsafe {
            crate::env::store(at, &self.encode()[..]);
        }
    }
}

impl Flush for i32 where Self: Encode {
    fn flush_at(&mut self, at: Key) {
        unsafe {
            crate::env::store(at, &self.encode()[..]);
        }
    }
}

impl Flush for i64 where Self: Encode {
    fn flush_at(&mut self, at: Key) {
        unsafe {
            crate::env::store(at, &self.encode()[..]);
        }
    }
}

impl Flush for u16 where Self: Encode {
    fn flush_at(&mut self, at: Key) {
        unsafe {
            crate::env::store(at, &self.encode()[..]);
        }
    }
}

impl Flush for u32 where Self: Encode {
    fn flush_at(&mut self, at: Key) {
        unsafe {
            crate::env::store(at, &self.encode()[..]);
        }
    }
}

impl Flush for u64 where Self: Encode {
    fn flush_at(&mut self, at: Key) {
        unsafe {
            crate::env::store(at, &self.encode()[..]);
        }
    }
}

impl Flush for bool where Self: Encode {
    fn flush_at(&mut self, at: Key) {
        unsafe {
            crate::env::store(at, &self.encode()[..]);
        }
    }
}
