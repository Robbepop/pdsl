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

#[test]
fn compile_tests() {
    let t = trybuild::TestCases::new();

    t.pass("tests/ui/pass/01-noop-contract.rs");
    t.pass("tests/ui/pass/02-flipper-contract.rs");
    t.pass("tests/ui/pass/03-incrementer-contract.rs");
    t.pass("tests/ui/pass/04-erc20-contract.rs");
    t.pass("tests/ui/pass/05-erc721-contract.rs");
    t.pass("tests/ui/pass/06-non-ink-items.rs");
    t.pass("tests/ui/pass/07-flipper-as-dependency.rs");
    t.pass("tests/ui/pass/08-flipper-as-dependency-trait.rs");
    t.pass("tests/ui/pass/09-static-env.rs");
    t.pass("tests/ui/pass/10-derive-for-storage.rs");
    t.pass("tests/ui/pass/11-alias-storage-struct-impl.rs");
    t.pass("tests/ui/pass/12-attributes.rs");

    t.compile_fail("tests/ui/fail/C-00-constructor-self-ref.rs");
    t.compile_fail("tests/ui/fail/C-01-constructor-self-mut.rs");
    t.compile_fail("tests/ui/fail/C-02-constructor-self-val.rs");
    t.compile_fail("tests/ui/fail/C-03-constructor-missing-return.rs");
    t.compile_fail("tests/ui/fail/C-04-missing-constructor.rs");
    t.compile_fail("tests/ui/fail/C-10-async-constructor.rs");
    t.compile_fail("tests/ui/fail/C-11-unsafe-constructor.rs");
    t.compile_fail("tests/ui/fail/C-12-const-constructor.rs");
    t.compile_fail("tests/ui/fail/C-13-abi-constructor.rs");
    t.compile_fail("tests/ui/fail/C-14-payable-constructor.rs");
    t.compile_fail("tests/ui/fail/C-15-payable-trait-constructor.rs");
    t.compile_fail("tests/ui/fail/C-16-function-arg-struct-destructuring.rs");

    t.compile_fail("tests/ui/fail/H-01-invalid-dyn-alloc.rs");
    t.compile_fail("tests/ui/fail/H-02-invalid-as-dependency.rs");
    t.compile_fail("tests/ui/fail/H-03-use-forbidden-idents.rs");

    t.compile_fail("tests/ui/fail/M-01-missing-message.rs");
    t.compile_fail("tests/ui/fail/M-02-message-missing-self-arg.rs");
    t.compile_fail("tests/ui/fail/M-03-message-returns-self.rs");
    t.compile_fail("tests/ui/fail/M-04-message-returns-non-codec.rs");
    t.compile_fail("tests/ui/fail/M-05-message-invalid-selector.rs");
    t.compile_fail("tests/ui/fail/M-10-method-unknown-ink-marker.rs");

    t.compile_fail("tests/ui/fail/S-01-missing-storage-struct.rs");
    t.compile_fail("tests/ui/fail/S-02-multiple-storage-structs.rs");
    t.compile_fail("tests/ui/fail/S-03-struct-unknown-ink-marker.rs");
    t.compile_fail("tests/ui/fail/S-04-non-storage-ink-impls.rs");
    t.compile_fail("tests/ui/fail/S-05-storage-as-event.rs");
    t.compile_fail("tests/ui/fail/S-06-event-as-storage.rs");

    t.pass("tests/ui/chain_extension/E-01-simple.rs");
}
