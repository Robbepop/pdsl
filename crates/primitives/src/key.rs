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

use core::{
    fmt,
    ops::{
        Add,
        AddAssign,
    },
};

/// Key into contract storage.
///
/// Used to identify contract storage cells for read and write operations.
/// Can be compared to a raw pointer and features simple pointer arithmetic.
///
/// # Note
///
/// This is the most low-level primitive to identify contract storage cells.
///
/// # Unsafe
///
/// Prefer using high-level types found in `ink_storage` to operate on the contract
/// storage.
#[derive(Copy, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Key([u64; 4]);

impl Key {
    fn write_bytes(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x")?;
        for limb in &self.0 {
            write!(f, "_")?;
            for byte in &limb.to_le_bytes() {
                write!(f, "{:02X}", byte)?;
            }
        }
        Ok(())
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Key(")?;
        self.write_bytes(f)?;
        write!(f, ")")?;
        Ok(())
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.write_bytes(f)
    }
}

impl From<[u8; 32]> for Key {
    #[inline]
    fn from(bytes: [u8; 32]) -> Self {
        if cfg!(target_endian = "little") {
            // SAFETY: If the machine has little endian byte ordering we can
            //         simply transmute the input bytes into the correct `u64`
            //         byte ordering for the `Key` data structure. Otherwise
            //         we have to manually convert them via the
            //         `from_bytes_be_fallback` procedure.
            //
            // We decided to have the little endian as default format for Key
            // instance since WebAssembly dictates little endian byte ordering
            // for the execution environment.
            Self(unsafe { ::core::mem::transmute::<[u8; 32], [u64; 4]>(bytes) })
        } else {
            Self::from_bytes_be_fallback(bytes)
        }
    }
}

impl Key {
    /// Creates a new key from the given bytes.
    ///
    /// # Note
    ///
    /// This is a fallback procedure in case the target machine does not have
    /// little endian byte ordering.
    #[inline]
    fn from_bytes_be_fallback(bytes: [u8; 32]) -> Self {
        #[inline]
        fn carve_out_u64_bytes(bytes: &[u8; 32], offset: u8) -> [u8; 8] {
            let o = (offset * 8) as usize;
            [
                bytes[o],
                bytes[o + 1],
                bytes[o + 2],
                bytes[o + 3],
                bytes[o + 4],
                bytes[o + 5],
                bytes[o + 6],
                bytes[o + 7],
            ]
        }
        Self([
            u64::from_le_bytes(carve_out_u64_bytes(&bytes, 0)),
            u64::from_le_bytes(carve_out_u64_bytes(&bytes, 1)),
            u64::from_le_bytes(carve_out_u64_bytes(&bytes, 2)),
            u64::from_le_bytes(carve_out_u64_bytes(&bytes, 3)),
        ])
    }

    /// Tries to return the underlying bytes as slice.
    ///
    /// This only returns `Some` if the execution environment has little-endian
    /// byte order.
    pub fn try_as_bytes(&self) -> Option<&[u8; 32]> {
        if cfg!(target_endian = "little") {
            return Some(self.as_bytes())
        }
        None
    }

    /// Returns the underlying bytes of the key.
    ///
    /// This only works and is supported if the target machine has little-endian
    /// byte ordering. Use [`Key::try_as_bytes`] as a general procedure instead.
    #[cfg(target_endian = "little")]
    pub fn as_bytes(&self) -> &[u8; 32] {
        // SAFETY: This pointer cast is possible since the outer struct
        //         (Key) is `repr(transparent)` and since we restrict
        //         ourselves to little-endian byte ordering. In any other
        //         case this is invalid which is why return `None` as
        //         fallback.
        unsafe { &*(&self.0 as *const [u64; 4] as *const [u8; 32]) }
    }
}

impl scale::Encode for Key {
    #[inline(always)]
    fn size_hint(&self) -> usize {
        32
    }

    #[inline]
    fn encode_to<T: scale::Output + ?Sized>(&self, dest: &mut T) {
        if cfg!(target_endian = "little") {
            dest.write(self.try_as_bytes().expect("little endian is asserted"))
        } else {
            dest.write(&self.to_bytes())
        }
    }
}

impl scale::Decode for Key {
    #[inline]
    fn decode<I: scale::Input>(input: &mut I) -> Result<Self, scale::Error> {
        Ok(Self::from(<[u8; 32] as scale::Decode>::decode(input)?))
    }
}

#[cfg(target_endian = "little")]
impl Key {
    /// Returns the bytes that are representing the key.
    #[inline]
    pub fn to_bytes(self) -> [u8; 32] {
        if cfg!(target_endian = "little") {
            // SAFETY: This pointer cast is possible since the outer struct
            //         (Key) is `repr(transparent)` and since we restrict
            //         ourselves to little-endian byte ordering. In any other
            //         case this is invalid which is why return `None` as
            //         fallback.
            unsafe { core::mem::transmute::<[u64; 4], [u8; 32]>(self.0) }
        } else {
            self.to_bytes_be_fallback()
        }
    }

    /// Fallback big-endian procedure to return the underlying bytes of `self`.
    fn to_bytes_be_fallback(self) -> [u8; 32] {
        let mut result = [0x00; 32];
        for i in 0..4 {
            let o = i * 8;
            result[o..(o + 8)].copy_from_slice(&self.0[i].to_le_bytes());
        }
        result
    }
}

impl Add<u64> for Key {
    type Output = Key;

    fn add(mut self, rhs: u64) -> Self::Output {
        self += rhs;
        self
    }
}

impl Add<u64> for &Key {
    type Output = Key;

