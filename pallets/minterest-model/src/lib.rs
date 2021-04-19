//! # Minterest Model Module
//!
//! ## Overview
//!
//! Minterest Model pallet is responsible for storing and updating parameters related to economy.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use codec::{Decode, Encode};
use frame_support::{ensure, pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use minterest_primitives::{CurrencyId, Rate};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	traits::{CheckedAdd, CheckedDiv, CheckedMul},
	DispatchError, FixedPointNumber, RuntimeDebug,
};
use sp_std::{cmp::Ordering, result};

pub use module::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct MinterestModelData {
	/// The utilization point at which the jump multiplier is applied
	pub kink: Rate,

	/// The base interest rate which is the y-intercept when utilization rate is 0
	pub base_rate_per_block: Rate,

	/// The multiplier of utilization rate that gives the slope of the interest rate
	pub multiplier_per_block: Rate,

	/// The multiplierPerBlock after hitting a specified utilization point
	pub jump_multiplier_per_block: Rate,
}

type RateResult = result::Result<Rate, DispatchError>;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		/// The approximate number of blocks per year
		type BlocksPerYear: Get<u128>;

		/// The origin which may update minterest model parameters. Root can
		/// always do this.
		type ModelUpdateOrigin: EnsureOrigin<Self::Origin>;

		/// Weight information for the extrinsics.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,
		/// Number overflow in calculation.
		NumOverflow,
		/// Base rate per block cannot be set to 0 at the same time as Multiplier per block.
		BaseRatePerBlockCannotBeZero,
		/// Multiplier per block cannot be set to 0 at the same time as Base rate per block.
		MultiplierPerBlockCannotBeZero,
		/// Parameter `kink` cannot be more than one.
		KinkCannotBeMoreThanOne,
		/// Borrow interest rate calculation error.
		BorrowRateCalculationError,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event {
		/// JumpMultiplierPerBlock has been successfully changed.
		JumpMultiplierPerBlockHasChanged,
		/// BaseRatePerBlock has been successfully changed.
		BaseRatePerBlockHasChanged,
		/// MultiplierPerBlock has been successfully changed.
		MultiplierPerBlockHasChanged,
		/// Parameter `kink` has been successfully changed.
		KinkHasChanged,
	}

	/// The Minterest Model data information: `(kink, base_rate_per_block, multiplier_per_block,
	/// jump_multiplier_per_block)`.
	#[pallet::storage]
	#[pallet::getter(fn minterest_model_dates)]
	pub(crate) type MinterestModelParams<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, MinterestModelData, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub minterest_model_dates: Vec<(CurrencyId, MinterestModelData)>,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			GenesisConfig {
				minterest_model_dates: vec![],
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			self.minterest_model_dates
				.iter()
				.for_each(|(currency_id, minterest_model_data)| {
					MinterestModelParams::<T>::insert(
						currency_id,
						MinterestModelData {
							..*minterest_model_data
						},
					)
				});
		}
	}

	#[cfg(feature = "std")]
	impl GenesisConfig {
		/// Direct implementation of `GenesisBuild::build_storage`.
		///
		/// Kept in order not to break dependency.
		pub fn build_storage<T: Config>(&self) -> Result<sp_runtime::Storage, String> {
			<Self as frame_support::traits::GenesisBuild<T>>::build_storage(self)
		}

		/// Direct implementation of `GenesisBuild::assimilate_storage`.
		///
		/// Kept in order not to break dependency.
		pub fn assimilate_storage<T: Config>(&self, storage: &mut sp_runtime::Storage) -> Result<(), String> {
			<Self as frame_support::traits::GenesisBuild<T>>::assimilate_storage(self, storage)
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set JumpMultiplierPerBlock from JumpMultiplierPerYear.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `jump_multiplier_rate_per_year`: used to calculate multiplier per block.
		///
		/// `jump_multiplier_per_block = jump_multiplier_rate_per_year / blocks_per_year`
		/// The dispatch origin of this call must be 'ModelUpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_jump_multiplier_per_year(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			jump_multiplier_rate_per_year: Rate,
		) -> DispatchResultWithPostInfo {
			T::ModelUpdateOrigin::ensure_origin(origin)?;

			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);

			// jump_multiplier_per_block = jump_multiplier_rate_per_year / blocks_per_year
			let new_jump_multiplier_per_block = jump_multiplier_rate_per_year
				.checked_div(&Rate::saturating_from_integer(T::BlocksPerYear::get()))
				.ok_or(Error::<T>::NumOverflow)?;

			// Write the previously calculated values into storage.
			MinterestModelParams::<T>::mutate(pool_id, |r| r.jump_multiplier_per_block = new_jump_multiplier_per_block);

			Self::deposit_event(Event::JumpMultiplierPerBlockHasChanged);

			Ok(().into())
		}

		/// Set BaseRatePerBlock from BaseRatePerYear.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `base_rate_per_year`: used to calculate rate per block.
		///
		/// `base_rate_per_block = base_rate_per_year / blocks_per_year`
		/// The dispatch origin of this call must be 'ModelUpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_base_rate_per_year(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			base_rate_per_year: Rate,
		) -> DispatchResultWithPostInfo {
			T::ModelUpdateOrigin::ensure_origin(origin)?;

			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);

			let new_base_rate_per_block = base_rate_per_year
				.checked_div(&Rate::saturating_from_integer(T::BlocksPerYear::get()))
				.ok_or(Error::<T>::NumOverflow)?;

			// Base rate per block cannot be set to 0 at the same time as Multiplier per block.
			if new_base_rate_per_block.is_zero() {
				ensure!(
					!Self::minterest_model_dates(pool_id).multiplier_per_block.is_zero(),
					Error::<T>::BaseRatePerBlockCannotBeZero
				);
			}

			// Write the previously calculated values into storage.
			MinterestModelParams::<T>::mutate(pool_id, |r| r.base_rate_per_block = new_base_rate_per_block);

			Self::deposit_event(Event::BaseRatePerBlockHasChanged);

			Ok(().into())
		}

		/// Set MultiplierPerBlock from MultiplierPerYear.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `multiplier_per_year`: used to calculate multiplier per block.
		///
		/// `multiplier_per_block = multiplier_per_year / blocks_per_year`
		/// The dispatch origin of this call must be 'ModelUpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_multiplier_per_year(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			multiplier_per_year: Rate,
		) -> DispatchResultWithPostInfo {
			T::ModelUpdateOrigin::ensure_origin(origin)?;

			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);

			let new_multiplier_per_block = multiplier_per_year
				.checked_div(&Rate::saturating_from_integer(T::BlocksPerYear::get()))
				.ok_or(Error::<T>::NumOverflow)?;

			// Multiplier per block cannot be set to 0 at the same time as Base rate per block .
			if new_multiplier_per_block.is_zero() {
				ensure!(
					!Self::minterest_model_dates(pool_id).base_rate_per_block.is_zero(),
					Error::<T>::MultiplierPerBlockCannotBeZero
				);
			}

			// Write the previously calculated values into storage.
			MinterestModelParams::<T>::mutate(pool_id, |r| r.multiplier_per_block = new_multiplier_per_block);
			Self::deposit_event(Event::MultiplierPerBlockHasChanged);
			Ok(().into())
		}

		/// Set parameter `kink`.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `kink`: new kink value, must be less or equal to 1.
		///
		/// The dispatch origin of this call must be 'ModelUpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_kink(origin: OriginFor<T>, pool_id: CurrencyId, kink: Rate) -> DispatchResultWithPostInfo {
			T::ModelUpdateOrigin::ensure_origin(origin)?;

			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);

			ensure!(kink <= Rate::one(), Error::<T>::KinkCannotBeMoreThanOne);

			// Write the previously calculated values into storage.
			MinterestModelParams::<T>::mutate(pool_id, |r| r.kink = kink);
			Self::deposit_event(Event::KinkHasChanged);

			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Calculates the current borrow rate per block.
	/// - `underlying_asset`: Asset ID for which the borrow interest rate is calculated.
	/// - `utilization_rate`: Current Utilization rate value.
	///
	/// returns `borrow_interest_rate`.
	pub fn calculate_borrow_interest_rate(underlying_asset: CurrencyId, utilization_rate: Rate) -> RateResult {
		let MinterestModelData {
			kink,
			base_rate_per_block,
			multiplier_per_block,
			jump_multiplier_per_block,
		} = Self::minterest_model_dates(underlying_asset);

		// if utilization_rate > kink:
		// normal_rate = kink * multiplier_per_block + base_rate_per_block
		// excess_util = utilization_rate * kink
		// borrow_rate = excess_util * jump_multiplier_per_block + normal_rate
		//
		// if utilization_rate <= kink:
		// borrow_rate = utilization_rate * multiplier_per_block + base_rate_per_block
		let borrow_interest_rate = match utilization_rate.cmp(&kink) {
			Ordering::Greater => {
				let normal_rate = kink
					.checked_mul(&multiplier_per_block)
					.and_then(|v| v.checked_add(&base_rate_per_block))
					.ok_or(Error::<T>::BorrowRateCalculationError)?;
				let excess_util = utilization_rate
					.checked_mul(&kink)
					.ok_or(Error::<T>::BorrowRateCalculationError)?;

				excess_util
					.checked_mul(&jump_multiplier_per_block)
					.and_then(|v| v.checked_add(&normal_rate))
					.ok_or(Error::<T>::BorrowRateCalculationError)?
			}
			_ => utilization_rate
				.checked_mul(&multiplier_per_block)
				.and_then(|v| v.checked_add(&base_rate_per_block))
				.ok_or(Error::<T>::BorrowRateCalculationError)?,
		};

		Ok(borrow_interest_rate)
	}
}
