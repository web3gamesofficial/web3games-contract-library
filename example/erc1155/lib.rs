#![cfg_attr(not(feature = "std"), no_std)]

pub use self::erc1155::{Erc1155, TokenId, TokenBalance};
use ink_lang as ink;
use ink_prelude::{
    vec::Vec,
};

#[ink::contract]
pub mod erc1155 {
    use ink_storage::collections::{
        HashMap as StorageHashMap,
    };
    use scale::{Encode, Decode};
    use crate::Vec;

    pub type TokenId = u32;
    pub type TokenBalance = u128;

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct Erc1155 {
        balances: StorageHashMap<(AccountId, TokenId), TokenBalance>,
        operator_approvals: StorageHashMap<(AccountId, AccountId), bool>,
    }

    #[ink(event)]
    pub struct TransferSingle {
        operator: AccountId,
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        id: TokenId,
        value: TokenBalance,
    }

    #[ink(event)]
    pub struct TransferBatch {
        operator: AccountId,
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        ids: Vec<TokenId>,
        values: Vec<TokenBalance>,
    }

    #[ink(event)]
    pub struct ApprovalForAll {
        #[ink(topic)]
        account: AccountId,
        #[ink(topic)]
        operator: AccountId,
        approved: bool,
    }

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature="std", derive(scale_info::TypeInfo))]
    pub enum Error {
        InsufficientBalance,
        NotOwnerOrNotApproved,
        ApprovalForSelf,
        InvalidArrayLength,
        InvalidZeroAccount,
        CannotFetchValue,
        CannotInsert,
    }

    impl Erc1155 {
        /// Creates a new ERC1155 token contract.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                balances: StorageHashMap::new(),
                operator_approvals: StorageHashMap::new(),
            }
        }

        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new()
        }

        /// Get the balance of an account's Tokens
        #[ink(message)]
        pub fn balance_of(&self, account: AccountId, id: TokenId) -> TokenBalance {
            self.balance_of_or_zero(&account, &id)
        }

        /// Get the balance of multiple account/token pairs
        #[ink(message)]
        pub fn balance_of_batch(&self, accounts: Vec<AccountId>, ids: Vec<TokenId>) -> Result<Vec<TokenBalance>, Error> {
            if accounts.len() != ids.len() {
                return Err(Error::InvalidArrayLength);
            }

            let mut batch_balances: Vec<TokenBalance> = Vec::new();

            for i in 0..accounts.len() {
                batch_balances.push(self.balance_of_or_zero(&accounts[i], &ids[i]));
            }

            Ok(batch_balances)
        }

        /// Grants or revokes permission to `operator` to transfer the caller's tokens, according to `approved`.
        /// Emits an {ApprovalForAll} event.
        #[ink(message)]
        pub fn set_approval_for_all(&mut self, operator: AccountId, approved: bool) -> Result<(), Error> {
            let caller = self.env().caller();
            if operator == caller {
                return Err(Error::ApprovalForSelf);
            }

            if self.approved_for_all(&caller, &operator) {
                let status = self
                    .operator_approvals
                    .get_mut(&(caller, operator))
                    .ok_or(Error::CannotFetchValue)?;
                *status = approved;
            } else {
                self.operator_approvals.insert((caller, operator), approved);
            }

            self.env().emit_event(ApprovalForAll {
                account: caller,
                operator,
                approved,
            });

            Ok(())
        }

        /// Returns true if `operator` is approved to transfer ``account``'s tokens.
        #[ink(message)]
        pub fn is_approved_for_all(&self, account: AccountId, operator: AccountId) -> bool {
            self.approved_for_all(&account, &operator)
        }

        /// Transfers `value` tokens of token type `id` from `from` to `to`.
        #[ink(message)]
        pub fn safe_transfer_from(&mut self, from: AccountId, to: AccountId, id: TokenId, value: TokenBalance) -> Result<(), Error> {
            let caller = self.env().caller();

            if to == AccountId::from([0x0; 32]) {
                return Err(Error::InvalidZeroAccount);
            }

            // if !(from == caller || self.approved_for_all(&from, &caller)) {
            //     return Err(Error::NotOwnerOrNotApproved);
            // }

            self.transfer_token_from(&from, &to, &id, value)?;

            self.env().emit_event(TransferSingle {
                operator: caller,
                from,
                to,
                id,
                value,
            });

            Ok(())
        }

        /// Send multiple types of Tokens from `from` to `to`.
        #[ink(message)]
        pub fn safe_batch_transfer_from(&mut self, from: AccountId, to: AccountId, ids: Vec<TokenId>, values: Vec<TokenBalance>) -> Result<(), Error> {
            let caller = self.env().caller();

            if ids.len() != values.len() {
                return Err(Error::InvalidArrayLength);
            }

            if to == AccountId::from([0x0; 32]) {
                return Err(Error::InvalidZeroAccount);
            }

            // if !(from == caller || self.approved_for_all(&from, &caller)) {
            //     return Err(Error::NotOwnerOrNotApproved);
            // }

            for i in 0..ids.len() {
                let id = ids[i];
                let value = values[i];

                self.transfer_token_from(&from, &to, &id, value)?;
            }

            self.env().emit_event(TransferBatch {
                operator: caller,
                from,
                to,
                ids,
                values,
            });

            Ok(())
        }

        /// Creates `value` tokens of token type `id`, and assigns them to `account`.
        #[ink(message)]
        pub fn mint(&mut self, to: AccountId, id: TokenId, value: TokenBalance) -> Result<(), Error> {
            let caller = self.env().caller();

            let zero_account = AccountId::from([0x0; 32]);
            if to == zero_account {
                return Err(Error::InvalidZeroAccount);
            }

            self.add_token_to(&to, &id, value)?;

            self.env().emit_event(TransferSingle {
                operator: caller,
                from: zero_account,
                to,
                id,
                value,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn mint_batch(&mut self, to: AccountId, ids: Vec<TokenId>, values: Vec<TokenBalance>) -> Result<(), Error> {
            let caller = self.env().caller();

            let zero_account = AccountId::from([0x0; 32]);
            if to == zero_account {
                return Err(Error::InvalidZeroAccount);
            }

            if ids.len() != values.len() {
                return Err(Error::InvalidArrayLength);
            }

            for i in 0..ids.len() {
                let id = ids[i];
                let value = values[i];
                
                self.add_token_to(&to, &id, value)?;
            }

            self.env().emit_event(TransferBatch {
                operator: caller,
                from: zero_account,
                to,
                ids,
                values,
            });

            Ok(())
        }

        /// Destroys `value` tokens of token type `id` from `account`
        #[ink(message)]
        pub fn burn(&mut self, from: AccountId, id: TokenId, value: TokenBalance) -> Result<(), Error> {
            let caller = self.env().caller();

            let zero_account = AccountId::from([0x0; 32]);
            if from == zero_account {
                return Err(Error::InvalidZeroAccount);
            }

            self.remove_token_from(&from, &id, value)?;

            self.env().emit_event(TransferSingle {
                operator: caller,
                from,
                to: zero_account,
                id,
                value,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn burn_batch(&mut self, from: AccountId, ids: Vec<TokenId>, values: Vec<TokenBalance>) -> Result<(), Error> {
            let caller = self.env().caller();

            let zero_account = AccountId::from([0x0; 32]);
            if from == zero_account {
                return Err(Error::InvalidZeroAccount);
            }

            if ids.len() != values.len() {
                return Err(Error::InvalidArrayLength);
            }

            for i in 0..ids.len() {
                let id = ids[i];
                let value = values[i];

                self.remove_token_from(&from, &id, value)?;
            }

            self.env().emit_event(TransferBatch {
                operator: caller,
                from,
                to: zero_account,
                ids,
                values,
            });

            Ok(())
        }

        fn transfer_token_from(&mut self, from: &AccountId, to: &AccountId, id: &TokenId, value: TokenBalance) -> Result<(), Error> {
            let from_balance = self.balance_of_or_zero(from, id);
            if from_balance < value {
                return Err(Error::InsufficientBalance);
            }

            self.balances.insert((*from, *id), from_balance - value);
            let to_balance = self.balance_of_or_zero(to, id);
            self.balances.insert((*to, *id), to_balance + value);

            Ok(())
        }

        fn add_token_to(&mut self, to: &AccountId, id: &TokenId, value: TokenBalance) -> Result<(), Error> {
            let to_balance = self.balance_of_or_zero(&to, &id);
            self.balances.insert((*to, *id), to_balance + value);

            Ok(())
        }

        fn remove_token_from(&mut self, from: &AccountId, id: &TokenId, value: TokenBalance) -> Result<(), Error> {
            let from_balance = self.balance_of_or_zero(from, id);
            if from_balance < value {
                return Err(Error::InsufficientBalance);
            }

            self.balances.insert((*from, *id), from_balance - value);

            Ok(())
        }

        fn balance_of_or_zero(&self, account: &AccountId, id: &TokenId) -> TokenBalance {
            *self.balances.get(&(*account, *id)).unwrap_or(&0)
        }

        fn approved_for_all(&self, account: &AccountId, operator: &AccountId) -> bool {
            *self.operator_approvals.get(&(*account, *operator)).unwrap_or(&false)
        }

    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// We test if the default constructor does its job.
        #[test]
        fn default_works() {
            let erc1155 = Erc1155::default();
            assert_eq!(erc1155.get(), false);
        }

        /// We test a simple use case of our contract.
        #[test]
        fn it_works() {
            let mut erc1155 = Erc1155::new(false);
            assert_eq!(erc1155.get(), false);
            erc1155.flip();
            assert_eq!(erc1155.get(), true);
        }
    }
}
