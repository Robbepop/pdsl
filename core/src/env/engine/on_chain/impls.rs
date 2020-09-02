// Copyright 2019-2020 Parity Technologies (UK) Ltd.
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
    ext,
    EnvInstance,
    Error as ExtError,
    ScopedBuffer,
};
use crate::env::{
    call::{
        CallParams,
        InstantiateParams,
        ReturnType,
    },
    Env,
    EnvError,
    EnvTypes,
    Result,
    ReturnFlags,
    Topics,
    TypedEnv,
};
use ink_primitives::Key;

impl From<ext::Error> for EnvError {
    fn from(ext_error: ext::Error) -> Self {
        match ext_error {
            ext::Error::UnknownError => Self::UnknownError,
            ext::Error::CalleeTrapped => Self::CalleeTrapped,
            ext::Error::CalleeReverted => Self::CalleeReverted,
            ext::Error::KeyNotFound => Self::KeyNotFound,
            ext::Error::BelowSubsistenceThreshold => Self::BelowSubsistenceThreshold,
            ext::Error::TransferFailed => Self::TransferFailed,
            ext::Error::NewContractNotFunded => Self::NewContractNotFunded,
            ext::Error::CodeNotFound => Self::CodeNotFound,
            ext::Error::NotCallable => Self::NotCallable,
        }
    }
}

impl EnvInstance {
    /// Returns a new scoped buffer for the entire scope of the static 16kB buffer.
    fn scoped_buffer(&mut self) -> ScopedBuffer {
        ScopedBuffer::from(&mut self.buffer[..])
    }

    /// Returns the contract property value.
    fn get_property<T>(&mut self, ext_fn: fn(output: &mut &mut [u8])) -> Result<T>
    where
        T: scale::Decode,
    {
        let full_scope = &mut self.scoped_buffer().take_rest();
        ext_fn(full_scope);
        scale::Decode::decode(&mut &full_scope[..]).map_err(Into::into)
    }

    /// Reusable implementation for invoking another contract message.
    fn invoke_contract_impl<T, Args, RetType, R>(
        &mut self,
        params: &CallParams<T, Args, RetType>,
    ) -> Result<R>
    where
        T: EnvTypes,
        Args: scale::Encode,
        R: scale::Decode,
    {
        let mut scope = self.scoped_buffer();
        let gas_limit = params.gas_limit();
        let enc_callee = scope.take_encoded(params.callee());
        let enc_transferred_value = scope.take_encoded(params.transferred_value());
        let enc_input = scope.take_encoded(params.input_data());
        let output = &mut scope.take_rest();
        ext::call(
            enc_callee,
            gas_limit,
            enc_transferred_value,
            enc_input,
            output,
        )?;
        let decoded = scale::Decode::decode(&mut &output[..])?;
        Ok(decoded)
    }
}

impl Env for EnvInstance {
    fn set_contract_storage<V>(&mut self, key: &Key, value: &V)
    where
        V: scale::Encode,
    {
        let buffer = self.scoped_buffer().take_encoded(value);
        ext::set_storage(key.as_bytes(), &buffer[..]);
    }

    fn get_contract_storage<R>(&mut self, key: &Key) -> Result<Option<R>>
    where
        R: scale::Decode,
    {
        let output = &mut self.scoped_buffer().take_rest();
        match ext::get_storage(key.as_bytes(), output) {
            Ok(_) => (),
            Err(ExtError::KeyNotFound) => return Ok(None),
            Err(_) => panic!("encountered unexpected error"),
        }
        let decoded = scale::Decode::decode(&mut &output[..])?;
        Ok(Some(decoded))
    }

    fn clear_contract_storage(&mut self, key: &Key) {
        ext::clear_storage(key.as_bytes())
    }

    fn decode_input<T>(&mut self) -> Result<T>
    where
        T: scale::Decode,
    {
        self.get_property::<T>(ext::input)
    }

    fn return_value<R>(&mut self, flags: ReturnFlags, return_value: &R) -> !
    where
        R: scale::Encode,
    {
        let mut scope = self.scoped_buffer();
        let enc_return_value = scope.take_encoded(return_value);
        ext::return_value(flags, enc_return_value);
    }

    fn println(&mut self, content: &str) {
        ext::println(content)
    }

    fn hash_keccak_256(input: &[u8], output: &mut [u8; 32]) {
        ext::hash_keccak_256(input, output)
    }

    fn hash_blake2_256(input: &[u8], output: &mut [u8; 32]) {
        ext::hash_blake2_256(input, output)
    }

    fn hash_blake2_128(input: &[u8], output: &mut [u8; 16]) {
        ext::hash_blake2_128(input, output)
    }

    fn hash_sha2_256(input: &[u8], output: &mut [u8; 32]) {
        ext::hash_sha2_256(input, output)
    }

    #[cfg(feature = "ink-unstable-chain-extensions")]
    fn call_chain_extension<I, O>(
        &mut self,
        func_id: u32,
        input: &I,
    ) -> Result<O>
    where
        I: scale::Encode,
        O: scale::Decode,
    {
        let mut scope = self.scoped_buffer();
        let enc_input = scope.take_encoded(input);
        let output = &mut scope.take_rest();
        ext::call_chain_extension(func_id, enc_input, output)?;
        scale::Decode::decode(&mut &output[..]).map_err(Into::into)
    }
}

impl TypedEnv for EnvInstance {
    fn caller<T: EnvTypes>(&mut self) -> Result<T::AccountId> {
        self.get_property::<T::AccountId>(ext::caller)
    }

    fn transferred_balance<T: EnvTypes>(&mut self) -> Result<T::Balance> {
        self.get_property::<T::Balance>(ext::value_transferred)
    }

