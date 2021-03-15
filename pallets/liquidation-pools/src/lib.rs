//! # Liquidation Pools Module
//!
//! ## Overview
//!
//! Liquidation Pools are responsible for holding funds for automatic liquidation.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use codec::{Decode, Encode};
use frame_support::{ensure, pallet_prelude::*, traits::Get, transactional};
use frame_system::offchain::{SendTransactionTypes, SubmitTransaction};
use frame_system::pallet_prelude::*;
use minterest_primitives::{Balance, CurrencyId, OffchainErr, Rate};
use orml_traits::MultiCurrency;
use pallet_traits::{PoolsManager, PriceProvider};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::traits::{AccountIdConversion, CheckedDiv, CheckedMul, Zero};
use sp_runtime::{transaction_validity::TransactionPriority, FixedPointNumber, ModuleId, RuntimeDebug};
use sp_std::prelude::*;

pub use module::*;
use sp_std::cmp::Ordering;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Liquidation Pool metadata
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct LiquidationPoolData {
	/// Balance Deviation Threshold represents how much current value in a pool may differ from
	/// ideal value (defined by balance_ratio).
	pub deviation_threshold: Rate,
	/// Balance Ration represents the percentage of Working pool value to be covered by value in
	/// Liquidation Poll.
	pub balance_ratio: Rate,
}

type LiquidityPools<T> = liquidity_pools::Module<T>;

#[frame_support::pallet]
pub mod module {
	use super::*;
	use pallet_traits::DEXManager;

	#[pallet::config]
	pub trait Config: frame_system::Config + liquidity_pools::Config + SendTransactionTypes<Call<Self>> {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		type UnsignedPriority: Get<TransactionPriority>;

		#[pallet::constant]
		/// The Liquidation Pool's module id, keep all assets in Pools.
		type LiquidationPoolsModuleId: Get<ModuleId>;

		#[pallet::constant]
		/// The Liquidation Pool's account id, keep all assets in Pools.
		type LiquidationPoolAccountId: Get<Self::AccountId>;

		/// The basic liquidity pools manager.
		type LiquidityPoolsManager: PoolsManager<Self::AccountId>;

		/// The origin which may update liquidation pools parameters. Root can
		/// always do this.
		type UpdateOrigin: EnsureOrigin<Self::Origin>;

		/// The DEX participating in balancing
		type DEX: DEXManager<Self::AccountId, CurrencyId, Balance>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Number overflow in calculation.
		NumOverflow,
		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,
		/// Value must be in range [0..1]
		NotValidDeviationThresholdValue,
		/// Value must be in range [0..1]
		NotValidBalanceRatioValue,
		/// Feed price is invalid
		InvalidFeedPrice,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Liquidation pools are balanced
		LiquidationPoolsBalanced,
		///  Balancing period has been successfully changed: \[new_period\]
		BalancingPeriodChanged(T::BlockNumber),
		///  Deviation Threshold has been successfully changed: \[new_threshold_value\]
		DeviationThresholdChanged(Rate),
		///  Balance ratio has been successfully changed: \[new_threshold_value\]
		BalanceRatioChanged(Rate),
	}

