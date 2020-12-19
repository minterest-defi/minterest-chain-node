#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure};
use frame_system::ensure_root;
use minterest_primitives::{Balance, CurrencyId};
use pallet_traits::Borrowing;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{traits::Zero, DispatchResult, FixedU128, RuntimeDebug};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct Reserve {
	pub total_balance: Balance,
	pub current_interest_rate: FixedU128, // FIXME: how can i use it?
	pub total_borrowed: Balance,
	pub current_exchange_rate: FixedU128,
	pub is_lock: bool,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct ReserveUserData<BlockNumber> {
	pub total_borrowed: Balance,
	pub collateral: bool,
	pub timestamp: BlockNumber,
}

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub trait Trait: frame_system::Trait {
	type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;
}

decl_event!(
	pub enum Event {
		/// Reserve locked: \[reserve_id\]
		ReserveLocked(CurrencyId),

		/// Reserve unlocked: \[reserve_id\]
		ReserveUnLocked(CurrencyId),
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {

	ReserveNotFound,

	NotEnoughBalance,

	BalanceOverflowed,
	}
}

decl_storage! {
	 trait Store for Module<T: Trait> as LiquidityPoolsStorage {
		pub Reserves get(fn reserves) config(): map hasher(blake2_128_concat) CurrencyId => Reserve;
		pub ReserveUserDates get(fn reserve_user_data) config(): double_map
			hasher(blake2_128_concat) T::AccountId,
			hasher(blake2_128_concat) CurrencyId => ReserveUserData<T::BlockNumber>;
	}
}

decl_module! {
		pub struct Module<T: Trait> for enum Call where origin: T::Origin {
			type Error = Error<T>;
			fn deposit_event() = default;

			/// Locks all operations (deposit, redeem, borrow, repay)  with the reserve.
			///
			/// The dispatch origin of this call must be _Root_.
			#[weight = 10_000]
			pub fn lock_reserve_transactions(origin, reserve_id: CurrencyId) -> DispatchResult {
				let _ = ensure_root(origin)?;
				ensure!(Self::pool_exists(&reserve_id), Error::<T>::ReserveNotFound);
				Reserves::mutate(reserve_id, |r| r.is_lock = true);
				Self::deposit_event(Event::ReserveLocked(reserve_id));
				Ok(())
			}

			/// Unlocks all operations (deposit, redeem, borrow, repay)  with the reserve.
			///
			/// The dispatch origin of this call must be _Root_.
			#[weight = 10_000]
			pub fn unlock_reserve_transactions(origin, reserve_id: CurrencyId) -> DispatchResult {
				let _ = ensure_root(origin)?;
				ensure!(Self::pool_exists(&reserve_id), Error::<T>::ReserveNotFound);
				Reserves::mutate(reserve_id, |r| r.is_lock = false);
				Self::deposit_event(Event::ReserveUnLocked(reserve_id));
				Ok(())
			}
	}
}

// Setters for LiquidityPools
impl<T: Trait> Module<T> {
	pub fn set_current_interest_rate(underlying_asset_id: CurrencyId, _rate: FixedU128) -> DispatchResult {
		Reserves::mutate(underlying_asset_id, |r| {
			r.current_interest_rate = FixedU128::from_inner(1)
		});
		Ok(())
	}

	pub fn set_current_exchange_rate(underlying_asset_id: CurrencyId, rate: FixedU128) -> DispatchResult {
		Reserves::mutate(underlying_asset_id, |r| r.current_exchange_rate = rate);
		Ok(())
	}

	pub fn update_state_on_deposit(amount: Balance, currency_id: CurrencyId) -> DispatchResult {
		Self::update_reserve_liquidity(amount, Balance::zero(), currency_id)?;

		Ok(())
	}

	pub fn update_state_on_redeem(amount: Balance, currency_id: CurrencyId) -> DispatchResult {
		Self::update_reserve_liquidity(Balance::zero(), amount, currency_id)?;

		Ok(())
	}
}

// Getters for LiquidityPools
impl<T: Trait> Module<T> {
	pub fn get_reserve_available_liquidity(currency_id: CurrencyId) -> Balance {
		Self::reserves(currency_id).total_balance
	}

	pub fn get_reserve_total_borrowed(currency_id: CurrencyId) -> Balance {
		Self::reserves(currency_id).total_borrowed
	}

	pub fn get_user_total_borrowed(who: &T::AccountId, currency_id: CurrencyId) -> Balance {
		Self::reserve_user_data(who, currency_id).total_borrowed
	}

	pub fn check_user_available_collateral(who: &T::AccountId, currency_id: CurrencyId) -> bool {
		Self::reserve_user_data(who, currency_id).collateral
	}

	pub fn get_reserve_total_insurance(_currency_id: CurrencyId) -> Balance {
		//FIXME
		Balance::zero()
	}

	fn pool_exists(underlying_asset_id: &CurrencyId) -> bool {
		Reserves::contains_key(underlying_asset_id)
	}
}

// Private methods for LiquidityPools
impl<T: Trait> Module<T> {
	fn update_reserve_liquidity(
		liquidity_added: Balance,
		liquidity_taken: Balance,
		underlying_asset_id: CurrencyId,
	) -> DispatchResult {
		ensure!(Self::pool_exists(&underlying_asset_id), Error::<T>::ReserveNotFound);

		let current_reserve_balance = Self::reserves(underlying_asset_id).total_balance;

		let new_reserve_balance: Balance;

		if liquidity_added != Balance::zero() {
			new_reserve_balance = current_reserve_balance
				.checked_add(liquidity_added)
				.ok_or(Error::<T>::BalanceOverflowed)?;
		} else {
			new_reserve_balance = current_reserve_balance
				.checked_sub(liquidity_taken)
				.ok_or(Error::<T>::NotEnoughBalance)?;
		}

		Reserves::mutate(underlying_asset_id, |r| r.total_balance = new_reserve_balance);

		Ok(())
	}

	fn update_reserve_and_user_total_borrowed(
		underlying_asset_id: CurrencyId,
		amount_borrowed_add: Balance,
		amount_borrowed_reduce: Balance,
		who: &T::AccountId,
	) -> DispatchResult {
		let current_user_borrow_balance = Self::reserve_user_data(who, underlying_asset_id).total_borrowed;
		let current_total_borrow_balance = Self::reserves(underlying_asset_id).total_borrowed;

		let new_user_borrow_balance: Balance;
		let new_total_borrow_balance: Balance;

		if amount_borrowed_add != Balance::zero() {
			new_user_borrow_balance = current_user_borrow_balance
				.checked_add(amount_borrowed_add)
				.ok_or(Error::<T>::BalanceOverflowed)?;
			new_total_borrow_balance = current_total_borrow_balance
				.checked_add(amount_borrowed_add)
				.ok_or(Error::<T>::BalanceOverflowed)?;
		} else {
			new_user_borrow_balance = current_user_borrow_balance
				.checked_sub(amount_borrowed_reduce)
				.ok_or(Error::<T>::NotEnoughBalance)?;
			new_total_borrow_balance = current_total_borrow_balance
				.checked_sub(amount_borrowed_add)
				.ok_or(Error::<T>::NotEnoughBalance)?;
		}

		ReserveUserDates::<T>::mutate(who, underlying_asset_id, |x| x.total_borrowed = new_user_borrow_balance);
		Reserves::mutate(underlying_asset_id, |x| x.total_borrowed = new_total_borrow_balance);

		Ok(())
	}
}

// Trait Borrowing for LiquidityPools
impl<T: Trait> Borrowing<T::AccountId> for Module<T> {
	fn update_state_on_borrow(
		underlying_asset_id: CurrencyId,
		amount_borrowed: Balance,
		who: &T::AccountId,
	) -> DispatchResult {
		Self::update_reserve_liquidity(Balance::zero(), amount_borrowed, underlying_asset_id)?;
		Self::update_reserve_and_user_total_borrowed(underlying_asset_id, amount_borrowed, Balance::zero(), who)?;
		Ok(())
	}

	fn update_state_on_repay(
		underlying_asset_id: CurrencyId,
		amount_borrowed: Balance,
		who: &T::AccountId,
	) -> DispatchResult {
		Self::update_reserve_liquidity(amount_borrowed, Balance::zero(), underlying_asset_id)?;
		Self::update_reserve_and_user_total_borrowed(underlying_asset_id, Balance::zero(), amount_borrowed, who)?;
		Ok(())
	}
}
