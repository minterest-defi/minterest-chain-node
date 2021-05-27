//! # Integration Tests Module
//!
//! ## Overview
//!
//! Integration Tests pallet is responsible for checking complex test cases with several pallets
//! involved.
//! Tests are split into different files depending on what pallet they are related to. There is also
//! a scenario_tests.rs file which isn`t related to any particular pallet.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
	use frame_support::{assert_noop, assert_ok, ord_parameter_types, pallet_prelude::GenesisBuild, parameter_types};
	use frame_system::{offchain::SendTransactionTypes, EnsureSignedBy};
	use liquidity_pools::{Pool, PoolUserData};
	pub use minterest_primitives::currency::CurrencyType::{UnderlyingAsset, WrappedToken};
	use minterest_primitives::{Balance, CurrencyId, Price, Rate};
	use minterest_protocol::PoolInitData;
	use orml_traits::{parameter_type_with_key, MultiCurrency};
	use sp_core::H256;
	use sp_runtime::{
		testing::{Header, TestXt},
		traits::{AccountIdConversion, BlakeTwo256, IdentityLookup, Zero},
		transaction_validity::TransactionPriority,
		FixedPointNumber, ModuleId,
	};

	use controller::{ControllerData, PauseKeeper};
	use frame_support::traits::Contains;
	use minterest_model::MinterestModelData;
	use minterest_protocol::Error as MinterestProtocolError;
	use pallet_traits::{PoolsManager, PriceProvider};
	use sp_std::cell::RefCell;
	use test_helper::*;

	mod controller_tests;
	mod liquidity_pools_tests;
	mod minterest_model_tests;
	mod minterest_protocol_tests;
	mod mnt_token_tests;
	mod scenario_tests;

	pub type AccountId = u64;

	type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
	type Block = frame_system::mocking::MockBlock<Test>;

	// Configure a mock runtime to test the pallet.
	frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
		{
			System: frame_system::{Module, Call, Config, Storage, Event<T>},
			Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
			Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
			Currencies: orml_currencies::{Module, Call, Event<T>},
			MinterestProtocol: minterest_protocol::{Module, Storage, Call, Event<T>},
			TestPools: liquidity_pools::{Module, Storage, Call, Config<T>},
			TestLiquidationPools: liquidation_pools::{Module, Storage, Call, Event<T>, Config<T>},
			TestController: controller::{Module, Storage, Call, Event, Config<T>},
			TestMinterestModel: minterest_model::{Module, Storage, Call, Event, Config},
			TestDex: dex::{Module, Storage, Call, Event<T>},
			TestMntToken: mnt_token::{Module, Storage, Call, Event<T>, Config<T>},
			TestRiskManager: risk_manager::{Module, Storage, Call, Event<T>, Config},
		}
	);

	ord_parameter_types! {
		pub const ZeroAdmin: AccountId = 0;
	}

	parameter_types! {
		pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/lqdy");
		pub const LiquidationPoolsModuleId: ModuleId = ModuleId(*b"min/lqdn");
		pub const MntTokenModuleId: ModuleId = ModuleId(*b"min/mntt");
		pub LiquidityPoolAccountId: AccountId = LiquidityPoolsModuleId::get().into_account();
		pub LiquidationPoolAccountId: AccountId = LiquidationPoolsModuleId::get().into_account();
		pub MntTokenAccountId: AccountId = MntTokenModuleId::get().into_account();
		pub InitialExchangeRate: Rate = Rate::one();
		pub EnabledUnderlyingAssetsIds: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset);
		pub EnabledWrappedTokensId: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(WrappedToken);
	}

	pub struct WhitelistMembers;
	mock_impl_system_config!(Test);
	mock_impl_balances_config!(Test);
	mock_impl_orml_tokens_config!(Test);
	mock_impl_orml_currencies_config!(Test);
	mock_impl_liquidity_pools_config!(Test);
	mock_impl_liquidation_pools_config!(Test);
	mock_impl_controller_config!(Test, ZeroAdmin);
	mock_impl_minterest_model_config!(Test, ZeroAdmin);
	mock_impl_dex_config!(Test);
	mock_impl_minterest_protocol_config!(Test, ZeroAdmin);
	mock_impl_mnt_token_config!(Test, ZeroAdmin);
	mock_impl_risk_manager_config!(Test, ZeroAdmin);

	pub struct MockPriceSource;

	impl PriceProvider<CurrencyId> for MockPriceSource {
		fn get_underlying_price(_currency_id: CurrencyId) -> Option<Price> {
			Some(Price::one())
		}

		fn lock_price(_currency_id: CurrencyId) {}

		fn unlock_price(_currency_id: CurrencyId) {}
	}

	thread_local! {
		static FOUR: RefCell<Vec<u64>> = RefCell::new(vec![4]);
	}

	impl Contains<u64> for WhitelistMembers {
		fn sorted_members() -> Vec<u64> {
			FOUR.with(|v| v.borrow().clone())
		}
		#[cfg(feature = "runtime-benchmarks")]
		fn add(new: &u128) {
			TEN_TO_FOURTEEN.with(|v| {
				let mut members = v.borrow_mut();
				members.push(*new);
				members.sort();
			})
		}
	}

	pub const ADMIN: AccountId = 0;
	pub const ALICE: AccountId = 1;
	pub const BOB: AccountId = 2;
	pub const CAROL: AccountId = 3;
	pub const ONE_HUNDRED: Balance = 100_000 * DOLLARS;
	pub const BALANCE_ZERO: Balance = 0;
	pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
	pub const RATE_ZERO: Rate = Rate::from_inner(0);
	pub const PROTOCOL_INTEREST_TRANSFER_THRESHOLD: Balance = 1_000_000_000_000_000_000_000;

	pub fn admin() -> Origin {
		Origin::signed(ADMIN)
	}
	pub fn alice() -> Origin {
		Origin::signed(ALICE)
	}
	pub fn bob() -> Origin {
		Origin::signed(BOB)
	}
	pub fn carol() -> Origin {
		Origin::signed(CAROL)
	}

	pub struct ExtBuilder {
		endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
		pools: Vec<(CurrencyId, Pool)>,
		pool_user_data: Vec<(CurrencyId, AccountId, PoolUserData)>,
		minted_pools: Vec<CurrencyId>,
		controller_data: Vec<(CurrencyId, ControllerData<BlockNumber>)>,
		minterest_model_params: Vec<(CurrencyId, MinterestModelData)>,
	}

	impl Default for ExtBuilder {
		fn default() -> Self {
			Self {
				endowed_accounts: vec![],
				pools: vec![],
				pool_user_data: vec![],
				minted_pools: vec![],
				controller_data: vec![
					(
						DOT,
						ControllerData {
							last_interest_accrued_block: 0,
							protocol_interest_factor: Rate::saturating_from_rational(1, 10),
							max_borrow_rate: Rate::saturating_from_rational(5, 1000),
							collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
							borrow_cap: None,
							protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
						},
					),
					(
						ETH,
						ControllerData {
							last_interest_accrued_block: 0,
							protocol_interest_factor: Rate::saturating_from_rational(1, 10),
							max_borrow_rate: Rate::saturating_from_rational(5, 1000),
							collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
							borrow_cap: None,
							protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
						},
					),
					(
						BTC,
						ControllerData {
							last_interest_accrued_block: 0,
							protocol_interest_factor: Rate::saturating_from_rational(1, 10),
							max_borrow_rate: Rate::saturating_from_rational(5, 1000),
							collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
							borrow_cap: None,
							protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
						},
					),
				],
				minterest_model_params: vec![
					(
						DOT,
						MinterestModelData {
							kink: Rate::saturating_from_rational(8, 10),
							base_rate_per_block: Rate::zero(),
							multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
							jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
						},
					),
					(
						ETH,
						MinterestModelData {
							kink: Rate::saturating_from_rational(8, 10),
							base_rate_per_block: Rate::zero(),
							multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
							jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
						},
					),
					(
						BTC,
						MinterestModelData {
							kink: Rate::saturating_from_rational(8, 10),
							base_rate_per_block: Rate::zero(),
							multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
							jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
						},
					),
				],
			}
		}
	}

	impl ExtBuilder {
		pub fn set_controller_data(mut self, pools: Vec<(CurrencyId, ControllerData<BlockNumber>)>) -> Self {
			self.controller_data = pools;
			self
		}

		pub fn set_minterest_model_params(mut self, pools: Vec<(CurrencyId, MinterestModelData)>) -> Self {
			self.minterest_model_params = pools;
			self
		}

		pub fn mnt_enabled_pools(mut self, pools: Vec<CurrencyId>) -> Self {
			self.minted_pools = pools;
			self
		}

		pub fn user_balance(mut self, user: AccountId, currency_id: CurrencyId, balance: Balance) -> Self {
			self.endowed_accounts.push((user, currency_id, balance));
			self
		}

		pub fn pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
			self.endowed_accounts
				.push((TestPools::pools_account_id(), currency_id, balance));
			self
		}

		pub fn pool_total_borrowed(mut self, pool_id: CurrencyId, total_borrowed: Balance) -> Self {
			self.pools.push((
				pool_id,
				Pool {
					total_borrowed,
					borrow_index: Rate::one(),
					total_protocol_interest: Balance::zero(),
				},
			));
			self
		}

		pub fn pool_user_data(
			mut self,
			pool_id: CurrencyId,
			user: AccountId,
			total_borrowed: Balance,
			interest_index: Rate,
			is_collateral: bool,
			liquidation_attempts: u8,
		) -> Self {
			self.pool_user_data.push((
				pool_id,
				user,
				PoolUserData {
					total_borrowed,
					interest_index,
					is_collateral,
					liquidation_attempts,
				},
			));
			self
		}

		pub fn pool_initial(mut self, pool_id: CurrencyId) -> Self {
			self.pools.push((
				pool_id,
				Pool {
					total_borrowed: Balance::zero(),
					borrow_index: Rate::one(),
					total_protocol_interest: Balance::zero(),
				},
			));
			self
		}

		pub fn mnt_account_balance(mut self, balance: Balance) -> Self {
			self.endowed_accounts
				.push((TestMntToken::get_account_id(), MNT, balance));
			self
		}

		pub fn build(self) -> sp_io::TestExternalities {
			let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

			pallet_balances::GenesisConfig::<Test> {
				balances: self
					.endowed_accounts
					.clone()
					.into_iter()
					.filter(|(_, currency_id, _)| *currency_id == MNT)
					.map(|(account_id, _, initial_balance)| (account_id, initial_balance))
					.collect::<Vec<_>>(),
			}
			.assimilate_storage(&mut t)
			.unwrap();

			orml_tokens::GenesisConfig::<Test> {
				endowed_accounts: self
					.endowed_accounts
					.into_iter()
					.filter(|(_, currency_id, _)| *currency_id != MNT)
					.collect::<Vec<_>>(),
			}
			.assimilate_storage(&mut t)
			.unwrap();

			controller::GenesisConfig::<Test> {
				controller_dates: self.controller_data,
				pause_keepers: vec![
					(ETH, PauseKeeper::all_unpaused()),
					(DOT, PauseKeeper::all_unpaused()),
					(KSM, PauseKeeper::all_paused()),
					(BTC, PauseKeeper::all_unpaused()),
				],
				whitelist_mode: false,
			}
			.assimilate_storage(&mut t)
			.unwrap();

			liquidity_pools::GenesisConfig::<Test> {
				pools: self.pools,
				pool_user_data: self.pool_user_data,
			}
			.assimilate_storage(&mut t)
			.unwrap();

			minterest_model::GenesisConfig {
				minterest_model_params: self.minterest_model_params,
			}
			.assimilate_storage::<Test>(&mut t)
			.unwrap();

			mnt_token::GenesisConfig::<Test> {
				mnt_rate: 100_000_000_000_000_000, // 0.1
				mnt_claim_threshold: 100 * DOLLARS,
				minted_pools: self.minted_pools,
				_phantom: Default::default(),
			}
			.assimilate_storage(&mut t)
			.unwrap();

			let mut ext = sp_io::TestExternalities::new(t);
			ext.execute_with(|| System::set_block_number(1));
			ext
		}
	}
}
