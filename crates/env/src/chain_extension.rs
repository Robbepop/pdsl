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

//! Definitions and utilities for calling chain extension methods.
//!
//! Users should not use these types and definitions directly but rather use the provided
//! `#[ink::chain_extension]` procedural macro defined in the `ink_lang` crate.

use core::marker::PhantomData;

use crate::{
    backend::EnvBackend,
    engine::{
        EnvInstance,
        OnInstance,
    },
};

/// Implemented by error codes in order to construct them from status codes.
///
/// A status code is returned by calling an ink! chain extension method.
/// It is the `u32` return value.
///
/// The purpose of an `ErrorCode` type that implements this trait is to provide
/// more context information about the status of an ink! chain extension method call.
pub trait FromStatusCode: Sized {
    /// Returns `Ok` if the status code for the called chain extension method is valid.
    ///
    /// Returning `Ok` will query the output buffer of the call if the chain extension
    /// method definition has a return value.
    ///
    /// # Note
    ///
    /// The convention is to use `0` as the only `raw` value that yields `Ok` whereas
    /// every other value represents one error code. By convention this mapping should
    /// never panic and therefore every `raw` value must map to either `Ok` or to a proper
    /// `ErrorCode` variant.
    fn from_status_code(status_code: u32) -> Result<(), Self>;
}

/// A concrete instance of a chain extension method.
///
/// This is a utility type used to drive the execution of a chain extension method call.
/// It has several different specializations of its `call` method for different ways to
/// manage error handling when calling a predefined chain extension method.
///
/// - `I` represents the input type of the chain extension method.
///   All tuple types that may act as input parameters for the chain extension method are valid.
///   Examples include `()`, `i32`, `(u8, [u8; 5], i32)`, etc.
/// - `O` represents the return (or output) type of the chain extension method.
///   Only `Result<T, E>` or `NoResult<O>` generic types are allowed for `O`.
///   The `Result<T, E>` type says that the chain extension method returns a `Result` type
///   whereas the `NoResult<O>` type says that the chain extension method returns a non-`Result` value
///   of type `O`.
/// - `ErrorCode` represents how the chain extension method handles the chain extension's error code.
///   Only `HandleErrorCode<E>` and `IgnoreErrorCode` types are allowed that each say to either properly
///   handle or ignore the chain extension's error code respectively.
///
/// The type states for type parameter `O` and `ErrorCode` represent 4 different states:
///
/// 1. The chain extension method makes use of the chain extension's error code: `HandleErrorCode(E)`
///     - **A:** The chain extension method returns a `Result<T, E>` type.
///     - **B:** The chain extension method returns a type `T` that is not a `Result` type: `NoResult<T>`
/// 2. The chain extension ignores the chain extension's error code: `IgnoreErrorCode`
///     - **A:** The chain extension method returns a `Result<T, E>` type.
///     - **B:** The chain extension method returns a type `T` that is not a `Result` type: `NoResult<T>`
#[derive(Debug)]
pub struct ChainExtensionMethodInstance<I, O, ErrorCode> {
    func_id: u32,
    state: PhantomData<fn() -> (I, O, ErrorCode)>,
}

impl ChainExtensionMethodInstance<(), (), ()> {
    /// Creates a new chain extension method instance.
    #[inline(always)]
    pub fn build(func_id: u32) -> Self {
        Self {
            func_id,
            state: Default::default(),
        }
    }
}

impl<O, ErrorCode> ChainExtensionMethodInstance<(), O, ErrorCode> {
    /// Sets the input types of the chain extension method call to `I`.
    ///
    /// # Note
    ///
    /// `I` represents the input type of the chain extension method.
    /// All tuple types that may act as input parameters for the chain extension method are valid.
    /// Examples include `()`, `i32`, `(u8, [u8; 5], i32)`, etc.
    #[inline(always)]
    pub fn input<I>(self) -> ChainExtensionMethodInstance<I, O, ErrorCode>
    where
        I: scale::Encode,
    {
        ChainExtensionMethodInstance {
            func_id: self.func_id,
            state: Default::default(),
        }
    }
}

