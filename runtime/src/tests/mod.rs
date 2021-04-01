use crate::{
	AccountId, Balance, Block, Controller, Currencies,
	CurrencyId::{self, *},
	Dex, EnabledUnderlyingAssetId, Event, LiquidationPools, LiquidityPools, MinterestCouncilMembership,
	MinterestOracle, MinterestProtocol, Prices, Rate, RiskManager, Runtime, System, WhitelistCouncilMembership,
	DOLLARS, PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
};
use controller::{ControllerData, PauseKeeper};
use controller_rpc_runtime_api::runtime_decl_for_ControllerApi::ControllerApi;
use controller_rpc_runtime_api::PoolState;
use controller_rpc_runtime_api::UserPoolBalanceData;
use frame_support::{assert_err, assert_noop, assert_ok, parameter_types};
use frame_support::{error::BadOrigin, pallet_prelude::GenesisBuild, traits::OnFinalize};
use liquidation_pools::{LiquidationPoolData, Sales};
use liquidity_pools::{Pool, PoolUserData};
use minterest_model::MinterestModelData;
use minterest_primitives::{Operation, Price};
use orml_traits::MultiCurrency;
use pallet_traits::{DEXManager, PoolsManager, PriceProvider};
use risk_manager::RiskManagerData;
use sp_runtime::traits::Zero;
use sp_runtime::{DispatchResult, FixedPointNumber};

mod balancing_pools_tests;
mod dexes_tests;
mod liquidation_tests;
mod misc;
mod rpc_tests;

parameter_types! {
	pub ALICE: AccountId = AccountId::from([1u8; 32]);
	pub BOB: AccountId = AccountId::from([2u8; 32]);
	pub CHARLIE: AccountId = AccountId::from([3u8; 32]);
	pub ORACLE1: AccountId = AccountId::from([4u8; 32]);
	pub ORACLE2: AccountId = AccountId::from([5u8; 32]);
	pub ORACLE3: AccountId = AccountId::from([6u8; 32]);

}

struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	pools: Vec<(CurrencyId, Pool)>,
	pool_user_data: Vec<(CurrencyId, AccountId, PoolUserData)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![
				// seed: initial assets. Initial MINT to pay for gas.
				(ALICE::get(), CurrencyId::MNT, 100_000 * DOLLARS),
				(ALICE::get(), CurrencyId::DOT, 100_000 * DOLLARS),
				(ALICE::get(), CurrencyId::ETH, 100_000 * DOLLARS),
				(ALICE::get(), CurrencyId::BTC, 100_000 * DOLLARS),
				(BOB::get(), CurrencyId::MNT, 100_000 * DOLLARS),
				(BOB::get(), CurrencyId::DOT, 100_000 * DOLLARS),
				(BOB::get(), CurrencyId::ETH, 100_000 * DOLLARS),
				(BOB::get(), CurrencyId::BTC, 100_000 * DOLLARS),
				(CHARLIE::get(), CurrencyId::MNT, 100_000 * DOLLARS),
				(CHARLIE::get(), CurrencyId::DOT, 100_000 * DOLLARS),
				(CHARLIE::get(), CurrencyId::ETH, 100_000 * DOLLARS),
				(CHARLIE::get(), CurrencyId::BTC, 100_000 * DOLLARS),
			],
			pools: vec![],
			pool_user_data: vec![],
		}
	}
}

impl ExtBuilder {
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

	pub fn user_balance(mut self, user: AccountId, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts.push((user, currency_id, balance));
		self
	}

