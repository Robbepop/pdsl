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

//! The public raw interface towards the host Wasm engine.

use crate::{
    backend::{
        EnvBackend,
        ReturnFlags,
        TypedEnvBackend,
    },
    call::{
        utils::ReturnType,
        CallParams,
        CreateParams,
    },
    engine::{
        EnvInstance,
        OnInstance,
    },
    hash::{
        CryptoHash,
        HashOutput,
    },
    topics::Topics,
    types::{
        RentParams,
        RentStatus,
    },
    Environment,
    Result,
};
use ink_primitives::Key;

/// Returns the address of the caller of the executed contract.
///
/// # Errors
///
/// If the returned caller cannot be properly decoded.
pub fn caller<T>() -> Result<T::AccountId>
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::caller::<T>(instance)
    })
}

/// Returns the transferred balance for the contract execution.
///
/// # Errors
///
/// If the returned value cannot be properly decoded.
pub fn transferred_balance<T>() -> Result<T::Balance>
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::transferred_balance::<T>(instance)
    })
}

/// Returns the price for the specified amount of gas.
///
/// # Errors
///
/// If the returned value cannot be properly decoded.
pub fn weight_to_fee<T>(gas: u64) -> Result<T::Balance>
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::weight_to_fee::<T>(instance, gas)
    })
}

/// Returns the amount of gas left for the contract execution.
///
/// # Errors
///
/// If the returned value cannot be properly decoded.
pub fn gas_left<T>() -> Result<u64>
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::gas_left::<T>(instance)
    })
}

/// Returns the current block timestamp.
///
/// # Errors
///
/// If the returned value cannot be properly decoded.
pub fn block_timestamp<T>() -> Result<T::Timestamp>
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::block_timestamp::<T>(instance)
    })
}

/// Returns the account ID of the executed contract.
///
/// # Note
///
/// This method was formerly known as `address`.
///
/// # Errors
///
/// If the returned value cannot be properly decoded.
pub fn account_id<T>() -> Result<T::AccountId>
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::account_id::<T>(instance)
    })
}

/// Returns the balance of the executed contract.
///
/// # Errors
///
/// If the returned value cannot be properly decoded.
pub fn balance<T>() -> Result<T::Balance>
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::balance::<T>(instance)
    })
}

/// Returns the current rent allowance for the executed contract.
///
/// # Errors
///
/// If the returned value cannot be properly decoded.
pub fn rent_allowance<T>() -> Result<T::Balance>
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::rent_allowance::<T>(instance)
    })
}

/// Returns information needed for rent calculations.
///
/// # Errors
///
/// If the returned value cannot be properly decoded.
pub fn rent_params<T>() -> Result<RentParams<T>>
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::rent_params::<T>(instance)
    })
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
/// # Errors
///
/// If the returned value cannot be properly decoded.
pub fn rent_status<T>(at_refcount: Option<core::num::NonZeroU32>) -> Result<RentStatus<T>>
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::rent_status::<T>(instance, at_refcount)
    })
}

/// Returns the current block number.
///
/// # Errors
///
/// If the returned value cannot be properly decoded.
pub fn block_number<T>() -> Result<T::BlockNumber>
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::block_number::<T>(instance)
    })
}

/// Returns the minimum balance that is required for creating an account.
///
/// # Errors
///
/// If the returned value cannot be properly decoded.
pub fn minimum_balance<T>() -> Result<T::Balance>
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::minimum_balance::<T>(instance)
    })
}

/// Returns the tombstone deposit for the contracts chain.
///
/// # Errors
///
/// If the returned value cannot be properly decoded.
pub fn tombstone_deposit<T>() -> Result<T::Balance>
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::tombstone_deposit::<T>(instance)
    })
}

/// Emits an event with the given event data.
pub fn emit_event<T, Event>(event: Event)
where
    T: Environment,
    Event: Topics + scale::Encode,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::emit_event::<T, Event>(instance, event)
    })
}

/// Sets the rent allowance of the executed contract to the new value.
pub fn set_rent_allowance<T>(new_value: T::Balance)
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::set_rent_allowance::<T>(instance, new_value)
    })
}

/// Writes the value to the contract storage under the given key.
///
/// # Panics
///
/// - If the encode length of value exceeds the configured maximum value length of a storage entry.
pub fn set_contract_storage<V>(key: &Key, value: &V)
where
    V: scale::Encode,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        EnvBackend::set_contract_storage::<V>(instance, key, value)
    })
}

/// Returns the value stored under the given key in the contract's storage if any.
///
/// # Errors
///
/// - If the decoding of the typed value failed (`KeyNotFound`)
pub fn get_contract_storage<R>(key: &Key) -> Result<Option<R>>
where
    R: scale::Decode,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        EnvBackend::get_contract_storage::<R>(instance, key)
    })
}