    fn add(self, rhs: u64) -> Self::Output {
        <Key as Add<u64>>::add(*self, rhs)
    }
}

impl Add<&u64> for Key {
    type Output = Key;

    fn add(self, rhs: &u64) -> Self::Output {
        <Key as Add<u64>>::add(self, *rhs)
    }
}

impl Add<&u64> for &Key {
    type Output = Key;

    fn add(self, rhs: &u64) -> Self::Output {
        <&Key as Add<u64>>::add(self, *rhs)
    }
}

#[cfg(feature = "std")]
const _: () = {
    use scale_info::{
        build::Fields,
        Path,
        Type,
        TypeInfo,
    };

    impl TypeInfo for Key {
        type Identity = Self;

        fn type_info() -> Type {
            Type::builder()
                .path(Path::new("Key", "ink_primitives"))
                .composite(
                    Fields::unnamed().field(|f| f.ty::<[u8; 32]>().type_name("[u8; 32]")),
                )
        }
    }
};

impl AddAssign<u64> for Key {
    #[inline]
    #[rustfmt::skip]
    fn add_assign(&mut self, rhs: u64) {
        let (res_0,  ovfl_0) = self.0[0].overflowing_add(rhs);
        let (res_1,  ovfl_1) = self.0[1].overflowing_add(ovfl_0 as u64);
        let (res_2,  ovfl_2) = self.0[2].overflowing_add(ovfl_1 as u64);
        let (res_3, _ovfl_3) = self.0[3].overflowing_add(ovfl_2 as u64);
        self.0[0] = res_0;
        self.0[1] = res_1;
        self.0[2] = res_2;
        self.0[3] = res_3;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_bytes() -> [u8; 32] {
        *b"\
            \x00\x01\x02\x03\x04\x05\x06\x07\
            \x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\
            \x10\x11\x12\x13\x14\x15\x16\x17\
            \x18\x19\x1A\x1B\x1C\x1D\x1E\x1F\
        "
    }

    #[test]
    fn default_works() {
        assert_eq!(<Key as Default>::default().to_bytes(), [0x00; 32]);
    }

    #[test]
    fn debug_works() {
        let key = Key::from(test_bytes());
        assert_eq!(
            format!("{:?}", key),
            String::from(
                "Key(0x\
                    _0001020304050607\
                    _08090A0B0C0D0E0F\
                    _1011121314151617\
                    _18191A1B1C1D1E1F\
                )"
            ),
        );
    }

    #[test]
    #[rustfmt::skip]
    fn from_works() {
        let test_bytes = test_bytes();
        assert_eq!(Key::from(test_bytes).to_bytes(), test_bytes);
        assert_eq!(Key::from_bytes_be_fallback(test_bytes).to_bytes(), test_bytes);
        assert_eq!(Key::from(test_bytes).to_bytes_be_fallback(), test_bytes);
        assert_eq!(Key::from_bytes_be_fallback(test_bytes).to_bytes_be_fallback(), test_bytes);
    }

    #[test]
    fn add_one_to_zero() {
        let bytes = [0x00; 32];
        let expected = {
            let mut bytes = [0x00; 32];
            bytes[0] = 0x01;
            bytes
        };
        let mut key = Key::from(bytes);
        key.add_assign(1u64);
        assert_eq!(key.to_bytes(), expected);
    }

    #[test]
    fn add_with_ovfl() {
        let bytes = *b"\
            \xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\
            \x00\x00\x00\x00\x00\x00\x00\x00\
            \x00\x00\x00\x00\x00\x00\x00\x00\
            \x00\x00\x00\x00\x00\x00\x00\x00\
        ";
        let expected = {
            let mut expected = [0x00; 32];
            expected[8] = 0x01;
            expected
        };
        let mut key = Key::from(bytes);
        key.add_assign(1u64);
        assert_eq!(key.to_bytes(), expected);
    }

    #[test]
    fn add_with_ovfl_2() {
        let bytes = *b"\
            \xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\
            \xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\
            \x00\x00\x00\x00\x00\x00\x00\x00\
            \x00\x00\x00\x00\x00\x00\x00\x00\
        ";
        let expected = {
            let mut expected = [0x00; 32];
            expected[16] = 0x01;
            expected
        };
        let mut key = Key::from(bytes);
        key.add_assign(1u64);
        assert_eq!(key.to_bytes(), expected);
    }

    #[test]
    fn add_with_ovfl_3() {
        let bytes = *b"\
            \xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\
            \xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\
            \xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\
            \x00\x00\x00\x00\x00\x00\x00\x00\
        ";
        let expected = {
            let mut expected = [0x00; 32];
            expected[24] = 0x01;
            expected
        };
        let mut key = Key::from(bytes);
        key.add_assign(1u64);
        assert_eq!(key.to_bytes(), expected);
    }

    #[test]
    fn add_with_wrap() {
        let bytes = [0xFF; 32];
        let expected = [0x00; 32];
        let mut key = Key::from(bytes);
        key.add_assign(1u64);
        assert_eq!(key.to_bytes(), expected);
    }

    #[test]
    fn add_assign_to_zero() {
        for test_value in &[0_u64, 1, 42, 10_000, u32::MAX as u64, u64::MAX] {
            let mut key = <Key as Default>::default();
            let expected = {
                let mut expected = [0x00; 32];
                expected[0..8].copy_from_slice(&test_value.to_le_bytes());
                expected
            };
            key.add_assign(*test_value);
            assert_eq!(key.to_bytes(), expected);
        }
    }
}