	pub fn liquidity_pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((LiquidityPools::pools_account_id(), currency_id, balance));
		self
	}

	pub fn liquidation_pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((LiquidationPools::pools_account_id(), currency_id, balance));
		self
	}

	pub fn dex_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((Dex::dex_account_id(), currency_id, balance));
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
		collateral: bool,
		liquidation_attempts: u8,
	) -> Self {
		self.pool_user_data.push((
			pool_id,
			user,
			PoolUserData {
				total_borrowed,
				interest_index,
				collateral,
				liquidation_attempts,
			},
		));
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		liquidity_pools::GenesisConfig::<Runtime> {
			pools: self.pools,
			pool_user_data: self.pool_user_data,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		controller::GenesisConfig::<Runtime> {
			controller_dates: vec![
				(
					DOT,
					ControllerData {
						// Set the timestamp to one, so that the accrue_interest_rate() does not work.
						timestamp: 1,
						protocol_interest_factor: Rate::saturating_from_rational(1, 10), // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),        // 0.5%
						collateral_factor: Rate::saturating_from_rational(9, 10),        // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					ETH,
					ControllerData {
						// Set the timestamp to one, so that the accrue_interest_rate() does not work.
						timestamp: 1,
						protocol_interest_factor: Rate::saturating_from_rational(1, 10), // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),        // 0.5%
						collateral_factor: Rate::saturating_from_rational(9, 10),        // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					BTC,
					ControllerData {
						// Set the timestamp to one, so that the accrue_interest_rate() does not work.
						timestamp: 1,
						protocol_interest_factor: Rate::saturating_from_rational(1, 10), // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),        // 0.5%
						collateral_factor: Rate::saturating_from_rational(9, 10),        // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
			],
			pause_keepers: vec![
				(
					DOT,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					ETH,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					BTC,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
			],
			whitelist_mode: false,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		minterest_model::GenesisConfig {
			minterest_model_dates: vec![
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
		.assimilate_storage::<Runtime>(&mut t)
		.unwrap();

		risk_manager::GenesisConfig {
			risk_manager_dates: vec![
				(
					CurrencyId::DOT,
					RiskManagerData {
						max_attempts: 3,
						min_sum: 100_000 * DOLLARS,
						threshold: Rate::saturating_from_rational(103, 100),
						liquidation_incentive: Rate::saturating_from_rational(105, 100),
					},
				),
				(
					CurrencyId::ETH,
					RiskManagerData {
						max_attempts: 3,
						min_sum: 100_000 * DOLLARS,
						threshold: Rate::saturating_from_rational(103, 100),
						liquidation_incentive: Rate::saturating_from_rational(105, 100),
					},
				),
				(
					CurrencyId::BTC,
					RiskManagerData {
						max_attempts: 3,
						min_sum: 100_000 * DOLLARS,
						threshold: Rate::saturating_from_rational(103, 100),
						liquidation_incentive: Rate::saturating_from_rational(105, 100),
					},
				),
			],
		}
		.assimilate_storage::<Runtime>(&mut t)
		.unwrap();

		pallet_membership::GenesisConfig::<Runtime, pallet_membership::Instance3> {
			members: vec![ORACLE1::get().clone(), ORACLE2::get().clone(), ORACLE3::get().clone()],
			phantom: Default::default(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		liquidation_pools::GenesisConfig::<Runtime> {
			balancing_period: 30, // Blocks per 3 minutes.
			liquidation_pools: vec![
				(
					CurrencyId::DOT,
					LiquidationPoolData {
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(2, 10),
					},
				),
				(
					CurrencyId::ETH,
					LiquidationPoolData {
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(2, 10),
					},
				),
				(
					CurrencyId::BTC,
					LiquidationPoolData {
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(2, 10),
					},
				),
				(
					CurrencyId::KSM,
					LiquidationPoolData {
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(2, 10),
					},
				),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext: sp_io::TestExternalities = t.into();
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

fn pool_balance(pool_id: CurrencyId) -> Balance {
	Currencies::free_balance(pool_id, &LiquidityPools::pools_account_id())
}

fn liquidation_pool_balance(pool_id: CurrencyId) -> Balance {
	Currencies::free_balance(pool_id, &LiquidationPools::pools_account_id())
}

fn dex_balance(pool_id: CurrencyId) -> Balance {
	Currencies::free_balance(pool_id, &Dex::dex_account_id())
}

fn liquidity_pool_state_rpc(currency_id: CurrencyId) -> Option<PoolState> {
	<Runtime as ControllerApi<Block, AccountId>>::liquidity_pool_state(currency_id)
}

fn get_total_supply_and_borrowed_usd_balance_rpc(account_id: AccountId) -> Option<UserPoolBalanceData> {
	<Runtime as ControllerApi<Block, AccountId>>::get_total_supply_and_borrowed_usd_balance(account_id)
}

fn is_admin_rpc(caller: AccountId) -> Option<bool> {
	<Runtime as ControllerApi<Block, AccountId>>::is_admin(caller)
}

fn dollars(amount: u128) -> u128 {
	amount.saturating_mul(Price::accuracy())
}

fn alice() -> <Runtime as frame_system::Config>::Origin {
	<Runtime as frame_system::Config>::Origin::signed((ALICE::get()).clone())
}

fn bob() -> <Runtime as frame_system::Config>::Origin {
	<Runtime as frame_system::Config>::Origin::signed((BOB::get()).clone())
}

fn charlie() -> <Runtime as frame_system::Config>::Origin {
	<Runtime as frame_system::Config>::Origin::signed((CHARLIE::get()).clone())
}

fn origin_of(account_id: AccountId) -> <Runtime as frame_system::Config>::Origin {
	<Runtime as frame_system::Config>::Origin::signed(account_id)
}

fn origin_none() -> <Runtime as frame_system::Config>::Origin {
	<Runtime as frame_system::Config>::Origin::none()
}

fn origin_root() -> <Runtime as frame_system::Config>::Origin {
	<Runtime as frame_system::Config>::Origin::root()
}

fn set_oracle_price_for_all_pools(price: u128) -> DispatchResult {
	let prices: Vec<(CurrencyId, Price)> = EnabledUnderlyingAssetId::get()
		.into_iter()
		.map(|pool_id| (pool_id, Price::saturating_from_integer(price)))
		.collect();
	MinterestOracle::on_finalize(0);
	assert_ok!(MinterestOracle::feed_values(origin_of(ORACLE1::get().clone()), prices));
	Ok(())
}

pub fn run_to_block(n: u32) {
	while System::block_number() < n {
		MinterestProtocol::on_finalize(System::block_number());
		MinterestOracle::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
	}
}