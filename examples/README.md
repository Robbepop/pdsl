# ink! code examples

This folder contains a set of example contracts for ink!.

Have a look at the different examples to better understand how to use ink! to build your own Substrate smart contracts.

## Preparation

For building the example smart contracts found in this folder you will need to have [`cargo-contract`](https://github.com/paritytech/cargo-contract) installed.

```
cargo install cargo-contract --force
```

We use the `--force` to update to the most recent `cargo-contract` version.

## Build example contract and generate the contracts metadata

To build a single example and generate the contracts Wasm file, navigate to the root of the smart contract and run the following command:

`cargo contract build`

You should now have an optimized `<contract-name>.wasm` file, a `metadata.json` file and a `<contract-name>.contract` file in the `target` folder of your contract.
The `.contract` file combines the Wasm and metadata into one file and can be used for deployment.

## License

The entire code within this repository is licensed under the [GPL v3](LICENSE). Please [contact us](https://www.parity.io/contact/) if you have questions about the licensing of our products.
