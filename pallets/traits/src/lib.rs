#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::upper_case_acronyms)]

use minterest_primitives::{Balance, CurrencyId, Price, Rate};
use sp_runtime::{DispatchError, DispatchResult};

/// An abstraction of basic borrowing functions
pub trait Borrowing<AccountId> {
	/// Updates the state of the core as a consequence of a borrow action.
	fn update_state_on_borrow(
		who: &AccountId,
		underlying_asset_id: CurrencyId,
		amount_borrowed: Balance,
		account_borrows: Balance,
	) -> DispatchResult;

	/// updates the state of the core as a consequence of a repay action.
	fn update_state_on_repay(
		who: &AccountId,
		underlying_asset_id: CurrencyId,
		repay_amount: Balance,
		account_borrows: Balance,
	) -> DispatchResult;
}

/// An abstraction of pools basic functionalities.
pub trait PoolsManager<AccountId> {
	/// Return module account id.
	fn pools_account_id() -> AccountId;

	/// Return liquidity balance of `pool_id`.
	fn get_pool_available_liquidity(pool_id: CurrencyId) -> Balance;

	/// Check if pool exists
	fn pool_exists(underlying_asset_id: &CurrencyId) -> bool;
}

/// Provides liquidity pool functionality
pub trait LiquidityPoolsManager {
	/// Gets total amount borrowed from the pool.
	fn get_pool_total_borrowed(pool_id: CurrencyId) -> Balance;

	/// Gets pool borrow index
	/// Accumulator of the total earned interest rate since the opening of the pool
	fn get_pool_borrow_index(pool_id: CurrencyId) -> Rate;

	/// Gets current total amount of protocol interest of the underlying held in this pool.
	fn get_pool_total_protocol_interest(pool_id: CurrencyId) -> Balance;
}

pub trait PriceProvider<CurrencyId> {
	fn get_underlying_price(currency_id: CurrencyId) -> Option<Price>;
	fn lock_price(currency_id: CurrencyId);
	fn unlock_price(currency_id: CurrencyId);
}

pub trait DEXManager<AccountId, CurrencyId, Balance> {
	fn swap_with_exact_supply(
		who: &AccountId,
		target_currency_id: CurrencyId,
		supply_currency_id: CurrencyId,
		supply_amount: Balance,
		min_target_amount: Balance,
	) -> sp_std::result::Result<Balance, DispatchError>;

	fn swap_with_exact_target(
		who: &AccountId,
		supply_currency_id: CurrencyId,
		target_currency_id: CurrencyId,
		max_supply_amount: Balance,
		target_amount: Balance,
	) -> sp_std::result::Result<Balance, DispatchError>;
}