impl<I, ErrorCode> ChainExtensionMethodInstance<I, (), ErrorCode> {
    /// Sets the output type of the chain extension method call to `Result<T, E>`.
    ///
    /// # Note
    ///
    /// This indicates that the chain extension method return value might represent a failure.
    #[inline(always)]
    pub fn output_result<T, E>(
        self,
    ) -> ChainExtensionMethodInstance<I, Result<T, E>, ErrorCode>
    where
        Result<T, E>: scale::Decode + From<scale::Error>,
    {
        ChainExtensionMethodInstance {
            func_id: self.func_id,
            state: Default::default(),
        }
    }

    /// Sets the output type of the chain extension method call to `O`.
    ///
    /// # Note
    ///
    /// The set returned type `O` must not be of type `Result<T, E>`.
    /// When using the `#[ink::chain_extension]` procedural macro to define
    /// this chain extension method the above constraint is enforced at
    /// compile time.
    #[inline(always)]
    pub fn output<O>(
        self,
    ) -> ChainExtensionMethodInstance<I, state::NoResult<O>, ErrorCode>
    where
        O: scale::Decode,
    {
        ChainExtensionMethodInstance {
            func_id: self.func_id,
            state: Default::default(),
        }
    }
}

impl<I, O> ChainExtensionMethodInstance<I, O, ()> {
    /// Makes the chain extension method call assume that the returned status code is always success.
    ///
    /// # Note
    ///
    /// This will avoid handling of failure status codes returned by the chain extension method call.
    /// Use this only if you are sure that the chain extension method call will never return an error
    /// code that represents failure.
    ///
    /// The output of the chain extension method call is always decoded and returned in this case.
    #[inline(always)]
    pub fn ignore_error_code(
        self,
    ) -> ChainExtensionMethodInstance<I, O, state::IgnoreErrorCode> {
        ChainExtensionMethodInstance {
            func_id: self.func_id,
            state: Default::default(),
        }
    }

    /// Makes the chain exntesion method call handle the returned status code.
    ///
    /// # Note
    ///
    /// This will handle the returned status code and only loads and decodes the value
    /// returned in the output of the chain extension method call in case of success.
    #[inline(always)]
    pub fn handle_error_code<ErrorCode>(
        self,
    ) -> ChainExtensionMethodInstance<I, O, state::HandleErrorCode<ErrorCode>>
    where
        ErrorCode: FromStatusCode,
    {
        ChainExtensionMethodInstance {
            func_id: self.func_id,
            state: Default::default(),
        }
    }
}

/// Type states of the chain extension method instance.
pub mod state {
    use core::marker::PhantomData;

    /// Type state telling that the chain extension method ignores the chain extension's error code.
    #[derive(Debug)]
    pub enum IgnoreErrorCode {}

    /// Type state telling that the chain extension method uses the chain extension's error code.
    #[derive(Debug)]
    pub struct HandleErrorCode<T> {
        error_code: PhantomData<fn() -> T>,
    }

    /// Type state telling that the chain extension method deliberately does not return a `Result` type.
    ///
    /// Additionally this is enforced by the `#[ink::chain_extension]` proc. macro when used.
    #[derive(Debug)]
    pub struct NoResult<T> {
        no_result: PhantomData<fn() -> T>,
    }
}

impl<I, T, E, ErrorCode>
    ChainExtensionMethodInstance<I, Result<T, E>, state::HandleErrorCode<ErrorCode>>
