// Copyright 2018-2020 Parity Technologies (UK) Ltd.
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

//! # Plain Multisig Wallet
//!
//! This implements a plain multi owner wallet.
//!
//! ## Warning
//!
//! This contract is an *example*. It is neither audited nor endorsed for production use.
//! Do **not** rely on it to keep anything of value secure.
//!
//! ## Overview
//!
//! Each instantiation of this contract has a set of `owners` and a `requirement` of
//! how many of them need to agree on a `Transaction` for it to be able to be executed.
//! Every owner can submit a transaction and when enough of the other owners confirm
//! it will be able to be executed. The following invariant is enforced by the contract:
//!
//! ```ignore
//! 0 < requirement && requirement <= owners && owners <= MAX_OWNERS
//! ```
//!
//! ## Error Handling
//!
//! With the exeception of `execute_transaction` no error conditions are signalled
//! through return types. Any error or invariant violation triggers a panic and therefore
//! rolls back the transaction.
//!
//! ## Interface
//!
//! The interface is modelled after the popular gnosis multisig wallet. However, there
//! are subtle variations from the interface. For example the `confirm_transaction`
//! will never trigger the execution of a `Transaction` even if the treshold is reached.
//! A call of `execute_transaction` is always required. This can be called by anyone.
//!
//! All the messages that are declared as only callable by the wallet must go through
//! the usual submit, confirm, execute cycle as any other transaction that should be
//! called by the wallet. For example to add an owner you would submit a transaction
//! that calls the wallets own `add_owner` message through `submit_transaction`.
//!
//! ### Owner Management
//!
//! The messages `add_owner`, `remove_owner`, and `replace_owner` can be used to manage
//! the owner set after instantiation.
//!
//! ### Changing the Requirement
//!
//! `change_requirement` can be used to tighten or relax the `requirement` of how many
//! owner signatures are needed to execute a `Transaction`.
//!
//! ### Transaction Management
//!
//! `submit_transaction`, `cancel_transaction`, `confirm_transaction`,
//! `revoke_confirmation` and `execute_transaction` are the bread and butter messages
//! of this contract. Use them to dispatch arbitrary messages to other contracts
//! with the wallet as a sender.

#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract(version = "0.1.0", env = ink_core::env::DefaultEnvTypes)]
mod multisig_plain {
    use ink_core::{
        env,
        storage,
    };
    use ink_prelude::vec::Vec;
    use scale::Output;

    /// Tune this to your liking but be wary that allowing too many owners will not perform well.
    const MAX_OWNERS: u32 = 50;

    type TransactionId = u32;
    const WRONG_TRANSACTION_ID: &str =
        "The user specified an invalid transaction id. Abort.";

