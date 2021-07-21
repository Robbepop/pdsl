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

use core::marker::PhantomData;
use ink_env::{
    call::{
        utils::ReturnType,
        CallParams,
        CreateParams,
    },
    hash::{
        CryptoHash,
        HashOutput,
    },
    Environment,
    RentParams,
    RentStatus,
    Result,
};
use ink_primitives::Key;

use crate::ChainExtensionInstance;

/// The environment of the compiled ink! smart contract.
pub trait ContractEnv {
    /// The environment type.
    type Env: ::ink_env::Environment;
}

/// Simplifies interaction with the host environment via `self`.
///
/// # Note
///
/// This is generally implemented for storage structs that include
/// their environment in order to allow the different dispatch functions
/// to use it for returning the contract's output.
pub trait Env {
    /// The access wrapper.
    type EnvAccess;

    /// Accesses the environment with predefined environmental types.
    fn env(self) -> Self::EnvAccess;
}

/// Simplifies interaction with the host environment via `Self`.
///
/// # Note
///
/// This is generally implemented for storage structs that include
/// their environment in order to allow the different dispatch functions
/// to use it for returning the contract's output.
pub trait StaticEnv {
    /// The access wrapper.
    type EnvAccess;

    /// Accesses the environment with predefined environmental types.
    fn env() -> Self::EnvAccess;
}

/// A typed accessor to the environment.
///
/// This allows ink! messages to make use of the environment efficiently
/// and user friendly while also maintaining access invariants.
#[derive(Copy, Clone)]
pub struct EnvAccess<'a, T> {
    /// Tricks the Rust compiler into thinking that we use `T`.
    marker: PhantomData<fn() -> &'a T>,
}

impl<'a, T> Default for EnvAccess<'a, T> {
    #[inline]
    fn default() -> Self {
        Self {
            marker: Default::default(),
        }
    }
}

impl<'a, E> core::fmt::Debug for EnvAccess<'a, E> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("EnvAccess").finish()
    }
}

impl<'a, T> EnvAccess<'a, T>
where
    T: Environment,
    <T as Environment>::ChainExtension: ChainExtensionInstance,
{
    /// Allows to call one of the available defined chain extension methods.
    pub fn extension(
        self,
    ) -> <<T as Environment>::ChainExtension as ChainExtensionInstance>::Instance {
        <<T as Environment>::ChainExtension as ChainExtensionInstance>::instantiate()
    }
}