where
    I: scale::Encode,
    T: scale::Decode,
    E: scale::Decode + From<ErrorCode> + From<scale::Error>,
    ErrorCode: FromStatusCode,
{
    /// Calls the chain extension method for case 1A described [here].
    ///
    /// [here]: [`ChainExtensionMethodInstance`]
    ///
    /// # Errors
    ///
    /// - If the called chain extension method returned a non-successful error code.
    /// - If the returned `Result` of the called chain extension method cannot be decoded into `O`.
    /// - In case chain extension method specific constraints have not been met.
    ///     - These constraints are determined and defined by the author of the chain extension method.
    #[inline(always)]
    pub fn call(self, input: &I) -> Result<T, E> {
        <EnvInstance as OnInstance>::on_instance(|instance| {
            EnvBackend::call_chain_extension::<I, T, E, ErrorCode, _, _>(
                instance,
                self.func_id,
                input,
                ErrorCode::from_status_code,
                |output| scale::Decode::decode(&mut &output[..]).map_err(Into::into),
            )
        })
    }
}

impl<I, T, E> ChainExtensionMethodInstance<I, Result<T, E>, state::IgnoreErrorCode>
where
    I: scale::Encode,
    T: scale::Decode,
    E: scale::Decode + From<scale::Error>,
{
    /// Calls the chain extension method for case 2A described [here].
    ///
    /// [here]: [`ChainExtensionMethodInstance`]
    ///
    /// # Errors
    ///
    /// - If the returned return value of the called chain extension method cannot be decoded into `O`.
    /// - In case chain extension method specific constraints have not been met.
    ///     - These constraints are determined and defined by the author of the chain extension method.
    #[inline(always)]
    pub fn call(self, input: &I) -> Result<T, E> {
        <EnvInstance as OnInstance>::on_instance(|instance| {
            EnvBackend::call_chain_extension::<I, T, E, E, _, _>(
                instance,
                self.func_id,
                input,
                |_status_code| Ok(()),
                |output| scale::Decode::decode(&mut &output[..]).map_err(Into::into),
            )
        })
    }
}

impl<I, O, ErrorCode>
    ChainExtensionMethodInstance<I, state::NoResult<O>, state::HandleErrorCode<ErrorCode>>
where
    I: scale::Encode,
    O: scale::Decode,
    ErrorCode: FromStatusCode,
{
    /// Calls the chain extension method for case 1B described [here].
    ///
    /// [here]: [`ChainExtensionMethodInstance`]
    ///
    /// # Errors
    ///
    /// If the called chain extension method returned a non-successful error code.
    ///
    /// # Panics
    ///
    /// If the returned return value of the called chain extension method cannot be decoded into `O`.
    #[inline(always)]
    pub fn call(self, input: &I) -> Result<O, ErrorCode> {
        <EnvInstance as OnInstance>::on_instance(|instance| {
            EnvBackend::call_chain_extension::<I, O, ErrorCode, ErrorCode, _, _>(
                instance,
                self.func_id,
                input,
                ErrorCode::from_status_code,
                |output| {
                    let decoded = <O as scale::Decode>::decode(&mut &output[..])
                        .expect("encountered error while decoding chain extension method call return value");
                    Ok(decoded)
                },
            )
        })
    }
}

impl<I, O> ChainExtensionMethodInstance<I, state::NoResult<O>, state::IgnoreErrorCode>
where
    I: scale::Encode,
    O: scale::Decode,
{
    /// Calls the chain extension method for case 2B described [here].
    ///
    /// [here]: [`ChainExtensionMethodInstance`]
    ///
    /// # Panics
    ///
    /// If the returned return value of the called chain extension method cannot be decoded into `O`.
    #[inline(always)]
    pub fn call(self, input: &I) -> O {
        <EnvInstance as OnInstance>::on_instance(|instance| {
            EnvBackend::call_chain_extension::<I, O, (), (), _, _>(
                instance,
                self.func_id,
                input,
                |_status_code| Ok(()),
                |output| {
                    let decoded = <O as scale::Decode>::decode(&mut &output[..])
                        .expect("encountered error while decoding chain extension method call return value");
                    Ok(decoded)
                },
            ).expect("assume the chain extension method never fails")
        })
    }
}