    /// A wrapper that allows us to pass untyped parameters as blob to a `CallBuilder`
    struct CallInput<'a>(&'a [u8]);

    impl<'a> scale::Encode for CallInput<'a> {
        fn encode_to<T: Output>(&self, dest: &mut T) {
            dest.write(self.0);
        }
    }

    /// A Transaction is what every `owner` can submit for confirmation by other owners.
    /// If enough owners agree it will be executed by the contract.
    #[derive(scale::Encode, scale::Decode, storage::Flush)]
    #[cfg_attr(feature = "ink-generate-abi", derive(type_metadata::Metadata))]
    #[cfg_attr(feature = "std", derive(Debug, PartialEq, Eq))]
    pub struct Transaction {
        /// The AccountId of the contract that is called in this transaction.
        callee: AccountId,
        /// The raw selector which is the function name of the `callee`that is called.
        selector: [u8; 4],
        /// The raw parameters that are passed to the called function.
        input: Vec<u8>,
        /// The amount of chain balance that is transferred to the callee.
        transferred_value: Balance,
        /// Gas limit for the transation.
        gas_limit: u64,
    }

    #[ink(storage)]
    struct MultisigPlain {
        /// Every entry in this map represents the confirmation of an owner for a
        /// transaction. This is effecively a set rather than a map.
        confirmations: storage::BTreeMap<(TransactionId, AccountId), ()>,
        /// The amount of confirmations for every transaction. This is a redundant
        /// information this kept in order to prevent iterating through the
        /// confirmation set to check if a transaction is confirmed.
        confirmation_count: storage::BTreeMap<TransactionId, u32>,
        /// Just the list of transactions. It is a stash as stable ids are necessary
        /// for referencing them in confirmation calls.
        transactions: storage::Stash<Transaction>,
        /// The list is a vector because iterating over it is necessary when cleaning
        /// up the confirmation set.
        owners: storage::Vec<AccountId>,
        /// Redundent information to speed up the check whether a caller is an owner.
        is_owner: storage::BTreeMap<AccountId, ()>,
        /// Minimum number of owners that have to confirm a transaction to be executed.
        requirement: storage::Value<u32>,
    }

    /// Emitted when an owner confirms a transaction.
    #[ink(event)]
    struct Confirmation {
        /// The transaction that was confirmed.
        #[ink(topic)]
        transaction: TransactionId,
        /// The owner that sent the confirmation.
        #[ink(topic)]
        from: AccountId,
    }

    /// Emitted when an owner revoked a confirmation.
    #[ink(event)]
    struct Revokation {
        /// The transaction that was revoked.
        #[ink(topic)]
        transaction: TransactionId,
        /// The owner that sent the revokation.
        #[ink(topic)]
        from: AccountId,
    }

    /// Emitted when an owner submits a transaction.
    #[ink(event)]
    struct Submission {
        /// The transaction that was submitted.
        #[ink(topic)]
        transaction: TransactionId,
    }

    /// Emitted when a transaction was canceled.
    #[ink(event)]
    struct Cancelation {
        /// The transaction that was canceled.
        #[ink(topic)]
        transaction: TransactionId,
    }

    /// Emitted when a transaction was executed.
    #[ink(event)]
    struct Execution {
        /// The transaction that was executed.
        #[ink(topic)]
        transaction: TransactionId,
        /// Indicates whether the transaction executed successfully.
        #[ink(topic)]
        result: Result<(), ()>,
    }

    /// Emitted when an owner is added to the wallet.
    #[ink(event)]
    struct OwnerAddition {
        /// The owner that was added.
        #[ink(topic)]
        owner: AccountId,
    }

    /// Emitted when an owner is removed from the wallet.
    #[ink(event)]
    struct OwnerRemoval {
        /// The owner that was removed.
        #[ink(topic)]
        owner: AccountId,
    }

    /// Emitted when the requirement changed.
    #[ink(event)]
    struct RequirementChange {
        /// The new requirement value.
        new_requirement: u32,
    }

    impl MultisigPlain {
        /// The only constructor of the contract.
        /// A list of owners must be supplied and a number of how many of them must
        /// confirm a transaction. Duplicate owners are silently dropped.
        #[ink(constructor)]
        fn new(&mut self, owners: Vec<AccountId>, requirement: u32) {
            for owner in &owners {
                self.is_owner.insert(*owner, ());
                self.owners.push(*owner);
            }
            self.ensure_requirement_is_valid(self.owners.len(), requirement);
            assert!(self.is_owner.len() == self.owners.len());
            self.requirement.set(requirement);
        }

        /// Add a new owner to the contract.
        /// Panics if the owner already exists.
        /// Only callable by the wallet itself.
        #[ink(message)]
        fn add_owner(&mut self, new_owner: AccountId) {
            self.ensure_from_wallet();
            self.ensure_no_owner(&new_owner);
            self.ensure_requirement_is_valid(self.owners.len() + 1, *self.requirement);
            self.is_owner.insert(new_owner, ());
            self.owners.push(new_owner);
            self.env().emit_event(OwnerAddition { owner: new_owner });
        }

        /// Remove an owner from the contract.
        /// Only callable by the wallet itself. If by doing this the amount of owners
        /// would be smaller than the requirement it is adjusted to be exactly the
        /// number of owners. Panics if `owner` is no owner of the wallet.
        #[ink(message)]
        fn remove_owner(&mut self, owner: AccountId) {
            self.ensure_from_wallet();
            self.ensure_owner(&owner);
            let len = self.owners.len() - 1;
            let requirement = u32::min(len, *self.requirement.get());
            self.ensure_requirement_is_valid(len, requirement);
            self.owners.swap_remove(self.owner_index(&owner));
            self.is_owner.remove(&owner);
            self.requirement.set(requirement);
            self.clean_owner_confirmations(&owner);
            self.env().emit_event(OwnerRemoval { owner });
        }

        /// Replace an owner from the contract with a new one.
        /// Panics if `old_owner` is no owner or if `new_owner` already is one.
        /// Only callable by the wallet itself.
        #[ink(message)]
        fn replace_owner(&mut self, old_owner: AccountId, new_owner: AccountId) {
            self.ensure_from_wallet();
            self.ensure_owner(&old_owner);
            self.ensure_no_owner(&new_owner);
            self.owners
                .replace(self.owner_index(&old_owner), || new_owner);
            self.is_owner.remove(&old_owner);
            self.is_owner.insert(new_owner, ());
            self.clean_owner_confirmations(&old_owner);
            self.env().emit_event(OwnerRemoval { owner: old_owner });
            self.env().emit_event(OwnerAddition { owner: new_owner });
        }

        /// Change the requirement to a new value.
        /// Only callable by the wallet itself.
        #[ink(message)]
        fn change_requirement(&mut self, new_requirement: u32) {
            self.ensure_from_wallet();
            self.ensure_requirement_is_valid(self.owners.len(), new_requirement);
            self.requirement.set(new_requirement);
            self.env().emit_event(RequirementChange { new_requirement });
        }

        /// Add a new transaction candiate to the contract.
        /// This also confirms the transaction for the caller.
        /// This can be called by any owner.
        #[ink(message)]
        fn submit_transaction(&mut self, transaction: Transaction) {
            self.ensure_caller_is_owner();
            let trans_id = self.transactions.put(transaction);
            self.confirmation_count.insert(trans_id, 0);
            self.env().emit_event(Submission {
                transaction: trans_id,
            });
            self.confirm_by_caller(self.env().caller(), trans_id);
        }

        /// Remove a transaction from the contract.
        /// Only callable by the wallet itself.
        /// Panics if `trans_id` is no valid transaction id.
        #[ink(message)]
        fn cancel_transaction(&mut self, trans_id: TransactionId) {
            self.ensure_from_wallet();
            if self.take_transaction(trans_id).is_some() {
                self.env().emit_event(Cancelation {
                    transaction: trans_id,
                });
            }
        }

        /// Confirm a transaction for the sender that was submitted by any owner.
        /// This can be called by any owner.
        /// Panics if `trans_id` is no valid transaction id.
        #[ink(message)]
        fn confirm_transaction(&mut self, trans_id: TransactionId) {
            self.ensure_caller_is_owner();
            self.ensure_transaction_exists(trans_id);
            self.confirm_by_caller(self.env().caller(), trans_id);
        }

        /// Revoke the senders confirmation.
        /// This can be called by any owner.
        /// Panics if `trans_id` is no valid transaction id.
        #[ink(message)]
        fn revoke_confirmation(&mut self, trans_id: TransactionId) {
            self.ensure_caller_is_owner();
            let caller = self.env().caller();
            if self.confirmations.remove(&(trans_id, caller)).is_some() {
                mutate_map(&mut self.confirmation_count, &trans_id, |count| *count -= 1);
                self.env().emit_event(Revokation {
                    transaction: trans_id,
                    from: caller,
                });
            }
        }

        /// Execute a confirmed execution.
        /// Its return type indicates whether the called transaction was succesful.
        /// This can be called by anyone.
        #[ink(message)]
        fn execute_transaction(&mut self, trans_id: TransactionId) -> Result<(), ()> {
            self.ensure_confirmed(trans_id);
            let t = self.take_transaction(trans_id).expect(WRONG_TRANSACTION_ID);
            let result = env::call::CallParams::<EnvTypes, ()>::invoke(
                t.callee,
                t.selector.into(),
            )
            .gas_limit(t.gas_limit)
            .transferred_value(t.transferred_value)
            .push_arg(&CallInput(&t.input))
            .fire()
            .map(|_| ())
            .map_err(|_| ());
            self.env().emit_event(Execution {
                transaction: trans_id,
                result,
            });
            result
        }

        /// Set the `transaction` as confirmed by `confirmer`.
        /// Idempotent operation regarding an already confirmed `transaction`
        /// by `confirmer`.
        fn confirm_by_caller(
            &mut self,
            confirmer: AccountId,
            transaction: TransactionId,
        ) {
            if self
                .confirmations
                .insert((transaction, confirmer), ())
                .is_none()
            {
                mutate_map(&mut self.confirmation_count, &transaction, |count| {
                    *count += 1
                });
                self.env().emit_event(Confirmation {
                    transaction,
                    from: confirmer,
                });
            }
        }

        /// Get the index of `owner` in `self.owners`.
        /// Panics if `owner` is not found in `self.owners`.
        fn owner_index(&self, owner: &AccountId) -> u32 {
            self.owners.iter().position(|x| *x == *owner).expect(
                "This is only called after it was already verified that the id is
                actually an owner.",
            ) as u32
        }

        /// Remove the transaction identified by `trans_id` from `self.transactions`.
        /// Also removes all confirmation state associated with it.
        fn take_transaction(&mut self, trans_id: TransactionId) -> Option<Transaction> {
            let transaction = self.transactions.take(trans_id);
            if transaction.is_some() {
                self.clean_transaction_confirmations(trans_id);
            }
            transaction
        }

        /// Remove all confirmation state associated with `owner`.
        /// Also adjusts the `self.confirmation_count` variable.
        fn clean_owner_confirmations(&mut self, owner: &AccountId) {
            for (trans_id, _) in self.transactions.iter() {
                if self.confirmations.remove(&(trans_id, *owner)).is_some() {
                    mutate_map(&mut self.confirmation_count, &trans_id, |count| {
                        *count += 1
                    });
                }
            }
        }

        /// This removes all confirmation state associated with `transaction`.
        fn clean_transaction_confirmations(&mut self, transaction: TransactionId) {
            for owner in self.owners.iter() {
                self.confirmations.remove(&(transaction, *owner));
            }
            self.confirmation_count.remove(&transaction);
        }

        /// Panic if transaction `trans_id` is not confirmed by at least
        /// `self.requirement` owners.
        fn ensure_confirmed(&self, trans_id: TransactionId) {
            assert!(
                self.confirmation_count
                    .get(&trans_id)
                    .expect(WRONG_TRANSACTION_ID)
                    >= self.requirement.get()
            );
        }

        /// Panic of the transaction `trans_id` does not exit.
        fn ensure_transaction_exists(&self, trans_id: TransactionId) {
            self.transactions.get(trans_id).expect(WRONG_TRANSACTION_ID);
        }

        /// Panic if the sender is no owner of the wallet.
        fn ensure_caller_is_owner(&self) {
            self.ensure_owner(&self.env().caller());
        }

        /// Panic if the sender is not this wallet.
        fn ensure_from_wallet(&self) {
            assert_eq!(self.env().caller(), self.env().account_id());
        }

        /// Panic if `owner` is not an owner,
        fn ensure_owner(&self, owner: &AccountId) {
            assert!(self.is_owner.contains_key(owner));
        }

        /// Panic if `owner` is an owner.
        fn ensure_no_owner(&self, owner: &AccountId) {
            assert!(!self.is_owner.contains_key(owner));
        }

        /// Panic if the number of `owners` under a `requirement` violates our
        /// requirement invariant.
        fn ensure_requirement_is_valid(&self, owners: u32, requirement: u32) {
            assert!(0 < requirement && requirement <= owners && owners <= MAX_OWNERS);
        }
    }

    /// Change a stored value by reinserting it.
    fn mutate_map<K, V, F>(map: &mut storage::BTreeMap<K, V>, key: &K, f: F)
    where
        K: Eq + Ord + scale::Codec + Copy,
        V: scale::Codec + Copy,
        F: FnOnce(&mut V),
    {
        let mut count = *map
            .get(key)
            .expect("User is responsible for only supplying existing keys.");
        f(&mut count);
        map.insert(*key, count);
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink_core::env::{
            call,
            test,
        };
        type Accounts = test::DefaultAccounts<EnvTypes>;
        const WALLET: [u8; 32] = [7; 32];

        impl Transaction {
            fn change_requirement(requirement: u32) -> Self {
                let mut call =
                    call::CallData::new(call::Selector::from_str("change_requirement"));
                call.push_arg(&requirement);
                Self {
                    callee: WALLET.into(),
                    selector: call.selector().to_bytes(),
                    input: call.params().to_owned(),
                    transferred_value: 0,
                    gas_limit: 1000000,
                }
            }
        }

        fn set_sender(sender: AccountId) {
            test::push_execution_context::<EnvTypes>(
                sender,
                WALLET.into(),
                1000000,
                1000000,
                call::CallData::new(call::Selector::from_str("dummy")),
            );
        }

        fn set_from_wallet() {
            set_sender(WALLET.into());
        }

        fn set_from_owner() {
            let accounts = default_accounts();
            set_sender(accounts.alice);
        }

        fn set_from_noowner() {
            let accounts = default_accounts();
            set_sender(accounts.django);
        }

        fn default_accounts() -> Accounts {
            test::default_accounts()
                .expect("Test environment is expected to be initialized.")
        }

        fn build_contract() -> MultisigPlain {
            let accounts = default_accounts();
            let owners = ink_prelude::vec![accounts.alice, accounts.bob, accounts.eve];
            MultisigPlain::new(owners, 2)
        }

        fn submit_transaction() -> MultisigPlain {
            let mut contract = build_contract();
            let accounts = default_accounts();
            set_from_owner();
            contract.submit_transaction(Transaction::change_requirement(1));
            assert_eq!(contract.transactions.len(), 1);
            assert_eq!(test::recorded_events().count(), 2);
            let transaction = contract.transactions.get(0).unwrap();
            assert_eq!(*transaction, Transaction::change_requirement(1));
            contract.confirmations.get(&(0, accounts.alice)).unwrap();
            assert_eq!(contract.confirmations.len(), 1);
            assert_eq!(*contract.confirmation_count.get(&0).unwrap(), 1);
            contract
        }

        #[test]
        fn construction_works() {
            let accounts = default_accounts();
            let owners = ink_prelude::vec![accounts.alice, accounts.bob, accounts.eve];
            let contract = build_contract();

            assert_eq!(contract.owners.len(), 3);
            assert_eq!(*contract.requirement.get(), 2);
            assert!(contract.owners.iter().eq(owners.iter()));
            assert!(contract.is_owner.get(&accounts.alice).is_some());
            assert!(contract.is_owner.get(&accounts.bob).is_some());
            assert!(contract.is_owner.get(&accounts.eve).is_some());
            assert!(contract.is_owner.get(&accounts.charlie).is_none());
            assert!(contract.is_owner.get(&accounts.django).is_none());
            assert!(contract.is_owner.get(&accounts.frank).is_none());
            assert_eq!(contract.confirmations.len(), 0);
            assert_eq!(contract.confirmation_count.len(), 0);
            assert_eq!(contract.transactions.len(), 0);
        }

        #[test]
        #[should_panic]
        fn empty_owner_construction_fails() {
            MultisigPlain::new(vec![], 0);
        }

        #[test]
        #[should_panic]
        fn zero_requirement_construction_fails() {
            let accounts = default_accounts();
            MultisigPlain::new(vec![accounts.alice, accounts.bob], 0);
        }

        #[test]
        #[should_panic]
        fn too_large_requirement_construction_fails() {
            let accounts = default_accounts();
            MultisigPlain::new(vec![accounts.alice, accounts.bob], 3);
        }

        #[test]
        fn add_owner_works() {
            let accounts = default_accounts();
            let mut contract = build_contract();
            set_from_wallet();
            let owners = contract.owners.len();
            contract.add_owner(accounts.frank);
            assert_eq!(contract.owners.len(), owners + 1);
            assert!(contract.is_owner.get(&accounts.frank).is_some());
            assert_eq!(test::recorded_events().count(), 1);
        }

        #[test]
        #[should_panic]
        fn add_owner_existing_fails() {
            let accounts = default_accounts();
            let mut contract = build_contract();
            set_from_wallet();
            contract.add_owner(accounts.bob);
        }

        #[test]
        #[should_panic]
        fn add_owner_permission_denied() {
            let accounts = default_accounts();
            let mut contract = build_contract();
            set_from_owner();
            contract.add_owner(accounts.frank);
        }

        #[test]
        fn remove_owner_works() {
            let accounts = default_accounts();
            let mut contract = build_contract();
            set_from_wallet();
            let owners = contract.owners.len();
            contract.remove_owner(accounts.alice);
            assert_eq!(contract.owners.len(), owners - 1);
            assert!(contract.is_owner.get(&accounts.alice).is_none());
            assert_eq!(test::recorded_events().count(), 1);
        }

        #[test]
        #[should_panic]
        fn remove_owner_nonexisting_fails() {
            let accounts = default_accounts();
            let mut contract = build_contract();
            set_from_wallet();
            contract.remove_owner(accounts.django);
        }

        #[test]
        #[should_panic]
        fn remove_owner_permission_denied() {
            let accounts = default_accounts();
            let mut contract = build_contract();
            set_from_owner();
            contract.remove_owner(accounts.alice);
        }

        #[test]
        fn replace_owner_works() {
            let accounts = default_accounts();
            let mut contract = build_contract();
            set_from_wallet();
            let owners = contract.owners.len();
            contract.replace_owner(accounts.alice, accounts.django);
            assert_eq!(contract.owners.len(), owners);
            assert!(contract.is_owner.get(&accounts.alice).is_none());
            assert!(contract.is_owner.get(&accounts.django).is_some());
            assert_eq!(test::recorded_events().count(), 2);
        }

        #[test]
        #[should_panic]
        fn replace_owner_existing_fails() {
            let accounts = default_accounts();
            let mut contract = build_contract();
            set_from_wallet();
            contract.replace_owner(accounts.alice, accounts.bob);
        }

        #[test]
        #[should_panic]
        fn replace_owner_nonexisting_fails() {
            let accounts = default_accounts();
            let mut contract = build_contract();
            set_from_wallet();
            contract.replace_owner(accounts.django, accounts.frank);
        }

        #[test]
        #[should_panic]
        fn replace_owner_permission_denied() {
            let accounts = default_accounts();
            let mut contract = build_contract();
            set_from_owner();
            contract.replace_owner(accounts.alice, accounts.django);
        }

        #[test]
        fn change_requirement_works() {
            let mut contract = build_contract();
            assert_eq!(*contract.requirement.get(), 2);
            set_from_wallet();
            contract.change_requirement(3);
            assert_eq!(*contract.requirement.get(), 3);
            assert_eq!(test::recorded_events().count(), 1);
        }

        #[test]
        #[should_panic]
        fn change_requirement_too_high() {
            let mut contract = build_contract();
            set_from_wallet();
            contract.change_requirement(4);
        }

        #[test]
        #[should_panic]
        fn change_requirement_zero_fails() {
            let mut contract = build_contract();
            set_from_wallet();
            contract.change_requirement(0);
        }

        #[test]
        fn submit_transaction_works() {
            submit_transaction();
        }

        #[test]
        #[should_panic]
        fn submit_transaction_noowner_fails() {
            let mut contract = build_contract();
            set_from_noowner();
            contract.submit_transaction(Transaction::change_requirement(1));
        }

        #[test]
        #[should_panic]
        fn submit_transaction_wallet_fails() {
            let mut contract = build_contract();
            set_from_wallet();
            contract.submit_transaction(Transaction::change_requirement(1));
        }

        #[test]
        fn cancel_transaction_works() {
            let mut contract = submit_transaction();
            set_from_wallet();
            contract.cancel_transaction(0);
            assert_eq!(contract.transactions.len(), 0);
            assert_eq!(test::recorded_events().count(), 3);
        }

        #[test]
        fn cancel_transaction_nonexisting() {
            let mut contract = submit_transaction();
            set_from_wallet();
            contract.cancel_transaction(1);
            assert_eq!(contract.transactions.len(), 1);
            assert_eq!(test::recorded_events().count(), 2);
        }

        #[test]
        #[should_panic]
        fn cancel_transaction_no_permission() {
            let mut contract = submit_transaction();
            contract.cancel_transaction(0);
        }

        #[test]
        fn confirm_transaction_works() {
            let mut contract = submit_transaction();
            let accounts = default_accounts();
            set_sender(accounts.bob);
            contract.confirm_transaction(0);
            assert_eq!(test::recorded_events().count(), 3);
            contract.confirmations.get(&(0, accounts.bob)).unwrap();
            assert_eq!(contract.confirmations.len(), 2);
            assert_eq!(*contract.confirmation_count.get(&0).unwrap(), 2);
        }

        #[test]
        fn confirm_transaction_already_confirmed() {
            let mut contract = submit_transaction();
            let accounts = default_accounts();
            set_sender(accounts.alice);
            contract.confirm_transaction(0);
            assert_eq!(test::recorded_events().count(), 2);
            contract.confirmations.get(&(0, accounts.alice)).unwrap();
            assert_eq!(contract.confirmations.len(), 1);
            assert_eq!(*contract.confirmation_count.get(&0).unwrap(), 1);
        }

        #[test]
        #[should_panic]
        fn confirm_transaction_noowner_fail() {
            let mut contract = submit_transaction();
            set_from_noowner();
            contract.confirm_transaction(0);
        }

        #[test]
        fn revoke_transaction_works() {
            let mut contract = submit_transaction();
            let accounts = default_accounts();
            set_sender(accounts.alice);
            contract.revoke_confirmation(0);
            assert_eq!(test::recorded_events().count(), 3);
            assert!(contract.confirmations.get(&(0, accounts.alice)).is_none());
            assert_eq!(contract.confirmations.len(), 0);
            assert_eq!(*contract.confirmation_count.get(&0).unwrap(), 0);
        }

        #[test]
        fn revoke_transaction_no_confirmer() {
            let mut contract = submit_transaction();
            let accounts = default_accounts();
            set_sender(accounts.bob);
            contract.revoke_confirmation(0);
            assert_eq!(test::recorded_events().count(), 2);
            assert!(contract.confirmations.get(&(0, accounts.alice)).is_some());
            assert_eq!(contract.confirmations.len(), 1);
            assert_eq!(*contract.confirmation_count.get(&0).unwrap(), 1);
        }

        #[test]
        #[should_panic]
        fn revoke_transaction_noowner_fail() {
            let mut contract = submit_transaction();
            let accounts = default_accounts();
            set_sender(accounts.django);
            contract.revoke_confirmation(0);
        }

        #[test]
        fn execute_transaction_works() {
            // Execution of calls is currently unsupported in off-chain test.
            // Calling execute_transaction panics in any case.
        }
    }
}