/// Clears the contract's storage key entry.
pub fn clear_contract_storage(key: &Key) {
    <EnvInstance as OnInstance>::on_instance(|instance| {
        EnvBackend::clear_contract_storage(instance, key)
    })
}

/// Invokes a contract message.
///
/// # Note
///
/// - Prefer using this over [`eval_contract`] if possible. [`invoke_contract`]
///   will generally have a better performance since it won't try to fetch any results.
/// - This is a low level way to invoke another smart contract.
///   Prefer to use the ink! guided and type safe approach to using this.
///
/// # Errors
///
/// - If the called account does not exist.
/// - If the called account is not a contract.
/// - If the called contract is a tombstone.
/// - If arguments passed to the called contract message are invalid.
/// - If the called contract execution has trapped.
/// - If the called contract ran out of gas upon execution.
pub fn invoke_contract<T, Args>(params: &CallParams<T, Args, ()>) -> Result<()>
where
    T: Environment,
    Args: scale::Encode,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::invoke_contract::<T, Args>(instance, params)
    })
}

/// Evaluates a contract message and returns its result.
///
/// # Note
///
/// This is a low level way to evaluate another smart contract.
/// Prefer to use the ink! guided and type safe approach to using this.
///
/// # Errors
///
/// - If the called account does not exist.
/// - If the called account is not a contract.
/// - If the called contract is a tombstone.
/// - If arguments passed to the called contract message are invalid.
/// - If the called contract execution has trapped.
/// - If the called contract ran out of gas upon execution.
/// - If the returned value failed to decode properly.
pub fn eval_contract<T, Args, R>(params: &CallParams<T, Args, ReturnType<R>>) -> Result<R>
where
    T: Environment,
    Args: scale::Encode,
    R: scale::Decode,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::eval_contract::<T, Args, R>(instance, params)
    })
}

/// Instantiates another contract.
///
/// # Note
///
/// This is a low level way to instantiate another smart contract.
/// Prefer to use the ink! guided and type safe approach to using this.
///
/// # Errors
///
/// - If the code hash is invalid.
/// - If the arguments passed to the instantiation process are invalid.
/// - If the instantiation process traps.
/// - If the instantiation process runs out of gas.
/// - If given insufficient endowment.
/// - If the returned account ID failed to decode properly.
pub fn instantiate_contract<T, Args, Salt, C>(
    params: &CreateParams<T, Args, Salt, C>,
) -> Result<T::AccountId>
where
    T: Environment,
    Args: scale::Encode,
    Salt: AsRef<[u8]>,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::instantiate_contract::<T, Args, Salt, C>(instance, params)
    })
}

/// Restores a smart contract in tombstone state.
///
/// # Params
///
/// - `account_id`: Account ID of the to-be-restored contract.
/// - `code_hash`: Code hash of the to-be-restored contract.
/// - `rent_allowance`: Rent allowance of the restored contract
///                     upon successful restoration.
/// - `filtered_keys`: Storage keys to be excluded when calculating the tombstone hash,
///                    which is used to decide whether the original contract and the
///                    to-be-restored contract have matching storage.
///
/// # Usage
///
/// A smart contract that has too few funds to pay for its storage fees
/// can eventually be evicted. An evicted smart contract `C` leaves behind
/// a tombstone associated with a hash that has been computed partially
/// by its storage contents.
///
/// To restore contract `C` back to a fully working contract the normal
/// process is to write another contract `C2` with the only purpose to
/// eventually have the absolutely same contract storage as `C` did when
/// it was evicted.
/// For that purpose `C2` can use other storage keys that have not been in
/// use by contract `C`.
/// Once `C2` contract storage matches the storage of `C` when it was evicted
/// `C2` can invoke this method in order to initiate restoration of `C`.
/// A tombstone hash is calculated for `C2` and if it matches the tombstone
/// hash of `C` the restoration is going to be successful.
/// The `filtered_keys` argument can be used to ignore the extraneous keys
/// used by `C2` but not used by `C`.
///
/// The process of such a smart contract restoration can generally be very expensive.
///
/// # Note
///
/// - `filtered_keys` can be used to ignore certain storage regions
///   in the restorer contract to not influence the hash calculations.
/// - Does *not* perform restoration right away but defers it to the end of
///   the contract execution.
/// - Restoration is canceled if there is no tombstone in the destination
///   address or if the hashes don't match. No changes are made in this case.
pub fn restore_contract<T>(
    account_id: T::AccountId,
    code_hash: T::Hash,
    rent_allowance: T::Balance,
    filtered_keys: &[Key],
) where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::restore_contract::<T>(
            instance,
            account_id,
            code_hash,
            rent_allowance,
            filtered_keys,
        )
    })
}

