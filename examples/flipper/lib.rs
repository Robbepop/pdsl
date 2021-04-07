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

#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;
//use ink_prelude;

#[ink::contract]
pub mod flipper {
    #[ink(storage)]
    pub struct Flipper {
        value: bool,
        //hmap: ink_storage::collections::HashMap<String, AccountId>,
        //hmap: ink_storage::collections::HashMap<ink_prelude::string::String, AccountId>,
        foo: String,
    }

    impl Flipper {
        /// Creates a new flipper smart contract initialized with the given value.
        #[ink(constructor)]
        pub fn new(init_value: bool) -> Self {
            //Self { value: init_value, hmap: ink_storage::collections::HashMap::new() }
            Self { value: init_value, foo: String::from("") }
        }

        /// Creates a new flipper smart contract initialized to `false`.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(Default::default())
        }

        /// Flips the current value of the Flipper's bool.
        ///
        /// If the contract still has to pay rent we require a small
        /// additional payment.
        ///
        /// If the contract's deposit is high enough so that it doesn't
        /// have to pay rent we don't require an additional payment.
        #[ink(message)]
        pub fn flip(&mut self) {
            //let rent = self.env().rent_params();
            //let needed_to_stay_alive = rent.total_balance; // TODO

            self.value = !self.value;
        }

        /// Returns the current value of the Flipper's bool.
        #[ink(message)]
        pub fn get(&self) -> bool {
            //let _ = self.env().rent_params();
            self.value
        }
        /*

        /// Returns the current value of the Flipper's bool.
        #[ink(message)]
        pub fn rent(&self) -> Balance {
            let rent = self.env().rent_params();
            let needed_to_stay_alive = rent.total_balance;
            needed_to_stay_alive
        }

         */
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn default_works() {
            let flipper = Flipper::default();
            assert_eq!(flipper.get(), false);
        }

        #[test]
        fn it_works() {
            let mut flipper = Flipper::new(false);
            assert_eq!(flipper.get(), false);
            flipper.flip();
            assert_eq!(flipper.get(), true);
        }
    }
}
