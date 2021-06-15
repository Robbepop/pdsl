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
        utils::{
            ReturnType,
            Set,
        },
        CallBuilder,
        ExecutionInput,
        Selector,
    },
    Environment,
};
use ink_storage::traits::SpreadLayout;

/// Trait used to indicate that an ink! trait definition has been checked
/// by the `#[ink::trait_definition]` procedural macro.
#[doc(hidden)]
pub unsafe trait TraitImplementer<const TRAIT_ID: u32> {}

/// Trait used by `#[ink::trait_definition]` to ensure that the associated
/// return type for each trait message is correct.
#[doc(hidden)]
pub trait ImpliesReturn<T> {}

impl<T> ImpliesReturn<T> for T {}
impl<T, E, Callee, GasCost, TransferredValue, Args> ImpliesReturn<T>
    for CallBuilder<
        E,
        Callee,
        GasCost,
        TransferredValue,
        Set<ExecutionInput<Args>>,
        Set<ReturnType<T>>,
    >
where
    E: Environment,
{
}

impl<E, Callee, GasCost, TransferredValue, Args> ImpliesReturn<()>
    for CallBuilder<
        E,
        Callee,
        GasCost,
        TransferredValue,
        Set<ExecutionInput<Args>>,
        Set<()>,
    >
where
    E: Environment,
{
}

/// Dispatchable functions that have inputs.
#[doc(hidden)]
pub trait FnInput {
    /// The tuple-type of all inputs.
    type Input: scale::Decode + 'static;
}

/// Dispatchable functions that have an output.
#[doc(hidden)]
pub trait FnOutput {
    /// The output type.
    type Output: scale::Encode + 'static;
}

/// The selector of dispatchable functions.
#[doc(hidden)]
pub trait FnSelector {
    /// The selector.
    const SELECTOR: Selector;
}

/// The storage state that the dispatchable function acts on.
#[doc(hidden)]
pub trait FnState {
    /// The storage state.
    type State: SpreadLayout + Sized;
}

/// A dispatchable contract constructor message.
#[doc(hidden)]
pub trait Constructor: FnInput + FnSelector + FnState {
    const CALLABLE: fn(<Self as FnInput>::Input) -> <Self as FnState>::State;
}

/// A `&self` dispatchable contract message.
#[doc(hidden)]
pub trait MessageRef: FnInput + FnOutput + FnSelector + FnState {
    const CALLABLE: fn(
        &<Self as FnState>::State,
        <Self as FnInput>::Input,
    ) -> <Self as FnOutput>::Output;
}

/// A `&mut self` dispatchable contract message.
#[doc(hidden)]
pub trait MessageMut: FnInput + FnOutput + FnSelector + FnState {
    const CALLABLE: fn(
        &mut <Self as FnState>::State,
        <Self as FnInput>::Input,
    ) -> <Self as FnOutput>::Output;
}

/// Indicates that some compile time expression is expected to be `true`.
#[doc(hidden)]
pub trait True {}

/// This type is known to ink! to implement all defined ink! trait definitions.
/// This property can be guaranteed by `#[ink::trait_definition]` proc. macro.
///
/// By the introduction of an new internal and hidden associated type called
/// `__ink_DynamicCallForwarder` for all ink! trait definitions it is possible
/// for ink! to map from any given ink! trait definition back to a concrete
/// Rust type.
/// Whenever the `ChainExtensionRegistry` implements an ink! trait definition
/// all calls are defaulted to produce linker errors (ideally compiler errors
/// if that was possible) and the only relevant implementation is the new
/// `__ink_DynamicCallForwarder` associated type that links to a concrete
/// type implementing `FromAccountId` and the ink! trait definition with
/// proper implementations.
///
/// Then ink! can map from the ink! trait definition `MyTrait` to this concrete
/// dynamic call forwarder type by:
/// ```no_compile
/// <::ink_lang::TraitDefinitionRegistry as MyTrait>::__ink_DynamicCallForwarder
/// ```
/// Normal implementations of ink! trait definitions default the new
/// `__ink_DynamicCallForwarder` associated type to `::ink_lang::NoDynamicCallForwarder`.
///
/// This is the technique used by ink! to resolve `&dyn MyTrait`, `&mut dyn MyTrait`
/// in message parameters or `dyn MyTrait` in ink! storage fields to concrete types
/// that ink! can serialize and deserialize as if it was an `AccountId` and call
/// ink! messages on it according to the ink! trait definition interface.
#[doc(hidden)]
pub struct TraitCallForwarderRegistry<E> {
    marker: PhantomData<fn() -> E>,
}