	/// Balancing pool frequency.
	#[pallet::storage]
	#[pallet::getter(fn balancing_period)]
	pub(crate) type BalancingPeriod<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn liquidation_pools_data)]
	pub(crate) type LiquidationPoolsData<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, LiquidationPoolData, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub balancing_period: T::BlockNumber,
		#[allow(clippy::type_complexity)]
		pub liquidation_pools: Vec<(CurrencyId, LiquidationPoolData)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				balancing_period: Default::default(),
				liquidation_pools: vec![],
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			BalancingPeriod::<T>::put(self.balancing_period);
			self.liquidation_pools.iter().for_each(|(currency_id, pool_data)| {
				LiquidationPoolsData::<T>::insert(currency_id, LiquidationPoolData { ..*pool_data })
			});
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		/// Runs balancing liquidation pools every 'balancing_period' blocks.
		fn offchain_worker(now: T::BlockNumber) {
			if let Err(error) = Self::_offchain_worker(now) {
				debug::info!(
					target: "LiquidationPool offchain worker",
					"cannot run offchain worker at {:?}: {:?}",
					now,
					error,
				);
			} else {
				debug::debug!(
					target: "LiquidationPool offchain worker",
					" LiquidationPool offchain worker start at block: {:?} already done!",
					now,
				);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set new value of balancing period.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `new_period`: New value of balancing period.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_balancing_period(origin: OriginFor<T>, new_period: T::BlockNumber) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			// Write new value into storage.
			BalancingPeriod::<T>::put(new_period);
			Self::deposit_event(Event::BalancingPeriodChanged(new_period));
			Ok(().into())
		}

		/// Set new value of deviation threshold.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `new_threshold`: New value of deviation threshold.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_deviation_threshold(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			new_threshold: u128,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;

			ensure!(
				<LiquidityPools<T>>::is_enabled_underlying_asset_id(pool_id),
				Error::<T>::NotValidUnderlyingAssetId
			);

			let new_deviation_threshold = Rate::from_inner(new_threshold);

			ensure!(
				(Rate::zero() <= new_deviation_threshold && new_deviation_threshold <= Rate::one()),
				Error::<T>::NotValidDeviationThresholdValue
			);

			// Write new value into storage.
			LiquidationPoolsData::<T>::mutate(pool_id, |x| x.deviation_threshold = new_deviation_threshold);

			Self::deposit_event(Event::DeviationThresholdChanged(new_deviation_threshold));

			Ok(().into())
		}

		/// Set new value of balance ratio.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `new_balance_ratio`: New value of deviation threshold.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_balance_ratio(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			new_balance_ratio: u128,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;

			ensure!(
				<LiquidityPools<T>>::is_enabled_underlying_asset_id(pool_id),
				Error::<T>::NotValidUnderlyingAssetId
			);

			let new_balance_ratio = Rate::from_inner(new_balance_ratio);

			ensure!(
				(Rate::zero() <= new_balance_ratio && new_balance_ratio <= Rate::one()),
				Error::<T>::NotValidBalanceRatioValue
			);

			// Write new value into storage.
			LiquidationPoolsData::<T>::mutate(pool_id, |x| x.balance_ratio = new_balance_ratio);

			Self::deposit_event(Event::BalanceRatioChanged(new_balance_ratio));

			Ok(().into())
		}

		/// Make balance the liquidation pools.
		///
		/// The dispatch origin of this call must be _None_.
		#[pallet::weight(0)]
		#[transactional]
		pub fn balance_liquidation_pools(
			origin: OriginFor<T>,
			extra_pool_id: CurrencyId,
			shortfall_pool_id: CurrencyId,
			amount: Balance,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_none(origin)?;
			let module_id = Self::pools_account_id();
			T::DEX::swap_with_exact_target(&module_id, extra_pool_id, shortfall_pool_id, amount, amount)?;
			Self::deposit_event(Event::LiquidationPoolsBalanced);
			Ok(().into())
		}
	}
}

/// Used in the liquidation pools balancing algorithm.
#[derive(Debug, Clone)]
struct LiquidationInformation {
	/// CurrencyId
	pool_id: CurrencyId,
	/// Pool current balance in USD.
	balance: Balance,
	/// Ideal pool balance when no balancing is required (USD).
	ideal_balance: Balance,
	/// Pool balance above ideal value (USD).
	oversupply: Balance,
	/// Pool balance below ideal value (USD).
	shortfall: Balance,
}

/// Information about the operations required for balancing Liquidation Pools.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Sales {
	/// Liquidation pool CurrencyId with oversupply.
	pub supply_pool_id: CurrencyId,
	/// Liquidation pool CurrencyId with shortfall.
	pub target_pool_id: CurrencyId,
	/// The amount of underlying asset to transfer from the oversupply pool to the shortfall pool.
	pub amount: Balance,
}

