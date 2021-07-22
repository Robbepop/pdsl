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

extern crate proc_macro;

mod chain_extension;
mod contract;
mod ink_test;
mod trait_def;

use proc_macro::TokenStream;

/// Entry point for writing ink! smart contracts.
///
/// If you are a beginner trying to learn ink! we recommend you to check out
/// our extensive [ink! workshop](https://substrate.dev/substrate-contracts-workshop/#/).
///
/// **Note:** In all below examples we will be using `ink_lang` crate aliased as just `ink`.
///           You can do this yourself by adding the following line to your code:
///           `use ink_lang as ink;`
///
/// # Description
///
/// The macro does analysis on the provided smart contract code and generates
/// proper code.
///
/// ink! smart contracts can compile in several different modes.
/// There are two main compilation models using either
/// - on-chain mode: `no_std` and WebAssembly as target
/// - off-chain mode: `std`
///
/// We generally use the on-chain mode for actual smart contract deployment
/// whereas we use the off-chain mode for smart contract testing using the
/// off-chain environment provided by the `ink_env` crate.
///
/// # Usage
///
/// ## Header Arguments
///
/// The `#[ink::contract]` macro can be provided with some additional comma-separated
/// header arguments:
///
/// - `dynamic_storage_allocator: bool`
///
///     Tells the ink! code generator to allow usage of ink!'s built-in dynamic
///     storage allocator.
///     - `true`: Use the dynamic storage allocator provided by ink!.
///     - `false`: Do NOT use the dynamic storage allocator provided by ink!.
///
///     This feature is generally only needed for smart contracts that try to model
///     their data in a way that contains storage entities within other storage
///     entities.
///
///     Contract writers should try to write smart contracts that do not depend on the
///     dynamic storage allocator since enabling it comes at a cost of increased Wasm
///     file size. Although it will enable interesting use cases. Use it with care!
///
///     An example for this is the following type that could potentially be used
///     within a contract's storage struct definition:
///     ```
///     // A storage vector of storage vectors.
///     # use ink_storage as storage;
///     # type _unused =
///     storage::Vec<storage::Vec<i32>>
///     # ;
///     ```
///
///     **Usage Example:**
///     ```
///     # use ink_lang as ink;
///     #[ink::contract(dynamic_storage_allocator = true)]
///     mod my_contract {
///         # #[ink(storage)]
///         # pub struct MyStorage;
///         # impl MyStorage {
///         #     #[ink(constructor)]
///         #     pub fn construct() -> Self { MyStorage {} }
///         #     #[ink(message)]
///         #     pub fn message(&self) {}
///         # }
///         // ...
///     }
///     ```
///
///     **Default value:** `false`
///
/// - `compile_as_dependency: bool`
///
///     Tells the ink! code generator to **always** or **never**
///     compile the smart contract as if it was used as a dependency of another ink!
///     smart contract.
///
///     Normally this flag is only really useful for ink! developers who
///     want to inspect code generation of ink! smart contracts.
///     The author is not aware of any particular practical use case for users that
///     makes use of this flag but contract writers are encouraged to disprove this.
///
///     Note that it is recommended to make use of the built-in crate feature
///     `ink-as-dependency` to flag smart contract dependencies listed in a contract's
///     `Cargo.toml` as actual dependencies to ink!.
///
///     **Usage Example:**
///     ```
///     # use ink_lang as ink;
///     #[ink::contract(compile_as_dependency = true)]
///     mod my_contract {
///         # #[ink(storage)]
///         # pub struct MyStorage;
///         # impl MyStorage {
///         #     #[ink(constructor)]
///         #     pub fn construct() -> Self { MyStorage {} }
///         #     #[ink(message)]
///         #     pub fn message(&self) {}
///         # }
///         // ...
///     }
///     ```
///
///     **Default value:** Depends on the crate feature propagation of `Cargo.toml`.
///
/// - `env: impl Environment`
///
///     Tells the ink! code generator which environment to use for the ink! smart contract.
///     The environment must implement the `Environment` (defined in `ink_env`) trait and provides
///     all the necessary fundamental type definitions for `Balance`, `AccountId` etc.
///
///     When using a custom `Environment` implementation for a smart contract all types
///     that it exposes to the ink! smart contract and the mirrored types used in the runtime
///     must be aligned with respect to SCALE encoding and semantics.
///
///     **Usage Example:**
///
///     Given a custom `Environment` implementation:
///     ```
///     pub struct MyEnvironment;
///
///     impl ink_env::Environment for MyEnvironment {
///         const MAX_EVENT_TOPICS: usize = 3;
///         type AccountId = u64;
///         type Balance = u128;
///         type Hash = [u8; 32];
///         type Timestamp = u64;
///         type BlockNumber = u32;
///         type RentFraction = ::ink_env::Perbill;
///         type ChainExtension = ::ink_env::NoChainExtension;
///     }
///     ```
///     A user might implement their ink! smart contract using the above custom `Environment`
///     implementation as demonstrated below:
///     ```
///     # use ink_lang as ink;
///     #[ink::contract(env = MyEnvironment)]
///     mod my_contract {
///         # pub struct MyEnvironment;
///         #
///         # impl ink_env::Environment for MyEnvironment {
///         #     const MAX_EVENT_TOPICS: usize = 3;
///         #     type AccountId = u64;
///         #     type Balance = u128;
///         #     type Hash = [u8; 32];
///         #     type Timestamp = u64;
///         #     type BlockNumber = u32;
///         #     type ChainExtension = ::ink_env::NoChainExtension;
///         #     type RentFraction = ::ink_env::Perbill;
///         # }
///         #
///         # #[ink(storage)]
///         # pub struct MyStorage;
///         # impl MyStorage {
///         #     #[ink(constructor)]
///         #     pub fn construct() -> Self { MyStorage {} }
///         #     #[ink(message)]
///         #     pub fn message(&self) {}
///         # }
///         // ...
///     }
///     ```
///
///     **Default value:** `DefaultEnvironment` defined in `ink_env` crate.
///
/// ## Analysis
///
/// The `#[ink::contract]` macro fully analyses its input smart contract
/// against invalid arguments and structure.
///
/// Some example rules include but are not limited to:
///
/// - There must be exactly one `#[ink(storage)]` struct.
///
///     This struct defines the layout of the storage that the ink! smart contract operates on.
///     The user is able to use a variety of built-in facilities, combine them in various ways
///     or even provide their own implementations of storage data structures.
///
///     For more information visit the `ink_storage` crate documentation.
///
///     **Example:**
///
///     ```
///     # use ink_lang as ink;
///     #[ink::contract]
///     mod flipper {
///         #[ink(storage)]
///         pub struct Flipper {
///             value: bool,
///         }
///         # impl Flipper {
///         #     #[ink(constructor)]
///         #     pub fn construct() -> Self { Flipper { value: false } }
///         #     #[ink(message)]
///         #     pub fn message(&self) {}
///         # }
///     }
///     ```
///
/// - There must be at least one `#[ink(constructor)]` defined method.
///
///     Methods flagged with `#[ink(constructor)]` are special in that they are dispatchable
///     upon contract instantiation. A contract may define multiple such constructors which
///     allow users of the contract to instantiate a contract in multiple different ways.
///
///     **Example:**
///
///     Given the `Flipper` contract definition above we add an `#[ink(constructor)]`
///     as follows:
///
///     ```
///     # use ink_lang as ink;
///     # #[ink::contract]
///     # mod flipper {
///         # #[ink(storage)]
///         # pub struct Flipper {
///         #     value: bool,
///         # }
///     impl Flipper {
///         #[ink(constructor)]
///         pub fn new(initial_value: bool) -> Self {
///             Flipper { value: false }
///         }
///         # #[ink(message)]
///         # pub fn message(&self) {}
///     }
///     # }
///     ```
///
/// - There must be at least one `#[ink(message)]` defined method.
///
///     Methods flagged with `#[ink(message)]` are special in that they are dispatchable
///     upon contract invocation. The set of ink! messages defined for an ink! smart contract
///     define its API surface with which users are allowed to interact.
///
///     An ink! smart contract can have multiple such ink! messages defined.
///
///     **Note:**
///
///     - An ink! message with a `&self` receiver may only read state whereas an ink! message
///       with a `&mut self` receiver may mutate the contract's storage.
///
///     **Example:**
///
///     Given the `Flipper` contract definition above we add some `#[ink(message)]` definitions
///     as follows:
///
///     ```
///     # use ink_lang as ink;
///     # #[ink::contract]
///     # mod flipper {
///         # #[ink(storage)]
///         # pub struct Flipper {
///         #     value: bool,
///         # }
///     impl Flipper {
///         # #[ink(constructor)]
///         # pub fn new(initial_value: bool) -> Self {
///         #     Flipper { value: false }
///         # }
///         /// Flips the current value.
///         #[ink(message)]
///         pub fn flip(&mut self) {
///             self.value = !self.value;
///         }
///
///         /// Returns the current value.
///         #[ink(message)]
///         pub fn get(&self) -> bool {
///             self.value
///         }
///     }
///     # }
///     ```
///
///     **Payable Messages:**
///
///     An ink! message by default will reject calls that additional fund the smart contract.
///     Authors of ink! smart contracts can make an ink! message payable by adding the `payable`
///     flag to it. An example below:
///
///     Note that ink! constructors are always implicitly payable and thus cannot be flagged
///     as such.
///
///     ```
///     # use ink_lang as ink;
///     # #[ink::contract]
///     # mod flipper {
///         # #[ink(storage)]
///         # pub struct Flipper {
///         #     value: bool,
///         # }
///     impl Flipper {
///         # #[ink(constructor)]
///         # pub fn new(initial_value: bool) -> Self {
///         #     Flipper { value: false }
///         # }
///         /// Flips the current value.
///         #[ink(message)]
///         #[ink(payable)] // You can either specify payable out-of-line.
///         pub fn flip(&mut self) {
///             self.value = !self.value;
///         }
///
///         /// Returns the current value.
///         #[ink(message, payable)] // ... or specify payable inline.
///         pub fn get(&self) -> bool {
///             self.value
///         }
///     }
///     # }
///     ```
///
///     **Controlling the messages selector:**
///
///     Every ink! message and ink! constructor has a unique selector with which the
///     message or constructor can be uniquely identified within the ink! smart contract.
///     These selectors are mainly used to drive the contract's dispatch upon calling it.
///
///     An ink! smart contract author can control the selector of an ink! message or ink!
///     constructor using the `selector` flag. An example is shown below:
///
///     ```
///     # use ink_lang as ink;
///     # #[ink::contract]
///     # mod flipper {
///         # #[ink(storage)]
///         # pub struct Flipper {
///         #     value: bool,
///         # }
///     impl Flipper {
///         #[ink(constructor)]
///         #[ink(selector = "0xDEADBEEF")] // Works on constructors as well.
///         pub fn new(initial_value: bool) -> Self {
///             Flipper { value: false }
///         }
///
///         # /// Flips the current value.
///         # #[ink(message)]
///         # #[ink(selector = "0xCAFEBABE")] // You can either specify selector out-of-line.
///         # pub fn flip(&mut self) {
///         #     self.value = !self.value;
///         # }
///         #
///         /// Returns the current value.
///         #[ink(message, selector = "0xFEEDBEEF")] // ... or specify selector inline.
///         pub fn get(&self) -> bool {
///             self.value
///         }
///     }
///     # }
///     ```
///
///     **Controlling the naming of messages in ABI:**
///
///     Every ink! message, ink! constructor and ink! implementation section has a name which
///     is used during generation of metadata(ABI).
///
///     An ink! smart contract author can control the naming in ABI using the `metadata_name` flag. An example is shown below:
///
///     ```
///     # use ink_lang as ink;
///     # #[ink::contract]
///     # mod flipper {
///         # #[ink(storage)]
///         # pub struct Flipper {
///         #     value: bool,
///         # }
///
///     #[ink(metadata_name = "MyFlipper")]
///     impl Flipper {
///         #[ink(constructor)]
///         #[ink(metadata_name = "new_flipper")] // Works on constructors as well.
///         pub fn new(initial_value: bool) -> Self {
///             Flipper { value: false }
///         }
///
///         # /// Flips the current value.
///         # #[ink(message)]
///         # #[ink(metadata_name = "my_flip")] // You can either specify name out-of-line.
///         # pub fn flip(&mut self) {
///         #     self.value = !self.value;
///         # }
///         #
///         /// Returns the current value.
///         #[ink(message, metadata_name = "get_value")] // ... or specify selector inline.
///         pub fn get(&self) -> bool {
///             self.value
///         }
///     }
///     # }
///     ```
///
/// ## Interacting with the Contract Executor
///
/// The `ink_env` crate provides facilities to interact with the contract executor that
/// connects ink! smart contracts with the outer world.
///
/// For example it is possible to query the current call's caller via:
/// ```
/// # ink_env::test::run_test::<ink_env::DefaultEnvironment, _>(|_| {
/// let caller = ink_env::caller::<ink_env::DefaultEnvironment>();
/// # let _caller = caller;
/// # Ok(())
/// # }).unwrap();
/// ```
///
/// However, ink! provides a much simpler way to interact with the contract executor
/// via its environment accessor. An example below:
///
/// ```
/// # use ink_lang as ink;
/// #
/// #[ink::contract]
/// mod greeter {
///     use ink_env::debug_println;
///
///     #[ink(storage)]
///     pub struct Greeter;
///
///     impl Greeter {
///         #[ink(constructor)]
///         pub fn new() -> Self {
///             let caller = Self::env().caller();
///             debug_println!("thanks for instantiation {:?}", caller);
///             Greeter {}
///         }
///
///         #[ink(message, payable)]
///         pub fn fund(&self) {
///             let caller = self.env().caller();
///             let value = self.env().transferred_balance();
///             debug_println!("thanks for the funding of {:?} from {:?}", value, caller);
///         }
///     }
/// }
/// ```
///
/// ## Events
///
/// An ink! smart contract may define events that it can emit during contract execution.
/// Emitting events can be used by third party tools to query information about a contract's
/// execution and state.
///
/// The following example ink! contract shows how an event `Transferred` is defined and
/// emitted in the `#[ink(constructor)]`.
///
/// ```
/// # use ink_lang as ink;
/// #
/// #[ink::contract]
/// mod erc20 {
///     /// Defines an event that is emitted every time value is transferred.
///     #[ink(event)]
///     pub struct Transferred {
///         from: Option<AccountId>,
///         to: Option<AccountId>,
///         value: Balance,
///     }
///
///     #[ink(storage)]
///     pub struct Erc20 {
///         total_supply: Balance,
///         // more fields ...
///     }
///
///     impl Erc20 {
///         #[ink(constructor)]
///         pub fn new(initial_supply: Balance) -> Self {
///             let caller = Self::env().caller();
///             Self::env().emit_event(Transferred {
///                 from: None,
///                 to: Some(caller),
///                 value: initial_supply,
///             });
///             Self { total_supply: initial_supply }
///         }
///
///         #[ink(message)]
///         pub fn total_supply(&self) -> Balance {
///             self.total_supply
///         }
///     }
/// }
/// ```
///
/// ## Example: Flipper
///
/// The below code shows the complete implementation of the so-called Flipper
/// ink! smart contract.
/// For us it acts as the "Hello, World!" of the ink! smart contracts because
/// it is minimal while still providing some more or less useful functionality.
///
/// It controls a single `bool` value that can be either `false` or `true`
/// and allows the user to flip this value using the `Flipper::flip` message
/// or retrieve the current value using `Flipper::get`.
///
/// ```
/// use ink_lang as ink;
///
/// #[ink::contract]
/// pub mod flipper {
///     #[ink(storage)]
///     pub struct Flipper {
///         value: bool,
///     }
///
///     impl Flipper {
///         /// Creates a new flipper smart contract initialized with the given value.
///         #[ink(constructor)]
///         pub fn new(init_value: bool) -> Self {
///             Self { value: init_value }
///         }
///
///         /// Flips the current value of the Flipper's boolean.
///         #[ink(message)]
///         pub fn flip(&mut self) {
///             self.value = !self.value;
///         }
///
///         /// Returns the current value of the Flipper's boolean.
///         #[ink(message)]
///         pub fn get(&self) -> bool {
///             self.value
///         }
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn contract(attr: TokenStream, item: TokenStream) -> TokenStream {
    contract::generate(attr.into(), item.into()).into()
}

