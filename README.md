<div align="center">
    <img src="./.images/ink-logo-glow.svg" alt="ink!" height="136" />
<h1 align="center">
    Parity's ink! for writing smart contracts
</h1>

[![linux][a1]][a2] [![codecov][c1]][c2] [![coveralls][d1]][d2] [![loc][e1]][e2] [![matrix][k1]][k2] [![discord][l1]][l2]

[a1]: https://gitlab.parity.io/parity/ink/badges/master/pipeline.svg
[a2]: https://gitlab.parity.io/parity/ink/pipelines?ref=master
[c1]: https://codecov.io/gh/paritytech/ink/branch/master/graph/badge.svg
[c2]: https://codecov.io/gh/paritytech/ink/branch/master
[d1]: https://coveralls.io/repos/github/paritytech/ink/badge.svg?branch=master
[d2]: https://coveralls.io/github/paritytech/ink?branch=master
[e1]: https://tokei.rs/b1/github/paritytech/ink?category=code
[e2]: https://github.com/Aaronepower/tokei#badges
[f1]: https://img.shields.io/badge/click-blue.svg
[f2]: https://paritytech.github.io/ink/ink_storage
[g1]: https://img.shields.io/badge/click-blue.svg
[g2]: https://paritytech.github.io/ink/ink_env
[i1]: https://img.shields.io/badge/click-blue.svg
[i2]: https://paritytech.github.io/ink/ink_prelude
[j1]: https://img.shields.io/badge/click-blue.svg
[j2]: https://paritytech.github.io/ink/ink_lang
[k1]: https://img.shields.io/badge/matrix-chat-brightgreen.svg?style=flat
[k2]: https://riot.im/app/#/room/#ink:matrix.parity.io
[l1]: https://img.shields.io/discord/722223075629727774?style=flat-square&label=discord
[l2]: https://discord.com/invite/wGUDt2p