impl<'a, T> EnvAccess<'a, T>
where
    T: Environment,
{
    /// Returns the address of the caller of the executed contract.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// #
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// #[ink(message)]
    /// pub fn call_me(&self) {
    ///     let caller = self.env().caller();
    ///     ink_env::debug_println!("got a call from {:?}", &caller);
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::caller`]
    pub fn caller(self) -> T::AccountId {
        ink_env::caller::<T>().expect("couldn't decode caller")
    }

    /// Returns the transferred balance for the contract execution.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// #
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// /// Allows funding the contract. Prints a debug message with the transferred balance.
    /// #[ink(message, payable)]
    /// pub fn fund(&self) {
    ///     let caller = self.env().caller();
    ///     let value = self.env().transferred_balance();
    ///     ink_env::debug_println!("thanks for the funding of {:?} from {:?}", value, caller);
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::transferred_balance`]
    pub fn transferred_balance(self) -> T::Balance {
        ink_env::transferred_balance::<T>().expect("couldn't decode transferred balance")
    }

    /// Returns the price for the specified amount of gas.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// #
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// /// Returns a tuple of
    /// ///   - the result of adding the `rhs` to the `lhs`
    /// ///   - the gas costs of this addition operation
    /// ///   - the price for the gas
    /// #[ink(message)]
    /// pub fn addition_gas_cost(&self, rhs: i32, lhs: i32) -> (i32, u64, Balance) {
    ///     let before = self.env().gas_left();
    ///     let result = rhs + lhs;
    ///     let after = self.env().gas_left();
    ///     let gas_used = after - before;
    ///     let gas_cost = self.env().weight_to_fee(gas_used);
    ///     (result, gas_used, gas_cost)
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::weight_to_fee`]
    pub fn weight_to_fee(self, gas: u64) -> T::Balance {
        ink_env::weight_to_fee::<T>(gas).expect("couldn't decode weight fee")
    }

    /// Returns the amount of gas left for the contract execution.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// /// Returns a tuple of
    /// ///   - the result of adding the `rhs` to the `lhs` and
    /// ///   - the gas used for this addition operation.
    /// #[ink(message)]
    /// pub fn addition_gas_cost(&self, rhs: i32, lhs: i32) -> (i32, u64) {
    ///     let before = self.env().gas_left();
    ///     let result = rhs + lhs;
    ///     let after = self.env().gas_left();
    ///     (result, after - before)
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::gas_left`]
    pub fn gas_left(self) -> u64 {
        ink_env::gas_left::<T>().expect("couldn't decode gas left")
    }

    /// Returns the timestamp of the current block.
    ///
    /// # Example
    ///
    /// ```
    /// use ink_lang as ink;
    ///
    /// #[ink::contract]
    /// pub mod my_contract {
    ///     #[ink(storage)]
    ///     pub struct MyContract {
    ///         last_invocation: Timestamp
    ///     }
    ///
    ///     impl MyContract {
    ///         #[ink(constructor)]
    ///         pub fn new() -> Self {
    ///             Self {
    ///                 last_invocation: Self::env().block_timestamp()
    ///             }
    ///         }
    ///
    ///         /// The function can be executed at most once every 100 blocks.
    ///         #[ink(message)]
    ///         pub fn execute_me(&mut self) {
    ///             let now = self.env().block_timestamp();
    ///             assert!(now - self.last_invocation > 100);
    ///             self.last_invocation = now;
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::block_timestamp`]
    pub fn block_timestamp(self) -> T::Timestamp {
        ink_env::block_timestamp::<T>().expect("couldn't decode block time stamp")
    }

    /// Returns the account ID of the executed contract.
    ///
    /// # Example
    ///
    /// ```
    /// use ink_lang as ink;
    ///
    /// #[ink::contract]
    /// pub mod only_owner {
    ///     #[ink(storage)]
    ///     pub struct OnlyOwner {
    ///         owner: AccountId,
    ///         value: u32,
    ///     }
    ///
    ///     impl OnlyOwner {
    ///         #[ink(constructor)]
    ///         pub fn new() -> Self {
    ///             Self {
    ///                 owner: Self::env().caller(),
    ///                 value: 0,
    ///             }
    ///         }
    ///
    ///         /// Allows incrementing the contract's `value` only
    ///         /// for the owner (i.e. the account which instantiated
    ///         /// this contract.
    ///         ///
    ///         /// The contract panics if the caller is not the owner.
    ///         #[ink(message)]
    ///         pub fn increment(&mut self) {
    ///             let caller = self.env().caller();
    ///             assert!(self.owner == caller);
    ///             self.value = self.value + 1;
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::account_id`]
    pub fn account_id(self) -> T::AccountId {
        ink_env::account_id::<T>().expect("couldn't decode contract account ID")
    }

    /// Returns the balance of the executed contract.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// /// Returns the contract's balance.
    /// #[ink(message)]
    /// pub fn my_balance(&self) -> Balance {
    ///     self.env().balance()
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::balance`]
    pub fn balance(self) -> T::Balance {
        ink_env::balance::<T>().expect("couldn't decode contract balance")
    }

    /// Returns the current rent allowance for the executed contract.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// /// Returns the amount of the contract's balance which
    /// /// can be used for paying rent.
    /// #[ink(message)]
    /// pub fn rent_allowance(&self) -> Balance {
    ///     self.env().rent_allowance()
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::rent_allowance`]
    pub fn rent_allowance(self) -> T::Balance {
        ink_env::rent_allowance::<T>().expect("couldn't decode contract rent allowance")
    }

    /// Sets the rent allowance of the executed contract to the new value.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// /// Limits the amount of contract balance which can be used for paying rent
    /// /// to half of the contract's total balance.
    /// #[ink(message)]
    /// pub fn limit_rent_allowance(&self) {
    ///     self.env().set_rent_allowance(self.env().balance() / 2);
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::set_rent_allowance`]
    pub fn set_rent_allowance(self, new_value: T::Balance) {
        ink_env::set_rent_allowance::<T>(new_value)
    }

    /// Returns information needed for rent calculations.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// /// Returns the balance every contract needs to deposit on this chain
    /// /// to stay alive indefinitely.
    /// #[ink(message)]
    /// pub fn deposit_per_contract(&self) -> Balance {
    ///     self.env().rent_params().deposit_per_contract
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::RentParams`]
    pub fn rent_params(self) -> RentParams<T> {
        ink_env::rent_params::<T>().expect("couldn't decode contract rent params")
    }

    /// Returns information about the required deposit and resulting rent.
    ///
    /// # Parameters
    ///
    /// - `at_refcount`: The `refcount` assumed for the returned `custom_refcount_*` fields.
    ///   If `None` is supplied the `custom_refcount_*` fields will also be `None`.
    ///
    ///   The `current_*` fields of `RentStatus` do **not** consider changes to the code's
    ///   `refcount` made during the currently running call.
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::RentStatus`]
    pub fn rent_status(
        self,
        at_refcount: Option<core::num::NonZeroU32>,
    ) -> RentStatus<T> {
        ink_env::rent_status::<T>(at_refcount)
            .expect("couldn't decode contract rent params")
    }

    /// Returns the current block number.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// #[ink(message)]
    /// pub fn block_number(&self) -> BlockNumber {
    ///     self.env().block_number()
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::block_number`]
    pub fn block_number(self) -> T::BlockNumber {
        ink_env::block_number::<T>().expect("couldn't decode block number")
    }

    /// Returns the minimum balance that is required for creating an account.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// #[ink(message)]
    /// pub fn minimum_balance(&self) -> Balance {
    ///     self.env().minimum_balance()
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::minimum_balance`]
    pub fn minimum_balance(self) -> T::Balance {
        ink_env::minimum_balance::<T>().expect("couldn't decode minimum account balance")
    }

    /// Returns the tombstone deposit for the contracts chain.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// #[ink(message)]
    /// pub fn tombstone_deposit(&self) -> Balance {
    ///     self.env().tombstone_deposit()
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::tombstone_deposit`]
    pub fn tombstone_deposit(self) -> T::Balance {
        ink_env::tombstone_deposit::<T>().expect("couldn't decode tombstone deposits")
    }

    /// Instantiates another contract.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// #     use ink_lang as ink;
    /// #     #[ink::contract(compile_as_dependency = true)]
    /// #     pub mod other_contract {
    /// #         #[ink(storage)]
    /// #         pub struct OtherContract { }
    /// #
    /// #         impl OtherContract {
    /// #             #[ink(constructor)]
    /// #             pub fn new() -> Self {
    /// #                 Self {}
    /// #             }
    /// #
    /// #             #[ink(message)]
    /// #             pub fn some_operation(&self) {
    /// #                 // ...
    /// #             }
    /// #         }
    /// #     }
    /// #
    /// use ink_env::{
    ///     DefaultEnvironment,
    ///     call::{build_create, Selector, ExecutionInput}
    /// };
    /// use other_contract::OtherContract;
    /// #
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    ///
    /// /// Instantiates another contract.
    /// #[ink(message)]
    /// pub fn instantiate_contract(&self) -> AccountId {
    ///     let create_params = build_create::<DefaultEnvironment, OtherContract>()
    ///         .code_hash(Hash::from([0x42; 32]))
    ///         .gas_limit(4000)
    ///         .endowment(25)
    ///         .exec_input(
    ///             ExecutionInput::new(Selector::new([0xCA, 0xFE, 0xBA, 0xBE]))
    ///                 .push_arg(42)
    ///                 .push_arg(true)
    ///                 .push_arg(&[0x10u8; 32])
    ///             )
    ///         .salt_bytes(&[0xCA, 0xFE, 0xBA, 0xBE])
    ///         .params();
    ///     self.env().instantiate_contract(&create_params).expect("instantiation must succeed")
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// See [our `delegator` example](https://github.com/paritytech/ink/tree/master/examples/delegator)
    /// for a complete contract example.
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::instantiate_contract`]
    pub fn instantiate_contract<Args, Salt, C>(
        self,
        params: &CreateParams<T, Args, Salt, C>,
    ) -> Result<T::AccountId>
    where
        Args: scale::Encode,
        Salt: AsRef<[u8]>,
    {
        ink_env::instantiate_contract::<T, Args, Salt, C>(params)
    }

    /// Invokes a contract message without fetching its result.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// use ink_env::{
    ///     DefaultEnvironment,
    ///     call::{build_call, Selector, ExecutionInput}
    /// };
    ///
    /// #
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// /// Invokes another contract message without fetching the result.
    /// #[ink(message)]
    /// pub fn invoke_contract(&self) {
    ///     let call_params = build_call::<DefaultEnvironment>()
    ///         .callee(AccountId::from([0x42; 32]))
    ///         .gas_limit(5000)
    ///         .transferred_value(10)
    ///         .exec_input(
    ///             ExecutionInput::new(Selector::new([0xCA, 0xFE, 0xBA, 0xBE]))
    ///                 .push_arg(42)
    ///                 .push_arg(true)
    ///                 .push_arg(&[0x10u8; 32])
    ///         )
    ///         .returns::<()>()
    ///         .params();
    ///     self.env().invoke_contract(&call_params).expect("call invocation must succeed");
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::invoke_contract`]
    pub fn invoke_contract<Args>(self, params: &CallParams<T, Args, ()>) -> Result<()>
    where
        Args: scale::Encode,
    {
        ink_env::invoke_contract::<T, Args>(params)
    }

    /// Evaluates a contract message and returns its result.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// use ink_env::{
    ///     DefaultEnvironment,
    ///     call::{build_call, Selector, ExecutionInput, utils::ReturnType}
    /// };
    ///
    /// #
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// /// Evaluates a contract message and fetches the result.
    /// #[ink(message)]
    /// pub fn evaluate_contract(&self) -> i32 {
    ///     let call_params = build_call::<DefaultEnvironment>()
    ///         .callee(AccountId::from([0x42; 32]))
    ///         .gas_limit(5000)
    ///         .transferred_value(10)
    ///         .exec_input(
    ///             ExecutionInput::new(Selector::new([0xCA, 0xFE, 0xBA, 0xBE]))
    ///                 .push_arg(42)
    ///                 .push_arg(true)
    ///                 .push_arg(&[0x10u8; 32])
    ///         )
    ///         .returns::<ReturnType<i32>>()
    ///         .params();
    ///     self.env().eval_contract(&call_params).expect("call invocation must succeed")
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::eval_contract`]
    pub fn eval_contract<Args, R>(
        self,
        params: &CallParams<T, Args, ReturnType<R>>,
    ) -> Result<R>
    where
        Args: scale::Encode,
        R: scale::Decode,
    {
        ink_env::eval_contract::<T, Args, R>(params)
    }

    /// Restores a smart contract from its tombstone state.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// /// Simple resurrection of a contract.
    /// #[ink(message)]
    /// pub fn resurrect(&self, contract: AccountId) {
    ///     self.env().restore_contract(
    ///         contract,
    ///         Hash::from([0x42; 32]),
    ///         1000,
    ///         &[]
    ///     )
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::restore_contract`]
    pub fn restore_contract(
        self,
        account_id: T::AccountId,
        code_hash: T::Hash,
        rent_allowance: T::Balance,
        filtered_keys: &[Key],
    ) {
        ink_env::restore_contract::<T>(
            account_id,
            code_hash,
            rent_allowance,
            filtered_keys,
        )
    }

    /// Terminates the existence of a contract without creating a tombstone.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// /// Terminates with the caller as beneficiary.
    /// #[ink(message)]
    /// pub fn terminate_me(&mut self) {
    ///     self.env().terminate_contract(self.env().caller());
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::terminate_contract`]
    pub fn terminate_contract(self, beneficiary: T::AccountId) -> ! {
        ink_env::terminate_contract::<T>(beneficiary)
    }

    /// Transfers value from the contract to the destination account ID.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// /// Transfers the token amount ten to the caller.
    /// #[ink(message)]
    /// pub fn give_me_ten(&mut self) {
    ///     let value: Balance = 10;
    ///     self.env().transfer(self.env().caller(), value).expect("transfer failed");
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::transfer`]
    pub fn transfer(self, destination: T::AccountId, value: T::Balance) -> Result<()> {
        ink_env::transfer::<T>(destination, value)
    }

    /// Returns a random hash seed.
    ///
    /// # Example
    ///
    /// ```
    /// # use ink_lang as ink;
    /// # #[ink::contract]
    /// # pub mod my_contract {
    /// #     #[ink(storage)]
    /// #     pub struct MyContract { }
    /// #
    /// #     impl MyContract {
    /// #         #[ink(constructor)]
    /// #         pub fn new() -> Self {
    /// #             Self {}
    /// #         }
    /// #
    /// #[ink(message)]
    /// pub fn random_bool(&self) -> bool {
    ///     let additional_randomness = b"seed";
    ///     let (hash, _block_number) = self.env().random(additional_randomness);
    ///     hash.as_ref()[0] != 0
    /// }
    /// #
    /// #     }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::random`]
    pub fn random(self, subject: &[u8]) -> (T::Hash, T::BlockNumber) {
        ink_env::random::<T>(subject).expect("couldn't decode randomized hash")
    }

    /// Computes the hash of the given bytes using the cryptographic hash `H`.
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(not(miri))]
    /// # {
    /// use ink_env::hash::{Sha2x256, HashOutput};
    ///
    /// let input: &[u8] = &[13, 14, 15];
    /// let mut output = <Sha2x256 as HashOutput>::Type::default(); // 256-bit buffer
    /// let hash  = ink_env::hash_bytes::<Sha2x256>(input, &mut output);
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::hash_bytes`]
    pub fn hash_bytes<H>(self, input: &[u8]) -> <H as HashOutput>::Type
    where
        H: CryptoHash,
    {
        let mut output = <H as HashOutput>::Type::default();
        ink_env::hash_bytes::<H>(input, &mut output);
        output
    }

    /// Computes the hash of the given SCALE encoded value using the cryptographic hash `H`.
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(not(miri))]
    /// # {
    /// use ink_env::hash::{Sha2x256, HashOutput};
    ///
    /// let encodable = (42, "foo", true); // Implements `scale::Encode`
    /// let mut output = <Sha2x256 as HashOutput>::Type::default(); // 256-bit buffer
    /// ink_env::hash_encoded::<Sha2x256, _>(&encodable, &mut output);
    ///
    /// const EXPECTED: [u8; 32] = [
    ///   243, 242, 58, 110, 205, 68, 100, 244, 187, 55, 188, 248,  29, 136, 145, 115,
    ///   186, 134, 14, 175, 178, 99, 183,  21,   4, 94,  92,  69, 199, 207, 241, 179,
    /// ];
    /// assert_eq!(output, EXPECTED);
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details visit: [`ink_env::hash_encoded`]
    pub fn hash_encoded<H, V>(self, value: &V) -> <H as HashOutput>::Type
    where
        H: CryptoHash,
        V: scale::Encode,
    {
        let mut output = <H as HashOutput>::Type::default();
        ink_env::hash_encoded::<H, V>(value, &mut output);
        output
    }
}