    fn gas_left<T: EnvTypes>(&mut self) -> Result<T::Balance> {
        self.get_property::<T::Balance>(ext::gas_left)
    }

    fn block_timestamp<T: EnvTypes>(&mut self) -> Result<T::Timestamp> {
        self.get_property::<T::Timestamp>(ext::now)
    }

    fn account_id<T: EnvTypes>(&mut self) -> Result<T::AccountId> {
        self.get_property::<T::AccountId>(ext::address)
    }

    fn balance<T: EnvTypes>(&mut self) -> Result<T::Balance> {
        self.get_property::<T::Balance>(ext::balance)
    }

    fn rent_allowance<T: EnvTypes>(&mut self) -> Result<T::Balance> {
        self.get_property::<T::Balance>(ext::rent_allowance)
    }

    fn block_number<T: EnvTypes>(&mut self) -> Result<T::BlockNumber> {
        self.get_property::<T::BlockNumber>(ext::block_number)
    }

    fn minimum_balance<T: EnvTypes>(&mut self) -> Result<T::Balance> {
        self.get_property::<T::Balance>(ext::minimum_balance)
    }

    fn tombstone_deposit<T: EnvTypes>(&mut self) -> Result<T::Balance> {
        self.get_property::<T::Balance>(ext::tombstone_deposit)
    }

    fn emit_event<T, Event>(&mut self, event: Event)
    where
        T: EnvTypes,
        Event: Topics<T> + scale::Encode,
    {
        let mut scope = self.scoped_buffer();
        let enc_topics = scope.take_encoded(&event.topics());
        let enc_data = scope.take_encoded(&event);
        ext::deposit_event(enc_topics, enc_data);
    }

    fn set_rent_allowance<T>(&mut self, new_value: T::Balance)
    where
        T: EnvTypes,
    {
        let buffer = self.scoped_buffer().take_encoded(&new_value);
        ext::set_rent_allowance(&buffer[..])
    }

    fn invoke_contract<T, Args>(
        &mut self,
        call_params: &CallParams<T, Args, ()>,
    ) -> Result<()>
    where
        T: EnvTypes,
        Args: scale::Encode,
    {
        self.invoke_contract_impl(call_params)
    }

    fn eval_contract<T, Args, R>(
        &mut self,
        call_params: &CallParams<T, Args, ReturnType<R>>,
    ) -> Result<R>
    where
        T: EnvTypes,
        Args: scale::Encode,
        R: scale::Decode,
    {
        self.invoke_contract_impl(call_params)
    }

    fn instantiate_contract<T, Args, C>(
        &mut self,
        params: &InstantiateParams<T, Args, C>,
    ) -> Result<T::AccountId>
    where
        T: EnvTypes,
        Args: scale::Encode,
    {
        let mut scoped = self.scoped_buffer();
        let gas_limit = params.gas_limit();
        let enc_code_hash = scoped.take_encoded(params.code_hash());
        let enc_endowment = scoped.take_encoded(params.endowment());
        let enc_input = scoped.take_encoded(params.input_data());
        // We support `AccountId` types with an encoding that requires up to
        // 1024 bytes. Beyond that limit ink! contracts will trap for now.
        // In the default configuration encoded `AccountId` require 32 bytes.
        let out_address = &mut scoped.take(1024);
        let out_return_value = &mut scoped.take_rest();
        // We currently do nothing with the `out_return_value` buffer.
        // This should change in the future but for that we need to add support
        // for constructors that may return values.
        // This is useful to support fallible constructors for example.
        ext::instantiate(
            enc_code_hash,
            gas_limit,
            enc_endowment,
            enc_input,
            out_address,
            out_return_value,
        )?;
        let account_id = scale::Decode::decode(&mut &out_address[..])?;
        Ok(account_id)
    }

    fn restore_contract<T>(
        &mut self,
        account_id: T::AccountId,
        code_hash: T::Hash,
        rent_allowance: T::Balance,
        filtered_keys: &[Key],
    ) where
        T: EnvTypes,
    {
        let mut scope = self.scoped_buffer();
        let enc_account_id = scope.take_encoded(&account_id);
        let enc_code_hash = scope.take_encoded(&code_hash);
        let enc_rent_allowance = scope.take_encoded(&rent_allowance);
        ext::restore_to(
            enc_account_id,
            enc_code_hash,
            enc_rent_allowance,
            filtered_keys,
        );
    }

    fn terminate_contract<T>(&mut self, beneficiary: T::AccountId) -> !
    where
        T: EnvTypes,
    {
        let buffer = self.scoped_buffer().take_encoded(&beneficiary);
        ext::terminate(&buffer[..]);
    }

    fn transfer<T>(&mut self, destination: T::AccountId, value: T::Balance) -> Result<()>
    where
        T: EnvTypes,
    {
        let mut scope = self.scoped_buffer();
        let enc_destination = scope.take_encoded(&destination);
        let enc_value = scope.take_encoded(&value);
        ext::transfer(enc_destination, enc_value).map_err(Into::into)
    }

    fn weight_to_fee<T: EnvTypes>(&mut self, gas: u64) -> Result<T::Balance> {
        let output = &mut self.scoped_buffer().take_rest();
        ext::weight_to_fee(gas, output);
        scale::Decode::decode(&mut &output[..]).map_err(Into::into)
    }

    fn random<T>(&mut self, subject: &[u8]) -> Result<T::Hash>
    where
        T: EnvTypes,
    {
        let mut scope = self.scoped_buffer();
        let enc_subject = scope.take_bytes(subject);
        let output = &mut scope.take_rest();
        ext::random(enc_subject, output);
        scale::Decode::decode(&mut &output[..]).map_err(Into::into)
    }
}