> <img src="./.images/ink-squid.svg" alt="squink, the ink! mascot" style="vertical-align: middle" align="left" height="60" />ink! is an [eDSL](https://wiki.haskell.org/Embedded_domain_specific_language) to write smart contracts in Rust for blockchains built on the [Substrate](https://github.com/paritytech/substrate) framework. ink! contracts are compiled to WebAssembly.

<br/>

[Guided Tutorial for Beginners](https://substrate.dev/substrate-contracts-workshop/#/0/building-your-contract)&nbsp;&nbsp;•&nbsp;&nbsp;
[ink! Documentation Portal](https://paritytech.github.io/ink-docs)

<br/>
</div>

More relevant links:
* Talk to us on [Element][k2] or in [Discord][l2]
  on the [`ink_smart-contracts`](https://discord.com/channels/722223075629727774/765280480609828864) channel
* [`cargo-contract`](https://github.com/paritytech/cargo-contract) ‒ CLI tool for ink! contracts
* [Canvas UI](https://paritytech.github.io/canvas-ui/#/upload) ‒ Frontend for contract deployment and interaction
* [Canvas Node](https://github.com/paritytech/canvas-node) ‒ Simple Substrate blockchain which includes smart contract functionality


## Table of Contents

* [Play with It](#play-with-it)
* [Usage](#usage)
* [Hello, World! ‒ The Flipper](#hello-world--the-flipper)
* [Examples](#examples)
* [How it Works](#how-it-works)
* [ink! Macros & Attributes Overview](#ink-macros--attributes-overview)
  * [Entry Point](#entry-point)
  * [Trait Definitions](#trait-definitions)
  * [Off-chain Testing](#off-chain-testing)
* [Developer Documentation](#developer-documentation)
* [Contributing](#contributing)
* [License](#license)


## Play with It

If you want to have a local setup you can use our [canvas-node](https://github.com/paritytech/canvas-node) for a quickstart.
It's a simple Substrate blockchain which includes the Substrate module for smart contract functionality ‒ the `contracts` pallet (see [How it Works](#how-it-works) for more).

The [Canvas UI](https://paritytech.github.io/canvas-ui/#/upload) can be used to deploy your contract to a chain and interact with it.

We also have [a demonstration testnet](https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fcanvas-rpc.parity.io) running.
You can request some tokens to play with from our [Faucet](https://riot.im/app/#/room/#canvas_faucet:matrix.parity.io) and deploy your contracts via the [Canvas UI](https://paritytech.github.io/canvas-ui/#/upload).


## Usage

A prerequisite for compiling smart contracts is to have Rust and Cargo installed. Here's [an installation guide](https://doc.rust-lang.org/cargo/getting-started/installation.html).

We recommend installing [`cargo-contract`](https://github.com/paritytech/cargo-contract) as well.
It's a CLI tool which helps set up and manage WebAssembly smart contracts written with ink!:

```
cargo install cargo-contract --force
```

Use the `--force` to ensure you are updated to the most recent `cargo-contract` version.

In order to initialize a new ink! project you can use:

```
cargo contract new flipper
```

This will create a folder `flipper` in your work directory.
The folder contains a scaffold `Cargo.toml` and a `lib.rs`, which both contain the necessary building blocks for using ink!.

The `lib.rs` contains our hello world contract ‒ the `Flipper`, which we explain in the next section.

In order to build the contract just execute this command in the `flipper` folder:
```
cargo +nightly contract build
```

As a result you'll get a file `target/flipper.wasm` file, a `metadata.json` file and a `<contract-name>.contract` file in the `target` folder of your contract.
The `.contract` file combines the Wasm and metadata into one file and needs to be used when deploying the contract.


## Hello, World! ‒ The Flipper

The `Flipper` contract is a simple contract containing only a single `bool` value.
It provides methods to
* flip its value from `true` to `false` (and vice versa) and
* return the current state.


Below you can see the code using the `ink_lang` version of ink!.

```rust
use ink_lang as ink;

#[ink::contract]
mod flipper {
    /// The storage of the flipper contract.
    #[ink(storage)]
    pub struct Flipper {
        /// The single `bool` value.
        value: bool,
    }

    impl Flipper {
        /// Instantiates a new Flipper contract and initializes
        /// `value` to `init_value`.
        #[ink(constructor)]
        pub fn new(init_value: bool) -> Self {
            Self {
                value: init_value,
            }
        }

        /// Flips `value` from `true` to `false` or vice versa.
        #[ink(message)]
        pub fn flip(&mut self) {
            self.value = !self.value;
        }

        /// Returns the current state of `value`.
        #[ink(message)]
        pub fn get(&self) -> bool {
            self.value
        }
    }

    /// Simply execute `cargo test` in order to test your contract
    /// using the below unit tests.
    #[cfg(test)]
    mod tests {
        use super::*;
        use ink_lang as ink;

        #[ink::test]
        fn it_works() {
            let mut flipper = Flipper::new(false);
            assert_eq!(flipper.get(), false);
            flipper.flip();
            assert_eq!(flipper.get(), true);
        }
    }
}
```

The [`flipper/src/lib.rs`](https://github.com/paritytech/ink/blob/master/examples/flipper/lib.rs)
file in our examples folder contains exactly this code. Run `cargo contract build` to build your
first ink! smart contract.

## Examples

In the `examples` folder you'll find a number of examples written in ink!.

Some of the most interesting ones:

* `delegator` ‒ Implements cross-contract calling.
* `trait-erc20` ‒ Defines a trait for `Erc20` contracts and implements it.
* `erc721` ‒ An exemplary implementation of `Erc721` NFT tokens.
* `dns` ‒  A simple `DomainNameService` smart contract.
* …and more, just rummage through the folder 🙃.

To build a single example navigate to the root of the example and run:
```
cargo contract build
```

You should now have an `<name>.contract` file in the `target` folder of the contract.

For information on how to deploy this to a chain, please have a look at the [Play with It](#play-with-it) section or our [smart contracts workshop](https://substrate.dev/substrate-contracts-workshop/).


## How it Works

* Substrate's [Framework for Runtime Aggregation of Modularised Entities (FRAME)](https://substrate.dev/docs/en/knowledgebase/runtime/frame) contains
a module  which implements an API for typical functions smart contracts need (storage, querying information about accounts, …).
This module is called the `contracts` pallet,
* The `contracts` pallet requires smart contracts to be uploaded to the blockchain as a Wasm blob.
* ink! is a smart contract language which targets the API exposed by `contracts`.
Hence ink! contracts are compiled to Wasm.
* When executing `cargo contract build` an additional file `metadata.json` is created.
It contains information about e.g. what methods the contract provides for others to call.

## ink! Macros & Attributes Overview

### Entry Point

In a module annotated with `#[ink::contract]` these attributes are available:

| Attribute | Where Applicable | Description |
|:--|:--|:--|
| `#[ink(storage)]` | On `struct` definitions. | Defines the ink! storage struct. There can only be one ink! storage definition per contract. |
| `#[ink(event)]` | On `struct` definitions. | Defines an ink! event. A contract can define multiple such ink! events. |
| `#[ink(anonymous)]` | Applicable to ink! events. | Tells the ink! codegen to treat the ink! event as anonymous which omits the event signature as topic upon emitting. Very similar to anonymous events in Solidity. |
| `#[ink(topic)]` | Applicate on ink! event field. | Tells the ink! codegen to provide a topic hash for the given field. Every ink! event can only have a limited number of such topic field. Similar semantics as to indexed event arguments in Solidity. |
| `#[ink(message)]` | Applicable to methods. | Flags a method for the ink! storage struct as message making it available to the API for calling the contract. |
| `#[ink(constructor)]` | Applicable to method. | Flags a method for the ink! storage struct as constructor making it available to the API for instantiating the contract. |
| `#[ink(payable)]` | Applicable to ink! messages. | Allows receiving value as part of the call of the ink! message. ink! constructors are implicitly payable. |
| `#[ink(selector = "..")]` | Applicable to ink! messages and ink! constructors. | Specifies a concrete dispatch selector for the flagged entity. This allows a contract author to precisely control the selectors of their APIs making it possible to rename their API without breakage. |
| `#[ink(metadata_name = "..")]` | Applicable to ink! messages, ink! constructors and ink! implementation blocks. | Specifies a concrete name of method/trait inside of metadata. This allows a contract author to precisely control the naming of methods inside of ABI making it possible to rename their methods without breakage. |
| `#[ink(namespace = "..")]` | Applicable to ink! trait implementation blocks. | Changes the resulting selectors of all the ink! messages and ink! constructors within the trait implementation. Allows to disambiguate between trait implementations with overlapping message or constructor names. Use only with great care and consideration! |
| `#[ink(impl)]` | Applicable to ink! implementation blocks. | Tells the ink! codegen that some implementation block shall be granted access to ink! internals even without it containing any ink! messages or ink! constructors. |

See [here](https://paritytech.github.io/ink/ink_lang/attr.contract.html) for a more detailed description of those and also for details on the `#[ink::contract]` macro.

### Trait Definitions

Use`#[ink::trait_definition]` to define your very own trait definitions that are then implementable by ink! smart contracts.
See e.g. the [`examples/trait-erc20`](https://github.com/paritytech/ink/blob/master/examples/trait-erc20/lib.rs#L49-L51) contract on how to utilize it or [the documentation](https://paritytech.github.io/ink/ink_lang/attr.trait_definition.html) for details.

### Off-chain Testing

The `#[ink::test]` procedural macro enables off-chain testing. See e.g. the [`examples/erc20`](https://github.com/paritytech/ink/blob/master/examples/erc20/lib.rs#L278-L280) contract on how to utilize those or [the documentation](https://paritytech.github.io/ink/ink_lang/attr.test.html) for details.

## Developer Documentation

We have [a very comprehensive documentation portal](https://paritytech.github.io/ink-docs),
but if you are looking for the crate level documentation itself, then these are
the relevant links:

| Crate | Docs | Description |
|:--|:--|:--|
`ink_lang` | [![][j1]][j2] | Language features expose by ink!. See [here](https://paritytech.github.io/ink/ink_lang/attr.contract.html) for a detailed description of attributes which you can use in an `#[ink::contract]`. |
`ink_storage` | [![][f1]][f2] | Data structures available in ink!. |
`ink_env` | [![][g1]][g2] | Low-level interface for interacting with the smart contract Wasm executor. |
`ink_prelude` | [![][i1]][i2] | Common API for no_std and std to access alloc crate types. |


## Contributing

Visit our [contribution guidelines](CONTRIBUTING.md) for more information.

Use the scripts provided under `scripts/check-*` directory in order to run checks on either the workspace or all examples. Please do this before pushing work in a PR.

## License

The entire code within this repository is licensed under the [Apache License 2.0](LICENSE).

Please [contact us](https://www.parity.io/contact/) if you have questions about the licensing of our products.