/// Marks trait definitions to ink! as special ink! trait definitions.
///
/// There are some restrictions that apply to ink! trait definitions that
/// this macro checks. Also ink! trait definitions are required to have specialized
/// structure so that the main [`#[ink::contract]`](`macro@crate::contract`) macro can
/// properly generate code for its implementations.
///
/// This definition allows only to use `#[ink(message)]` and `#[ink(constructor)]` ink! attributes.
///
/// # Example: Definition
///
/// ```
/// use ink_lang as ink;
/// # type Balance = <ink_env::DefaultEnvironment as ink_env::Environment>::Balance;
///
/// #[ink::trait_definition]
/// pub trait Erc20 {
///     /// Constructs a new ERC-20 compliant smart contract using the initial supply.
///     #[ink(constructor)]
///     fn new(initial_supply: Balance) -> Self;
///
///     /// Returns the total supply of the ERC-20 smart contract.
///     #[ink(message)]
///     fn total_supply(&self) -> Balance;
///
///     // etc.
/// }
/// ```
///
/// # Example: Implementation
///
/// Given the above trait definition you can implement it as shown below:
///
/// ```
/// # use ink_lang as ink;
/// #
/// #[ink::contract]
/// mod base_erc20 {
/// #    // We somehow cannot put the trait in the doc-test crate root due to bugs.
/// #    #[ink_lang::trait_definition]
/// #    pub trait Erc20 {
/// #        /// Constructors a new ERC-20 compliant smart contract using the initial supply.
/// #        #[ink(constructor)]
/// #        fn new(initial_supply: Balance) -> Self;
/// #
/// #        /// Returns the total supply of the ERC-20 smart contract.
/// #        #[ink(message)]
/// #        fn total_supply(&self) -> Balance;
/// #    }
/// #
///     #[ink(storage)]
///     pub struct BaseErc20 {
///         total_supply: Balance,
///         // etc ..
///     }
///
///     impl Erc20 for BaseErc20 {
///         #[ink(constructor)]
///         fn new(initial_supply: Balance) -> Self {
///             Self { total_supply: initial_supply }
///         }
///
///         /// Returns the total supply of the ERC-20 smart contract.
///         #[ink(message)]
///         fn total_supply(&self) -> Balance {
///             self.total_supply
///         }
///
///         // etc ..
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn trait_definition(attr: TokenStream, item: TokenStream) -> TokenStream {
    trait_def::analyze(attr.into(), item.into()).into()
}