impl<T: Config> Pallet<T> {
	fn submit_unsigned_tx(supply_pool_id: CurrencyId, target_pool_id: CurrencyId, amount: Balance) {
		let call = Call::<T>::balance_liquidation_pools(supply_pool_id, target_pool_id, amount);
		if SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).is_err() {
			debug::info!(
				target: "liquidation-pools offchain worker",
				"submit unsigned balancing tx for \n CurrencyId {:?} and CurrencyId {:?} \nfailed!",
				supply_pool_id, target_pool_id,
			);
		}
	}

	fn _offchain_worker(now: T::BlockNumber) -> Result<(), OffchainErr> {
		// Call a offchain_worker every `balancing_period` blocks
		if now % Self::balancing_period() == T::BlockNumber::zero() {
			return Ok(());
		}
		// Check if we are a potential validator
		if !sp_io::offchain::is_validator() {
			return Err(OffchainErr::NotValidator);
		}
		// Sending transactions to DEX.
		Self::collects_sales_list()
			.map_err(|_| OffchainErr::CheckFail)?
			.iter()
			.for_each(|sale| Self::submit_unsigned_tx(sale.supply_pool_id, sale.target_pool_id, sale.amount));

		Ok(())
	}

	/// Collects information about required transactions on DEX.
	pub fn collects_sales_list() -> sp_std::result::Result<Vec<Sales>, DispatchError> {
		// Collecting information about the current state of liquidation pools.
		let mut information_vec: Vec<LiquidationInformation> = T::EnabledUnderlyingAssetId::get().iter().try_fold(
			Vec::<LiquidationInformation>::new(),
			|mut acc, pool_id| -> sp_std::result::Result<Vec<LiquidationInformation>, DispatchError> {
				let oracle_price =
					T::PriceSource::get_underlying_price(*pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;
				let balance_ratio = Self::liquidation_pools_data(pool_id).balance_ratio;

				// Liquidation pool balance in USD: balance * oracle_price
				let liquidation_pool_balance = Rate::from_inner(Self::get_pool_available_liquidity(*pool_id))
					.checked_mul(&oracle_price)
					.map(|x| x.into_inner())
					.ok_or(Error::<T>::NumOverflow)?;

				// Liquidation pool ideal balance in USD: liquidity_pool_balance * balance_ratio * oracle_price
				let ideal_balance = Rate::from_inner(T::LiquidityPoolsManager::get_pool_available_liquidity(*pool_id))
					.checked_mul(&balance_ratio)
					.and_then(|v| v.checked_mul(&oracle_price))
					.map(|x| x.into_inner())
					.ok_or(Error::<T>::NumOverflow)?;

				let oversupply = if liquidation_pool_balance > ideal_balance {
					liquidation_pool_balance
						.checked_sub(ideal_balance)
						.ok_or(Error::<T>::NumOverflow)?
				} else {
					Balance::zero()
				};
				let shortfall = if liquidation_pool_balance < ideal_balance {
					ideal_balance
						.checked_sub(liquidation_pool_balance)
						.ok_or(Error::<T>::NumOverflow)?
				} else {
					Balance::zero()
				};

				acc.push(LiquidationInformation {
					pool_id: *pool_id,
					balance: liquidation_pool_balance,
					ideal_balance,
					oversupply,
					shortfall,
				});
				Ok(acc)
			},
		)?;

		// Calculate sum_extra and sum_shortfall for all pools.
		let (mut sum_oversupply, mut sum_shortfall) = information_vec.iter().try_fold(
			(Balance::zero(), Balance::zero()),
			|mut acc, pool| -> sp_std::result::Result<(Balance, Balance), DispatchError> {
				let deviation_threshold = Self::liquidation_pools_data(pool.pool_id).deviation_threshold;

				// right_border = ideal_balance + ideal_balance * deviation_threshold
				let right_border = Rate::from_inner(pool.ideal_balance)
					.checked_mul(&deviation_threshold)
					.map(|x| x.into_inner())
					.and_then(|v| v.checked_add(pool.ideal_balance))
					.ok_or(Error::<T>::NumOverflow)?;

				// left_border = ideal_balance - ideal_balance * deviation_threshold
				let left_border = pool
					.ideal_balance
					.checked_sub(
						Rate::from_inner(pool.ideal_balance)
							.checked_mul(&deviation_threshold)
							.map(|x| x.into_inner())
							.ok_or(Error::<T>::NumOverflow)?,
					)
					.ok_or(Error::<T>::NumOverflow)?;

				if pool.balance > right_border {
					acc.0 = acc.0.checked_add(pool.oversupply).ok_or(Error::<T>::NumOverflow)?;
				}
				if pool.balance < left_border {
					acc.1 = acc.1.checked_add(pool.shortfall).ok_or(Error::<T>::NumOverflow)?;
				}
				Ok(acc)
			},
		)?;

		// Contains information about the necessary transactions on the DEX
		let mut to_sell_list: Vec<Sales> = Vec::new();

		while sum_shortfall > Balance::zero() && sum_oversupply > Balance::zero() {
			let (max_oversupply_index, max_oversupply_pool_id, max_oversupply) = information_vec
				.iter()
				.enumerate()
				.max_by(|(_, a), (_, b)| a.oversupply.cmp(&b.oversupply))
				.map(|(index, pool)| (index, pool.pool_id, pool.oversupply))
				.ok_or(Error::<T>::NumOverflow)?;

			let (max_shortfall_index, max_shortfall_pool_id, max_shortfall) = information_vec
				.iter()
				.enumerate()
				.max_by(|(_, a), (_, b)| a.shortfall.cmp(&b.shortfall))
				.map(|(index, pool)| (index, pool.pool_id, pool.shortfall))
				.ok_or(Error::<T>::NumOverflow)?;

			let (bite, bite_in_usd) = match max_shortfall.cmp(&max_oversupply) {
				Ordering::Greater => {
					let oracle_price = T::PriceSource::get_underlying_price(max_oversupply_pool_id)
						.ok_or(Error::<T>::InvalidFeedPrice)?;
					(
						Rate::from_inner(max_oversupply)
							.checked_div(&oracle_price)
							.map(|x| x.into_inner())
							.ok_or(Error::<T>::NumOverflow)?,
						max_oversupply,
					)
				}
				_ => {
					let oracle_price = T::PriceSource::get_underlying_price(max_shortfall_pool_id)
						.ok_or(Error::<T>::InvalidFeedPrice)?;
					(
						Rate::from_inner(max_shortfall)
							.checked_div(&oracle_price)
							.map(|x| x.into_inner())
							.ok_or(Error::<T>::NumOverflow)?,
						max_shortfall,
					)
				}
			};

			to_sell_list.push(Sales {
				supply_pool_id: max_oversupply_pool_id,
				target_pool_id: max_shortfall_pool_id,
				amount: bite,
			});

			information_vec[max_oversupply_index] = LiquidationInformation {
				balance: information_vec[max_oversupply_index]
					.balance
					.checked_sub(bite_in_usd)
					.ok_or(Error::<T>::NumOverflow)?,
				oversupply: information_vec[max_oversupply_index]
					.oversupply
					.checked_sub(bite_in_usd)
					.ok_or(Error::<T>::NumOverflow)?,
				..information_vec[max_oversupply_index]
			};

			information_vec[max_shortfall_index] = LiquidationInformation {
				balance: information_vec[max_shortfall_index]
					.balance
					.checked_add(bite_in_usd)
					.ok_or(Error::<T>::NumOverflow)?,
				shortfall: information_vec[max_shortfall_index]
					.shortfall
					.checked_sub(bite_in_usd)
					.ok_or(Error::<T>::NumOverflow)?,
				..information_vec[max_shortfall_index]
			};

			sum_oversupply = sum_oversupply.checked_sub(bite_in_usd).ok_or(Error::<T>::NumOverflow)?;
			sum_shortfall = sum_shortfall.checked_sub(bite_in_usd).ok_or(Error::<T>::NumOverflow)?;
		}

		Ok(to_sell_list)
	}
}

impl<T: Config> PoolsManager<T::AccountId> for Pallet<T> {
	/// Gets module account id.
	fn pools_account_id() -> T::AccountId {
		T::LiquidationPoolsModuleId::get().into_account()
	}

	/// Gets current the total amount of cash the liquidation pool has.
	fn get_pool_available_liquidity(pool_id: CurrencyId) -> Balance {
		let module_account_id = Self::pools_account_id();
		T::MultiCurrency::free_balance(pool_id, &module_account_id)
	}

	/// Check if pool exists
	fn pool_exists(underlying_asset_id: &CurrencyId) -> bool {
		LiquidationPoolsData::<T>::contains_key(underlying_asset_id)
	}
}

impl<T: Config> ValidateUnsigned for Pallet<T> {
	type Call = Call<T>;

	fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
		match call {
			Call::balance_liquidation_pools(_supply_pool_id, _target_pool_id, _amount) => {
				ValidTransaction::with_tag_prefix("LiquidationPoolsOffchainWorker")
					.priority(T::UnsignedPriority::get())
					.and_provides(<frame_system::Module<T>>::block_number())
					.longevity(64_u64)
					.propagate(true)
					.build()
			}
			_ => InvalidTransaction::Call.into(),
		}
	}
}