/// Terminates the existence of the currently executed smart contract
/// without creating a tombstone.
///
/// This removes the calling account and transfers all remaining balance
/// to the given beneficiary.
///
/// No tombstone will be created, this function kills a contract completely!
///
/// # Note
///
/// This function never returns. Either the termination was successful and the
/// execution of the destroyed contract is halted. Or it failed during the termination
/// which is considered fatal and results in a trap and rollback.
pub fn terminate_contract<T>(beneficiary: T::AccountId) -> !
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::terminate_contract::<T>(instance, beneficiary)
    })
}

/// Transfers value from the contract to the destination account ID.
///
/// # Note
///
/// This is more efficient and simpler than the alternative to make a no-op
/// contract call or invoke a runtime function that performs the
/// transaction.
///
/// # Errors
///
/// - If the contract does not have sufficient funds.
/// - If the transfer had brought the sender's total balance below the
///   subsistence threshold.
pub fn transfer<T>(destination: T::AccountId, value: T::Balance) -> Result<()>
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::transfer::<T>(instance, destination, value)
    })
}

/// Returns the execution input to the executed contract and decodes it as `T`.
///
/// # Note
///
/// - The input is the 4-bytes selector followed by the arguments
///   of the called function in their SCALE encoded representation.
/// - No prior interaction with the environment must take place before
///   calling this procedure.
///
/// # Usage
///
/// Normally contracts define their own `enum` dispatch types respective
/// to their exported constructors and messages that implement `scale::Decode`
/// according to the constructors or messages selectors and their arguments.
/// These `enum` dispatch types are then given to this procedure as the `T`.
///
/// When using ink! users do not have to construct those enum dispatch types
/// themselves as they are normally generated by the ink! code generation
/// automatically.
///
/// # Errors
///
/// If the given `T` cannot be properly decoded from the expected input.
pub fn decode_input<T>() -> Result<T>
where
    T: scale::Decode,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        EnvBackend::decode_input::<T>(instance)
    })
}

/// Returns the value back to the caller of the executed contract.
///
/// # Note
///
/// This function  stops the execution of the contract immediately.
pub fn return_value<R>(return_flags: ReturnFlags, return_value: &R) -> !
where
    R: scale::Encode,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        EnvBackend::return_value::<R>(instance, return_flags, return_value)
    })
}

/// Returns a random hash seed and the block number since which it was determinable
/// by chain observers.
///
/// # Note
///
/// - The subject buffer can be used to further randomize the hash.
/// - Within the same execution returns the same random hash for the same subject.
///
/// # Errors
///
/// If the returned value cannot be properly decoded.
///
/// # Important
///
/// The returned seed should only be used to distinguish commitments made before
/// the returned block number. If the block number is too early (i.e. commitments were
/// made afterwards), then ensure no further commitments may be made and repeatedly
/// call this on later blocks until the block number returned is later than the latest
/// commitment.
pub fn random<T>(subject: &[u8]) -> Result<(T::Hash, T::BlockNumber)>
where
    T: Environment,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        TypedEnvBackend::random::<T>(instance, subject)
    })
}

/// Appends the given message to the debug message buffer.
pub fn debug_message(message: &str) {
    <EnvInstance as OnInstance>::on_instance(|instance| {
        EnvBackend::debug_message(instance, message)
    })
}

/// Conducts the crypto hash of the given input and stores the result in `output`.
///
/// # Example
///
/// ```
/// # #[cfg(not(miri))]
/// # {
/// use ink_env::hash::{Sha2x256, HashOutput};
/// let input: &[u8] = &[13, 14, 15];
/// let mut output = <Sha2x256 as HashOutput>::Type::default(); // 256-bit buffer
/// let hash  = ink_env::hash_bytes::<Sha2x256>(input, &mut output);
/// # }
/// ```
pub fn hash_bytes<H>(input: &[u8], output: &mut <H as HashOutput>::Type)
where
    H: CryptoHash,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        instance.hash_bytes::<H>(input, output)
    })
}

/// Conducts the crypto hash of the given encoded input and stores the result in `output`.
///
/// # Example
///
/// ```
/// # #[cfg(not(miri))]
/// # {
/// # use ink_env::hash::{Sha2x256, HashOutput};
/// const EXPECTED: [u8; 32] = [
///   243, 242, 58, 110, 205, 68, 100, 244, 187, 55, 188, 248,  29, 136, 145, 115,
///   186, 134, 14, 175, 178, 99, 183,  21,   4, 94,  92,  69, 199, 207, 241, 179,
/// ];
/// let encodable = (42, "foo", true); // Implements `scale::Encode`
/// let mut output = <Sha2x256 as HashOutput>::Type::default(); // 256-bit buffer
/// ink_env::hash_encoded::<Sha2x256, _>(&encodable, &mut output);
/// assert_eq!(output, EXPECTED);
/// # }
/// ```
pub fn hash_encoded<H, T>(input: &T, output: &mut <H as HashOutput>::Type)
where
    H: CryptoHash,
    T: scale::Encode,
{
    <EnvInstance as OnInstance>::on_instance(|instance| {
        instance.hash_encoded::<H, T>(input, output)
    })
}