/// Defines a unit test that makes use of ink!'s off-chain testing capabilities.
///
/// If your unit test does not require the existence of an off-chain environment
/// it is fine to not use this macro since it bears some overhead with the test.
///
/// Note that this macro is not required to run unit tests that require ink!'s
/// off-chain testing capabilities but merely improves code readability.
///
/// ## How do you find out if your test requires the off-chain environment?
///
/// Normally if the test recursively uses or invokes some contract methods that
/// call a method defined in `self.env()` or `Self::env()`.
///
/// An examples is the following:
///
/// ```no_compile
/// let caller: AccountId = self.env().caller();
/// ```
///
/// # Example
///
/// ```
/// use ink_lang as ink;
///
/// #[cfg(test)]
/// mod tests {
///     // Conventional unit test that works with assertions.
///     #[ink::test]
///     fn test1() {
///         // test code comes here as usual
///     }
///
///     // Conventional unit test that returns some Result.
///     // The test code can make use of operator-`?`.
///     #[ink::test]
///     fn test2() -> Result<(), ink_env::Error> {
///         // test code that returns a Rust Result type
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    ink_test::generate(attr.into(), item.into()).into()
}

/// Defines the interface for a chain extension.
///
/// # Structure
///
/// The interface consists of an error code that indicates lightweight errors
/// as well as the definition of some chain extension methods.
///
/// The overall structure follows that of a simple Rust trait definition.
/// The error code is defined as an associated type definition of the trait definition.
/// The methods are defined as associated trait methods without implementation.
///
/// Chain extension methods must not have a `self` receiver such as `&self` or `&mut self`
/// and must have inputs and output that implement the SCALE encoding and decoding.
/// Their return value follows specific rules that can be altered using the `handle_status`
/// and `returns_result` attributes which are described in more detail below.
///
/// # Usage
///
/// Usually the chain extension definition using this procedural macro is provided
/// by the author of the chain extension in a separate crate.
/// ink! smart contracts using this chain extension simply depend on this crate
/// and use its associated environment definition in order to make use of
/// the methods provided by the chain extension.
///
/// # Attributes
///
/// There are three different attributes with which the chain extension methods
/// can be flagged:
///
/// | Attribute | Required | Default Value | Description |
/// |:----------|:--------:|:--------------|:-----------:|
/// | `ink(extension = N: u32)` | Yes | - | Determines the unique function ID of the chain extension method. |
/// | `ink(handle_status = flag: bool)` | Optional | `true` | Assumes that the returned status code of the chain extension method always indicates success and therefore always loads and decodes the output buffer of the call. |
/// | `ink(returns_result = flag: bool)` | Optional | `true` | By default chain extension methods are assumed to return a `Result<T, E>` in the output buffer. Using `returns_result = false` this check is disabled and the chain extension method may return any other type. |
///
/// As with all ink! attributes multiple of them can either appear in a contiguous list:
/// ```
/// # type Access = i32;
/// # use ink_lang as ink;
/// # #[ink::chain_extension]
/// # pub trait MyChainExtension {
/// #     type ErrorCode = i32;
/// #[ink(extension = 5, handle_status = false, returns_result = false)]
/// fn key_access_for_account(key: &[u8], account: &[u8]) -> Access;
/// # }
/// ```
/// ... or as multiple stand alone ink! attributes applied to the same item:
/// ```
/// # type Access = i32;
/// # use ink_lang as ink;
/// # #[ink::chain_extension]
/// # pub trait MyChainExtension {
/// #     type ErrorCode = i32;
/// #[ink(extension = 5)]
/// #[ink(handle_status = false)]
/// #[ink(returns_result = false)]
/// fn key_access_for_account(key: &[u8], account: &[u8]) -> Access;
/// # }
/// ```
///
/// ## Details: `handle_status`
///
/// Default value: `true`
///
/// By default all chain extension methods return a `Result<T, E>` where `E: From<Self::ErrorCode>`.
/// The `Self::ErrorCode` represents the error code of the chain extension.
/// This means that a smart contract calling such a chain extension method first queries the returned
/// status code of the chain extension method and only loads and decodes the output if the returned
/// status code indicates a successful call.
/// This design was chosen as it is more efficient when no output besides the error
/// code is required for a chain extension call. When designing a chain extension try to utilize the
/// error code to return errors and only use the output buffer for information that does not fit in
/// a single `u32` value.
///
/// A chain extension method that is flagged with `handle_status = false` assumes that the returned error code
/// will always indicate success. Therefore it will always load and decode the output buffer and loses
/// the `E: From<Self::ErrorCode` constraint for the call.
///
/// ## Details: `returns_result`
///
/// Default value: `true`
///
/// By default chain extension methods are assumed to return a value of type `Result<T, E>` through the
/// output buffer. Using `returns_result = false` this check is disabled and the chain extension method may return
/// any other type.
///
/// Note that if a chain extension method is attributed with `returns_result = false`
/// and with `handle_status = true` it will still return a value of type `Result<T, Self::ErrorCode>`.
///
/// ## Usage: `handle_status` and `returns_result`
///
/// Use both `handle_status = false` and `returns_result = false` for the same chain extension method
/// if a call to it may never fail and never returns a `Result` type.
///
/// # Combinations
///
/// Due to the possibility to flag a chain extension method with `handle_status` and `returns_result`
/// there are 4 different cases with slightly varying semantics:
///
/// | `handle_status` | `returns_result` | Effects |
/// |:---------------:|:----------------:|:--------|
/// | `true`  | `true`  | The chain extension method is required to return a value of type `Result<T, E>` where `E: From<Self::ErrorCode>`. A call will always check if the returned status code indicates success and only then will load and decode the value in the output buffer. |
/// | `true`  | `false` | The chain extension method may return any non-`Result` type. A call will always check if the returned status code indicates success and only then will load and decode the value in the output buffer. The actual return type of the chain extension method is still `Result<T, Self::ErrorCode>` when the chain extension method was defined to return a value of type `T`. |
/// | `false` | `true`  | The chain extension method is required to return a value of type `Result<T, E>`. A call will always assume that the returned status code indicates success and therefore always load and decode the output buffer directly. |
/// | `false` | `false` | The chain extension method may return any non-`Result` type. A call will always assume that the returned status code indicates success and therefore always load and decode the output buffer directly. |
///
/// # Error Code
///
/// Every chain extension defines exactly one `ErrorCode` using the following syntax:
///
/// ```
/// use ink_lang as ink;
///
/// #[ink::chain_extension]
/// pub trait MyChainExtension {
///     type ErrorCode = MyErrorCode;
///
///     // more definitions ...
/// }
/// ```
///
/// The defined `ErrorCode` must implement `FromStatusCode` which should be implemented as a
/// more or less trivial conversion from the `u32` status code to a `Result<(), Self::ErrorCode>`.
/// The `Ok(())` value indicates that the call to the chain extension method was successful.
///
/// By convention an error code of `0` represents success.
/// However, chain extension authors may use whatever suits their needs.
///
/// # Example: Definition
///
/// In the below example a chain extension is defined that allows its users to read and write
/// from and to the runtime storage using access privileges:
///
/// ```
/// use ink_lang as ink;
///
/// /// Custom chain extension to read to and write from the runtime.
/// #[ink::chain_extension]
/// pub trait RuntimeReadWrite {
///     type ErrorCode = ReadWriteErrorCode;
///
///     /// Reads from runtime storage.
///     ///
///     /// # Note
///     ///
///     /// Actually returns a value of type `Result<Vec<u8>, Self::ErrorCode>`.
///     #[ink(extension = 1, returns_result = false)]
///     fn read(key: &[u8]) -> Vec<u8>;
///
///     /// Reads from runtime storage.
///     ///
///     /// Returns the number of bytes read and up to 32 bytes of the
///     /// read value. Unused bytes in the output are set to 0.
///     ///
///     /// # Errors
///     ///
///     /// If the runtime storage cell stores a value that requires more than
///     /// 32 bytes.
///     ///
///     /// # Note
///     ///
///     /// This requires `ReadWriteError` to implement `From<ReadWriteErrorCode>`
///     /// and may potentially return any `Self::ErrorCode` through its return value.
///     #[ink(extension = 2)]
///     fn read_small(key: &[u8]) -> Result<(u32, [u8; 32]), ReadWriteError>;
///
///     /// Writes into runtime storage.
///     ///
///     /// # Note
///     ///
///     /// Actually returns a value of type `Result<(), Self::ErrorCode>`.
///     #[ink(extension = 3, returns_result = false)]
///     fn write(key: &[u8], value: &[u8]);
///
///     /// Returns the access allowed for the key for the caller.
///     ///
///     /// # Note
///     ///
///     /// Assumes to never fail the call and therefore always returns `Option<Access>`.
///     #[ink(extension = 4, returns_result = false, handle_status = false)]
///     fn access(key: &[u8]) -> Option<Access>;
///
///     /// Unlocks previously aquired permission to access key.
///     ///
///     /// # Errors
///     ///
///     /// If the permission was not granted.
///     ///
///     /// # Note
///     ///
///     /// Assumes the call to never fail and therefore does _NOT_ require `UnlockAccessError`
///     /// to implement `From<Self::ErrorCode>` as in the `read_small` method above.
///     #[ink(extension = 5, handle_status = false)]
///     fn unlock_access(key: &[u8], access: Access) -> Result<(), UnlockAccessError>;
/// }
/// # #[derive(scale::Encode, scale::Decode, scale_info::TypeInfo)]
/// # pub enum ReadWriteErrorCode {
/// #     InvalidKey,
/// #     CannotWriteToKey,
/// #     CannotReadFromKey,
/// # }
/// # #[derive(scale::Encode, scale::Decode, scale_info::TypeInfo)]
/// # pub enum ReadWriteError {
/// #     ErrorCode(ReadWriteErrorCode),
/// #     BufferTooSmall { required_bytes: u32 },
/// # }
/// # impl From<ReadWriteErrorCode> for ReadWriteError {
/// #     fn from(error_code: ReadWriteErrorCode) -> Self {
/// #         Self::ErrorCode(error_code)
/// #     }
/// # }
/// # impl From<scale::Error> for ReadWriteError {
/// #     fn from(_: scale::Error) -> Self {
/// #         panic!("encountered unexpected invalid SCALE encoding")
/// #     }
/// # }
/// # #[derive(scale::Encode, scale::Decode, scale_info::TypeInfo)]
/// # pub struct UnlockAccessError {
/// #     reason: String,
/// # }
/// # impl From<scale::Error> for UnlockAccessError {
/// #     fn from(_: scale::Error) -> Self {
/// #         panic!("encountered unexpected invalid SCALE encoding")
/// #     }
/// # }
/// # #[derive(scale::Encode, scale::Decode, scale_info::TypeInfo)]
/// # pub enum Access {
/// #     ReadWrite,
/// #     ReadOnly,
/// #     WriteOnly,
/// # }
/// # impl ink_env::chain_extension::FromStatusCode for ReadWriteErrorCode {
/// #     fn from_status_code(status_code: u32) -> Result<(), Self> {
/// #         match status_code {
/// #             0 => Ok(()),
/// #             1 => Err(Self::InvalidKey),
/// #             2 => Err(Self::CannotWriteToKey),
/// #             3 => Err(Self::CannotReadFromKey),
/// #             _ => panic!("encountered unknown status code"),
/// #         }
/// #     }
/// # }
/// ```
///
/// All the error types and other utility types used in the chain extension definition
/// above are often required to implement various traits such as SCALE's `Encode` and `Decode`
/// as well as `scale-info`'s `TypeInfo` trait.
///
/// A full example of the above chain extension definition can be seen
/// [here](https://github.com/paritytech/ink/blob/017f71d60799b764425334f86b732cc7b7065fe6/crates/lang/macro/tests/ui/chain_extension/simple.rs).
///
/// # Example: Environment
///
/// In order to allow ink! smart contracts to use the above defined chain extension it needs
/// to be integrated into an `Environment` definition as shown below:
///
/// ```
/// # type RuntimeReadWrite = i32;
/// #
/// use ink_env::{Environment, DefaultEnvironment};
///
/// pub enum CustomEnvironment {}
///
/// impl Environment for CustomEnvironment {
///     const MAX_EVENT_TOPICS: usize =
///         <DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;
///
///     type AccountId = <DefaultEnvironment as Environment>::AccountId;
///     type Balance = <DefaultEnvironment as Environment>::Balance;
///     type Hash = <DefaultEnvironment as Environment>::Hash;
///     type BlockNumber = <DefaultEnvironment as Environment>::BlockNumber;
///     type Timestamp = <DefaultEnvironment as Environment>::Timestamp;
///     type RentFraction = <DefaultEnvironment as Environment>::RentFraction;
///
///     type ChainExtension = RuntimeReadWrite;
/// }
/// ```
///
/// Above we defined the `CustomEnvironment` which defaults to ink!'s `DefaultEnvironment`
/// for all constants and types but the `ChainExtension` type which is assigned to our newly
/// defined chain extension.
///
/// # Example: Usage
///
/// An ink! smart contract can use the above defined chain extension through the `Environment`
/// definition defined in the last example section using the `env` macro parameter as
/// shown below.
///
/// Note that chain extension methods are accessible through `Self::extension()` or
/// `self.extension()`. For example as in `Self::extension().read(..)` or `self.extension().read(..)`.
///
/// ```
/// # use ink_lang as ink;
/// #
/// #[ink::contract(env = CustomEnvironment)]
/// mod read_writer {
///     # use ink_lang as ink;
///     #
///     #[ink(storage)]
///     pub struct ReadWriter {}
///
///     impl ReadWriter {
///         #[ink(constructor)]
///         pub fn new() -> Self { Self {} }
///
///         #[ink(message)]
///         pub fn read(&self, key: Vec<u8>) -> Result<Vec<u8>, ReadWriteErrorCode> {
///             self.env()
///                 .extension()
///                 .read(&key)
///         }
///
///         #[ink(message)]
///         pub fn read_small(&self, key: Vec<u8>) -> Result<(u32, [u8; 32]), ReadWriteError> {
///             self.env()
///                 .extension()
///                 .read_small(&key)
///         }
///
///         #[ink(message)]
///         pub fn write(
///             &self,
///             key: Vec<u8>,
///             value: Vec<u8>,
///         ) -> Result<(), ReadWriteErrorCode> {
///             self.env()
///                 .extension()
///                 .write(&key, &value)
///         }
///
///         #[ink(message)]
///         pub fn access(&self, key: Vec<u8>) -> Option<Access> {
///             self.env()
///                 .extension()
///                 .access(&key)
///         }
///
///         #[ink(message)]
///         pub fn unlock_access(&self, key: Vec<u8>, access: Access) -> Result<(), UnlockAccessError> {
///             self.env()
///                 .extension()
///                 .unlock_access(&key, access)
///         }
///     }
/// # /// Custom chain extension to read to and write from the runtime.
/// # #[ink::chain_extension]
/// # pub trait RuntimeReadWrite {
/// #     type ErrorCode = ReadWriteErrorCode;
/// #     #[ink(extension = 1, returns_result = false)]
/// #     fn read(key: &[u8]) -> Vec<u8>;
/// #     #[ink(extension = 2)]
/// #     fn read_small(key: &[u8]) -> Result<(u32, [u8; 32]), ReadWriteError>;
/// #     #[ink(extension = 3, returns_result = false)]
/// #     fn write(key: &[u8], value: &[u8]);
/// #     #[ink(extension = 4, returns_result = false, handle_status = false)]
/// #     fn access(key: &[u8]) -> Option<Access>;
/// #     #[ink(extension = 5, handle_status = false)]
/// #     fn unlock_access(key: &[u8], access: Access) -> Result<(), UnlockAccessError>;
/// # }
/// # #[derive(scale::Encode, scale::Decode, scale_info::TypeInfo)]
/// # pub enum ReadWriteErrorCode {
/// #     InvalidKey,
/// #     CannotWriteToKey,
/// #     CannotReadFromKey,
/// # }
/// # #[derive(scale::Encode, scale::Decode, scale_info::TypeInfo)]
/// # pub enum ReadWriteError {
/// #     ErrorCode(ReadWriteErrorCode),
/// #     BufferTooSmall { required_bytes: u32 },
/// # }
/// # impl From<ReadWriteErrorCode> for ReadWriteError {
/// #     fn from(error_code: ReadWriteErrorCode) -> Self {
/// #         Self::ErrorCode(error_code)
/// #     }
/// # }
/// # impl From<scale::Error> for ReadWriteError {
/// #     fn from(_: scale::Error) -> Self {
/// #         panic!("encountered unexpected invalid SCALE encoding")
/// #     }
/// # }
/// # #[derive(scale::Encode, scale::Decode, scale_info::TypeInfo)]
/// # pub struct UnlockAccessError {
/// #     reason: String,
/// # }
/// # impl From<scale::Error> for UnlockAccessError {
/// #     fn from(_: scale::Error) -> Self {
/// #         panic!("encountered unexpected invalid SCALE encoding")
/// #     }
/// # }
/// # #[derive(scale::Encode, scale::Decode, scale_info::TypeInfo)]
/// # pub enum Access {
/// #     ReadWrite,
/// #     ReadOnly,
/// #     WriteOnly,
/// # }
/// # impl ink_env::chain_extension::FromStatusCode for ReadWriteErrorCode {
/// #     fn from_status_code(status_code: u32) -> Result<(), Self> {
/// #         match status_code {
/// #             0 => Ok(()),
/// #             1 => Err(Self::InvalidKey),
/// #             2 => Err(Self::CannotWriteToKey),
/// #             3 => Err(Self::CannotReadFromKey),
/// #             _ => panic!("encountered unknown status code"),
/// #         }
/// #     }
/// # }
/// # pub enum CustomEnvironment {}
/// # impl ink_env::Environment for CustomEnvironment {
/// #     const MAX_EVENT_TOPICS: usize =
/// #         <ink_env::DefaultEnvironment as ink_env::Environment>::MAX_EVENT_TOPICS;
/// #
/// #     type AccountId = <ink_env::DefaultEnvironment as ink_env::Environment>::AccountId;
/// #     type Balance = <ink_env::DefaultEnvironment as ink_env::Environment>::Balance;
/// #     type Hash = <ink_env::DefaultEnvironment as ink_env::Environment>::Hash;
/// #     type BlockNumber = <ink_env::DefaultEnvironment as ink_env::Environment>::BlockNumber;
/// #     type Timestamp = <ink_env::DefaultEnvironment as ink_env::Environment>::Timestamp;
/// #     type RentFraction = <ink_env::DefaultEnvironment as ink_env::Environment>::RentFraction;
/// #
/// #     type ChainExtension = RuntimeReadWrite;
/// # }
/// }
/// ```
///
/// # Technical Limitations
///
/// - Due to technical limitations it is not possible to refer to the `ErrorCode` associated type
///   using `Self::ErrorCode` anywhere within the chain extension and its defined methods.
///   Instead chain extension authors should directly use the error code type when required.
///   This limitation might be lifted in future versions of ink!.
/// - It is not possible to declare other chain extension traits as super traits or super
///   chain extensions of another.
#[proc_macro_attribute]
pub fn chain_extension(attr: TokenStream, item: TokenStream) -> TokenStream {
    chain_extension::generate(attr.into(), item.into()).into()
}

#[cfg(test)]
pub use contract::generate_or_err;
