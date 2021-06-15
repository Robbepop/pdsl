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

mod chain_extension;
mod contract;
mod cross_calling;
mod dispatcher;
mod env_access;
mod error;
mod events;
mod traits;

pub use self::{
    chain_extension::{
        ChainExtensionInstance,
        IsResultType,
    },
    contract::{
        DispatchMode,
        DispatchUsingMode,
    },
    cross_calling::{
        ForwardCall,
        ForwardCallMut,
        NeverReturns,
        ToAccountId,
    },
    dispatcher::{
        deny_payment,
        execute_constructor,
        execute_message,
        execute_message_mut,
        AcceptsPayments,
        ConstructorDispatcher,
        EnablesDynamicStorageAllocator,
        Execute,
        MessageDispatcher,
    },
    env_access::{
        ContractEnv,
        Env,
        EnvAccess,
        StaticEnv,
    },
    error::{
        DispatchError,
        DispatchResult,
        DispatchRetCode,
    },
    events::{
        BaseEvent,
        EmitEvent,
    },
    traits::{
        TraitImplementer,
        Constructor,
        FnInput,
        FnOutput,
        FnSelector,
        FnState,
        ImpliesReturn,
        MessageMut,
        MessageRef,
        NoConcreteImplementer,
        TraitCallBuilder,
        TraitCallForwarder,
        TraitCallForwarderRegistry,
        TraitUniqueId,
        TraitCallForwarderFor,
        TraitModulePath,
        True,
    },
};
pub use ::static_assertions;
pub use ink_lang_macro::{
    chain_extension,
    contract,
    test,
    trait_definition,
};
