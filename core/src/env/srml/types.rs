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

use core::{array::TryFromSliceError, convert::TryFrom};

use crate::{env::EnvTypes, impl_empty_flush_for, storage::Flush};
use parity_codec::{Decode, Encode};

/// The SRML fundamental types.
#[allow(unused)]
pub enum DefaultSrmlTypes {}

impl EnvTypes for DefaultSrmlTypes {
    type AccountId = AccountId;
    type Balance = Balance;
    type Hash = Hash;
    type Moment = Moment;
    type BlockNumber = BlockNumber;
}

/// The default SRML address type.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Encode, Decode)]
pub struct AccountId([u8; 32]);

impl From<[u8; 32]> for AccountId {
    fn from(address: [u8; 32]) -> AccountId {
        AccountId(address)
    }
}

impl<'a> TryFrom<&'a [u8]> for AccountId {
    type Error = TryFromSliceError;

    fn try_from(bytes: &'a [u8]) -> Result<AccountId, TryFromSliceError> {
        let address = <[u8; 32]>::try_from(bytes)?;
        Ok(AccountId(address))
    }
}

/// The default SRML balance type.
pub type Balance = u64;

/// The default SRML hash type.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Encode, Decode)]
pub struct Hash([u8; 32]);

impl From<[u8; 32]> for Hash {
    fn from(hash: [u8; 32]) -> Hash {
        Hash(hash)
    }
}

impl<'a> TryFrom<&'a [u8]> for Hash {
    type Error = TryFromSliceError;

    fn try_from(bytes: &'a [u8]) -> Result<Hash, TryFromSliceError> {
        let hash = <[u8; 32]>::try_from(bytes)?;
        Ok(Hash(hash))
    }
}

/// The default SRML moment type.
pub type Moment = u64;

/// The default SRML blocknumber type.
pub type BlockNumber = u64;

impl_empty_flush_for!(AccountId, Hash);