unsafe impl<E, const N: u32> TraitImplementer<N> for TraitCallForwarderRegistry<E> {}

/// The default type that ink! trait definition implementations use for the
/// `__ink_DynamicCallForwarder` associated type.
///
/// Read more about its use [here][TraitDefinitionRegistry].
#[doc(hidden)]
pub enum NoConcreteImplementer {}

/// The global call builder type for an ink! trait definition.
pub trait TraitCallBuilder {
    /// The call builder type.
    type Builder;

    /// Returns a shared reference to the global call builder type.
    ///
    /// This allows to call `&self` ink! trait messages.
    fn call(&self) -> &Self::Builder;

    /// Returns an exclusive reference to the global call builder type.
    ///
    /// This allows to call any ink! trait message.
    fn call_mut(&mut self) -> &mut Self::Builder;
}

/// Implemented by the global trait info provider.
///
/// This communicates the `u32` number that uniquely identifies
/// the ink! trait definition.
pub trait TraitUniqueId {
    /// The unique trait `u32` identifier.
    const ID: u32;
}

/// Implemented by the global trait info provider.
///
/// It is used to query the global trait call forwarder.
/// There is one global trait call forwarder that implements
/// the call forwarding (short- and long-form) for all calls
/// to this trait in `ink-as-dependency` configuration.
pub trait TraitCallForwarder {
    /// The call forwarder type.
    type Forwarder: TraitCallBuilder;
}

/// Captures the module path of the ink! trait definition.
///
/// This can be used to differentiate between two equally named
/// ink! trait definitions and also for metadata.
pub trait TraitModulePath {
    /// The module path of the ink! trait definition.
    ///
    /// This is equivalent to Rust's builtin `module_path!` macro
    /// invokation at the definition site of the ink! trait.
    const PATH: &'static str;

    /// The name of the ink! trait.
    ///
    /// This is just for convenience.
    const NAME: &'static str;
}

/// Implemented by call builders of smart contracts.
///
/// These might be implementing multiple different ink! traits.
/// The codegen makes them implement this trait once for every
/// ink! trait they have to implement.
///
/// While the trait is not necessary it encapsulates a lot of
/// utility and auxiliary code required for the actual ink! trait
/// implementations.
pub trait TraitCallForwarderFor<const ID: u32> {
    type Forwarder: TraitCallBuilder;

    /// Forwards the `&self` call.
    ///
    /// # Note
    ///
    /// This is used for the short-hand calling syntax.
    fn forward(&self) -> &Self::Forwarder;

    /// Forwards the `&mut self` call.
    ///
    /// # Note
    ///
    /// This is used for the short-hand calling syntax.
    fn forward_mut(&mut self) -> &mut Self::Forwarder;

    /// Builds up the `&self` call.
    ///
    /// # Note
    ///
    /// This is used for the long-hand calling syntax.
    fn build(&self) -> &<Self::Forwarder as TraitCallBuilder>::Builder;

    /// Builds up the `&mut self` call.
    ///
    /// # Note
    ///
    /// This is used for the long-hand calling syntax.
    fn build_mut(&mut self) -> &mut <Self::Forwarder as TraitCallBuilder>::Builder;
}

/// Stores information per trait message.
///
/// This information includes if the ink! trait message has been
/// annotated with `#[ink(payable)]`.
///
/// In the future this info trait might be extended to contain
/// more information about a single ink! trait message.
///
/// The information provided through this trait can be used on the
/// implementer side of an ink! trait to check and guard certain
/// properties on a Rust type system level. This is important since
/// ink! cannot be guaranteed to have both the ink! trait definition
/// and all of its implementers under its scope and radar.
///
/// # Note
///
/// - The `TRAIT_ID` is the `u32` identifier uniquely identifying the
///   ink! trait.
/// - The `MSG_ID` is the `u32` identifier derived from the selector
///   that uniquely identifies the message.
pub trait TraitMessageInfo<const TRAIT_ID: u32, const MSG_ID: u32> {
    /// Is `true` if the ink! trait message has been annotated with `#[ink(payable)]`.
    const PAYABLE: bool;

    /// The unique selector of the ink! trait message.
    const SELECTOR: [u8; 4];
}
