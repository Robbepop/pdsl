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

static TEST_INPUT: &[u8] = b"DEAD_BEEF";

#[test]
fn test_hash_keccak_256() {
    let mut output = [0x00_u8; 32];
    crate::hash_bytes::<crate::hash::Keccak256>(TEST_INPUT, &mut output);
    assert_eq!(
        output,
        [
            24, 230, 209, 59, 127, 30, 158, 244, 60, 177, 132, 150, 167, 244, 64, 69,
            184, 123, 185, 44, 211, 199, 208, 179, 14, 64, 126, 140, 217, 69, 36, 216
        ]
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_hash_sha2_256() {
    let mut output = [0x00_u8; 32];
    crate::hash_bytes::<crate::hash::Sha2x256>(TEST_INPUT, &mut output);
    assert_eq!(
        output,
        [
            136, 15, 25, 218, 88, 54, 49, 152, 115, 168, 147, 189, 207, 171, 243, 129,
            161, 76, 15, 141, 197, 106, 111, 213, 19, 197, 133, 219, 181, 233, 195, 120
        ]
    );
}

#[test]
fn test_hash_blake2_256() {
    let mut output = [0x00_u8; 32];
    crate::hash_bytes::<crate::hash::Blake2x256>(TEST_INPUT, &mut output);
    assert_eq!(
        output,
        [
            244, 247, 235, 182, 194, 161, 28, 69, 34, 106, 237, 7, 57, 87, 190, 12, 92,
            171, 91, 176, 135, 52, 247, 94, 8, 112, 94, 183, 140, 101, 208, 120
        ]
    );
}

#[test]
fn test_hash_blake2_128() {
    let mut output = [0x00_u8; 16];
    crate::hash_bytes::<crate::hash::Blake2x128>(TEST_INPUT, &mut output);
    assert_eq!(
        output,
        [180, 158, 48, 21, 171, 163, 217, 175, 145, 160, 25, 159, 213, 142, 103, 242]
    );
}
