use crate::{
	AccountId, Balance, Block, Controller, Currencies,
	CurrencyId::{self, DOT, ETH},
	EnabledUnderlyingAssetId, Event, LiquidationPools, LiquidationPoolsModuleId, LiquidityPools,
	LiquidityPoolsModuleId, MinterestOracle, MinterestProtocol, Rate, RiskManager, Runtime, System, DOLLARS,
};
use controller::{ControllerData, PauseKeeper};
use controller_rpc_runtime_api::runtime_decl_for_ControllerApi::ControllerApi;
use controller_rpc_runtime_api::PoolState;
use frame_support::{assert_err, assert_noop, assert_ok, parameter_types};
use frame_support::{pallet_prelude::GenesisBuild, traits::OnFinalize};
use liquidity_pools::{Pool, PoolUserData};
use minterest_model::MinterestModelData;
use minterest_primitives::{Operation, Price};
use orml_traits::MultiCurrency;
use pallet_traits::PoolsManager;
use risk_manager::RiskManagerData;
use sp_runtime::traits::{AccountIdConversion, One, Zero};
use sp_runtime::{DispatchResult, FixedPointNumber};

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
				(BOB::get(), CurrencyId::MNT, 100_000 * DOLLARS),
				(BOB::get(), CurrencyId::DOT, 100_000 * DOLLARS),
				(BOB::get(), CurrencyId::ETH, 100_000 * DOLLARS),
				(CHARLIE::get(), CurrencyId::MNT, 100_000 * DOLLARS),
				(CHARLIE::get(), CurrencyId::DOT, 100_000 * DOLLARS),
				(CHARLIE::get(), CurrencyId::ETH, 100_000 * DOLLARS),
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
				total_insurance: Balance::zero(),
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

	pub fn pool_total_borrowed(mut self, pool_id: CurrencyId, total_borrowed: Balance) -> Self {
		self.pools.push((
			pool_id,
			Pool {
				total_borrowed,
				borrow_index: Rate::one(),
				total_insurance: Balance::zero(),
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
					CurrencyId::DOT,
					ControllerData {
						// Set the timestamp to one, so that the accrue_interest_rate() does not work.
						timestamp: 1,
						insurance_factor: Rate::saturating_from_rational(1, 10),  // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000), // 0.5%
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
					},
				),
				(
					CurrencyId::ETH,
					ControllerData {
						// Set the timestamp to one, so that the accrue_interest_rate() does not work.
						timestamp: 1,
						insurance_factor: Rate::saturating_from_rational(1, 10),  // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000), // 0.5%
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
					},
				),
			],
			pause_keepers: vec![
				(
					CurrencyId::DOT,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					CurrencyId::ETH,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		minterest_model::GenesisConfig {
			minterest_model_dates: vec![
				(
					CurrencyId::DOT,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10),
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					CurrencyId::ETH,
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
			],
		}
		.assimilate_storage::<Runtime>(&mut t)
		.unwrap();

		pallet_membership::GenesisConfig::<Runtime, pallet_membership::Instance2> {
			members: vec![ORACLE1::get().clone(), ORACLE2::get().clone(), ORACLE3::get().clone()],
			phantom: Default::default(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		accounts::GenesisConfig::<Runtime> {
			allowed_accounts: vec![(ALICE::get(), ())],
			member_count: u8::one(),
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

fn liquidity_pool_state_rpc(currency_id: CurrencyId) -> Option<PoolState> {
	<Runtime as ControllerApi<Block>>::liquidity_pool_state(currency_id)
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

fn set_oracle_price_for_all_pools(price: u128) -> DispatchResult {
	let prices: Vec<(CurrencyId, Price)> = EnabledUnderlyingAssetId::get()
		.into_iter()
		.map(|pool_id| (pool_id, Price::saturating_from_integer(price)))
		.collect();
	MinterestOracle::on_finalize(0);
	assert_ok!(MinterestOracle::feed_values(origin_of(ORACLE1::get().clone()), prices));
	Ok(())
}

#[test]
fn test_rates_using_rpc() {
	ExtBuilder::default()
		.pool_initial(CurrencyId::DOT)
		.pool_initial(CurrencyId::ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, dollars(100_000)));
			assert_ok!(MinterestProtocol::deposit_underlying(alice(), ETH, dollars(100_000)));

			System::set_block_number(10);

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, dollars(70_000)));
			assert_ok!(MinterestProtocol::enable_as_collateral(bob(), DOT));
			assert_ok!(MinterestProtocol::enable_as_collateral(bob(), ETH));
			// exchange_rate = (150 - 0 + 0) / 150 = 1
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::one(),
					borrow_rate: Rate::zero(),
					supply_rate: Rate::zero()
				})
			);

			System::set_block_number(20);

			assert_ok!(MinterestProtocol::borrow(bob(), DOT, dollars(100_000)));
			assert_ok!(MinterestProtocol::repay(bob(), DOT, dollars(30_000)));
			assert_eq!(pool_balance(DOT), dollars(80_000));
			// exchange_rate = (80 - 0 + 70) / 150 = 1
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::one(),
					borrow_rate: Rate::from_inner(4_200_000_000),
					supply_rate: Rate::from_inner(1_764_000_000)
				})
			);

			System::set_block_number(30);

			assert_ok!(MinterestProtocol::deposit_underlying(charlie(), DOT, dollars(20_000)));
			assert_ok!(MinterestProtocol::deposit_underlying(charlie(), ETH, dollars(30_000)));
			// supply rate and borrow rate decreased
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1_000_000_017_640_000_000),
					borrow_rate: Rate::from_inner(3_705_882_450),
					supply_rate: Rate::from_inner(1_373_356_473)
				})
			);

			System::set_block_number(40);

			assert_ok!(MinterestProtocol::enable_as_collateral(charlie(), DOT));
			assert_ok!(MinterestProtocol::enable_as_collateral(charlie(), ETH));
			assert_ok!(MinterestProtocol::borrow(charlie(), DOT, dollars(20_000)));
			// supply rate and borrow rate increased
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1_000_000_031_373_564_979),
					borrow_rate: Rate::from_inner(4_764_706_035),
					supply_rate: Rate::from_inner(2_270_242_360)
				})
			);
		});
}

#[test]
fn demo_scenario_n2_without_insurance_should_work() {
	ExtBuilder::default()
		.pool_initial(CurrencyId::DOT)
		.pool_initial(CurrencyId::ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, 100_000 * DOLLARS));
			System::set_block_number(200);
			assert_ok!(MinterestProtocol::deposit_underlying(alice(), ETH, 100_000 * DOLLARS));
			System::set_block_number(600);
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, 80_000 * DOLLARS));
			System::set_block_number(1000);
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, 50_000 * DOLLARS));
			System::set_block_number(2000);
			assert_ok!(MinterestProtocol::deposit_underlying(charlie(), DOT, 100_000 * DOLLARS));
			System::set_block_number(3000);
			assert_ok!(MinterestProtocol::deposit_underlying(charlie(), ETH, 50_000 * DOLLARS));
			System::set_block_number(4000);

			assert_noop!(
				MinterestProtocol::borrow(charlie(), DOT, 20_000 * DOLLARS),
				minterest_protocol::Error::<Runtime>::BorrowControllerRejection
			);
			System::set_block_number(4100);
			assert_ok!(MinterestProtocol::enable_as_collateral(charlie(), DOT));
			System::set_block_number(4200);
			assert_ok!(MinterestProtocol::enable_as_collateral(charlie(), ETH));
			System::set_block_number(4300);
			assert_ok!(Controller::pause_specific_operation(alice(), DOT, Operation::Borrow));
			System::set_block_number(4400);
			assert_noop!(
				MinterestProtocol::borrow(charlie(), DOT, 20_000 * DOLLARS),
				minterest_protocol::Error::<Runtime>::OperationPaused
			);
			System::set_block_number(5000);
			assert_ok!(Controller::unpause_specific_operation(alice(), DOT, Operation::Borrow));

			System::set_block_number(6000);
			assert_ok!(MinterestProtocol::borrow(charlie(), DOT, 20_000 * DOLLARS));
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::one(),
					borrow_rate: Rate::from_inner(642857142),
					supply_rate: Rate::from_inner(41326530)
				})
			);
			System::set_block_number(7000);
			assert_ok!(MinterestProtocol::borrow(charlie(), ETH, 10_000 * DOLLARS));
			assert_eq!(
				liquidity_pool_state_rpc(ETH),
				Some(PoolState {
					exchange_rate: Rate::one(),
					borrow_rate: Rate::from_inner(450000000),
					supply_rate: Rate::from_inner(20250000)
				})
			);
			System::set_block_number(8000);
			assert_ok!(MinterestProtocol::borrow(charlie(), ETH, 20_000 * DOLLARS));
			assert_eq!(
				liquidity_pool_state_rpc(ETH),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1000000020250000000),
					borrow_rate: Rate::from_inner(1350000175),
					supply_rate: Rate::from_inner(182250047)
				})
			);
			System::set_block_number(9000);
			assert_ok!(MinterestProtocol::borrow(charlie(), ETH, 70_000 * DOLLARS));
			assert_eq!(
				liquidity_pool_state_rpc(ETH),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1000000202500050963),
					borrow_rate: Rate::from_inner(4500001113),
					supply_rate: Rate::from_inner(2025001001)
				})
			);
			System::set_block_number(10000);
			assert_ok!(MinterestProtocol::repay(charlie(), ETH, 50_000 * DOLLARS));
			assert_eq!(
				liquidity_pool_state_rpc(ETH),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1000002227501463063),
					borrow_rate: Rate::from_inner(2250017263),
					supply_rate: Rate::from_inner(506257768)
				})
			);
			System::set_block_number(11000);
			assert_ok!(MinterestProtocol::borrow(charlie(), DOT, 50_000 * DOLLARS));
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1000000206632652786),
					borrow_rate: Rate::from_inner(2250001601),
					supply_rate: Rate::from_inner(506250720)
				})
			);
			System::set_block_number(12000);
			assert_ok!(MinterestProtocol::repay(charlie(), DOT, 70_000 * DOLLARS));
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1000000712883477935),
					borrow_rate: Rate::from_inner(7128),
					supply_rate: Rate::zero()
				})
			);
			System::set_block_number(13000);
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, 10_000 * DOLLARS));
			System::set_block_number(13500);
			assert_ok!(MinterestProtocol::redeem(charlie(), ETH));
			System::set_block_number(14000);
			assert_ok!(MinterestProtocol::repay_all(charlie(), ETH));
			assert_eq!(
				liquidity_pool_state_rpc(ETH),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1_000_004_371_397_298_691),
					borrow_rate: Rate::zero(),
					supply_rate: Rate::zero()
				})
			);
			System::set_block_number(15000);
			assert_ok!(MinterestProtocol::redeem_underlying(charlie(), DOT, 50_000 * DOLLARS));
			System::set_block_number(16000);
			assert_ok!(MinterestProtocol::repay_all(charlie(), DOT));
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1_000_000_712_883_477_957),
					borrow_rate: Rate::zero(),
					supply_rate: Rate::zero()
				})
			);
			System::set_block_number(17000);
			assert_ok!(MinterestProtocol::redeem(charlie(), DOT));
			System::set_block_number(18000);
			assert_ok!(MinterestProtocol::redeem_underlying(bob(), DOT, 40_000 * DOLLARS));
			System::set_block_number(19000);
			assert_ok!(MinterestProtocol::redeem(bob(), DOT));
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1_000_000_712_883_477_958),
					borrow_rate: Rate::zero(),
					supply_rate: Rate::zero()
				})
			);
			assert_ok!(MinterestProtocol::redeem(bob(), ETH));
			assert_eq!(
				liquidity_pool_state_rpc(ETH),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1_000_004_371_397_298_690),
					borrow_rate: Rate::zero(),
					supply_rate: Rate::zero()
				})
			);
		});
}

// TODO tests for liquidation
#[test]
fn test_liquidation() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Currencies::deposit(
			DOT,
			&LiquidityPoolsModuleId::get().into_account(),
			dollars(200_u128)
		));
		assert_ok!(Currencies::deposit(
			DOT,
			&LiquidationPoolsModuleId::get().into_account(),
			dollars(40_u128)
		));
		assert_eq!(
			Currencies::free_balance(DOT, &LiquidityPoolsModuleId::get().into_account()),
			dollars(200_u128)
		);
		assert_eq!(
			Currencies::free_balance(DOT, &LiquidationPoolsModuleId::get().into_account()),
			dollars(40_u128)
		);
		assert_eq!(LiquidityPools::get_pool_available_liquidity(DOT), dollars(200_u128));
		assert_eq!(LiquidationPools::get_pool_available_liquidity(DOT), dollars(40_u128));
	});
}

#[test]
fn complete_liquidation_one_collateral_should_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(CurrencyId::DOT, 110_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::DOT, 100_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.user_balance(BOB::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.pool_user_data(CurrencyId::DOT, ALICE::get(), 90_000 * DOLLARS, Rate::one(), true, 3)
		.pool_total_borrowed(CurrencyId::DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(RiskManager::liquidate_unsafe_loan(ALICE::get(), CurrencyId::DOT));

			let expected_event = Event::risk_manager(risk_manager::Event::LiquidateUnsafeLoan(
				ALICE::get(),
				180_000 * DOLLARS,
				CurrencyId::DOT,
				vec![CurrencyId::DOT],
				false,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(
				Currencies::free_balance(CurrencyId::MDOT, &ALICE::get()),
				5_500 * DOLLARS
			);

			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(CurrencyId::DOT),
				105_500 * DOLLARS
			);
			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(CurrencyId::DOT),
				104_500 * DOLLARS
			);

			assert_eq!(LiquidityPools::pools(CurrencyId::DOT).total_borrowed, Balance::zero());
			assert_eq!(
				LiquidityPools::pool_user_data(CurrencyId::DOT, ALICE::get()).total_borrowed,
				Balance::zero()
			);

			assert_eq!(
				LiquidityPools::pool_user_data(CurrencyId::DOT, ALICE::get()).liquidation_attempts,
				0
			);
		})
}

#[test]
fn complete_liquidation_multi_collateral_should_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(CurrencyId::DOT, 160_000 * DOLLARS)
		.liquidity_pool_balance(CurrencyId::ETH, 50_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::DOT, 100_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::ETH, 100_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::MDOT, 50_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::METH, 50_000 * DOLLARS)
		.user_balance(BOB::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.user_balance(CHARLIE::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.pool_user_data(CurrencyId::DOT, ALICE::get(), 90_000 * DOLLARS, Rate::one(), true, 3)
		.pool_user_data(CurrencyId::ETH, ALICE::get(), 0, Rate::one(), true, 0)
		.pool_total_borrowed(CurrencyId::DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(RiskManager::liquidate_unsafe_loan(ALICE::get(), CurrencyId::DOT));

			let expected_event = Event::risk_manager(risk_manager::Event::LiquidateUnsafeLoan(
				ALICE::get(),
				180_000 * DOLLARS,
				CurrencyId::DOT,
				vec![CurrencyId::DOT, CurrencyId::ETH],
				false,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(
				Currencies::free_balance(CurrencyId::MDOT, &ALICE::get()),
				Balance::zero()
			);
			assert_eq!(
				Currencies::free_balance(CurrencyId::METH, &ALICE::get()),
				5_500 * DOLLARS
			);

			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(CurrencyId::DOT),
				200_000 * DOLLARS
			);
			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(CurrencyId::ETH),
				5_500 * DOLLARS
			);

			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(CurrencyId::DOT),
				60_000 * DOLLARS
			);
			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(CurrencyId::ETH),
				144_500 * DOLLARS
			);

			assert_eq!(LiquidityPools::pools(CurrencyId::DOT).total_borrowed, Balance::zero());
			assert_eq!(
				LiquidityPools::pool_user_data(CurrencyId::DOT, ALICE::get()).total_borrowed,
				Balance::zero()
			);

			assert_eq!(
				LiquidityPools::pool_user_data(CurrencyId::DOT, ALICE::get()).liquidation_attempts,
				0
			);
		})
}

#[test]
fn partial_liquidation_one_collateral_should_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(CurrencyId::DOT, 110_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::DOT, 100_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.user_balance(BOB::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.pool_user_data(CurrencyId::DOT, ALICE::get(), 90_000 * DOLLARS, Rate::one(), true, 0)
		.pool_total_borrowed(CurrencyId::DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(RiskManager::liquidate_unsafe_loan(ALICE::get(), CurrencyId::DOT));

			let expected_event = Event::risk_manager(risk_manager::Event::LiquidateUnsafeLoan(
				ALICE::get(),
				54_000 * DOLLARS,
				CurrencyId::DOT,
				vec![CurrencyId::DOT],
				true,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(
				Currencies::free_balance(CurrencyId::MDOT, &ALICE::get()),
				71_650 * DOLLARS
			);

			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(CurrencyId::DOT),
				108_650 * DOLLARS
			);
			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(CurrencyId::DOT),
				101_350 * DOLLARS
			);

			assert_eq!(LiquidityPools::pools(CurrencyId::DOT).total_borrowed, 63_000 * DOLLARS);
			assert_eq!(
				LiquidityPools::pool_user_data(CurrencyId::DOT, ALICE::get()).total_borrowed,
				63_000 * DOLLARS
			);

			assert_eq!(
				LiquidityPools::pool_user_data(CurrencyId::DOT, ALICE::get()).liquidation_attempts,
				1
			);
		})
}

#[test]
fn partial_liquidation_multi_collateral_should_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(CurrencyId::DOT, 130_000 * DOLLARS)
		.liquidity_pool_balance(CurrencyId::ETH, 80_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::DOT, 100_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::ETH, 100_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::MDOT, 20_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::METH, 80_000 * DOLLARS)
		.user_balance(BOB::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.user_balance(CHARLIE::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.pool_user_data(CurrencyId::DOT, ALICE::get(), 90_000 * DOLLARS, Rate::one(), true, 0)
		.pool_user_data(CurrencyId::ETH, ALICE::get(), 0, Rate::one(), true, 0)
		.pool_total_borrowed(CurrencyId::DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(RiskManager::liquidate_unsafe_loan(ALICE::get(), CurrencyId::DOT));

			let expected_event = Event::risk_manager(risk_manager::Event::LiquidateUnsafeLoan(
				ALICE::get(),
				54_000 * DOLLARS,
				CurrencyId::DOT,
				vec![CurrencyId::ETH],
				true,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(
				Currencies::free_balance(CurrencyId::MDOT, &ALICE::get()),
				20_000 * DOLLARS
			);
			assert_eq!(
				Currencies::free_balance(CurrencyId::METH, &ALICE::get()),
				51_650 * DOLLARS
			);

			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(CurrencyId::DOT),
				157_000 * DOLLARS
			);
			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(CurrencyId::ETH),
				51_650 * DOLLARS
			);

			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(CurrencyId::DOT),
				73_000 * DOLLARS
			);
			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(CurrencyId::ETH),
				128_350 * DOLLARS
			);

			assert_eq!(LiquidityPools::pools(CurrencyId::DOT).total_borrowed, 63_000 * DOLLARS);
			assert_eq!(
				LiquidityPools::pool_user_data(CurrencyId::DOT, ALICE::get()).total_borrowed,
				63_000 * DOLLARS
			);

			assert_eq!(
				LiquidityPools::pool_user_data(CurrencyId::DOT, ALICE::get()).liquidation_attempts,
				1
			);
		})
}

#[test]
fn complete_liquidation_should_not_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(CurrencyId::DOT, 60_000 * DOLLARS)
		.liquidity_pool_balance(CurrencyId::ETH, 50_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::MDOT, 50_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::METH, 50_000 * DOLLARS)
		.user_balance(CHARLIE::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.pool_user_data(CurrencyId::DOT, ALICE::get(), 90_000 * DOLLARS, Rate::one(), true, 3)
		.pool_user_data(CurrencyId::ETH, ALICE::get(), 0, Rate::one(), false, 0)
		.pool_total_borrowed(CurrencyId::DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_err!(
				RiskManager::liquidate_unsafe_loan(ALICE::get(), CurrencyId::DOT),
				minterest_protocol::Error::<Runtime>::NotEnoughUnderlyingsAssets
			);
		})
}

#[test]
fn partial_liquidation_should_not_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(CurrencyId::DOT, 20_000 * DOLLARS)
		.liquidity_pool_balance(CurrencyId::ETH, 15_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::MDOT, 10_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::METH, 15_000 * DOLLARS)
		.user_balance(CHARLIE::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.pool_user_data(CurrencyId::DOT, ALICE::get(), 90_000 * DOLLARS, Rate::one(), true, 2)
		.pool_user_data(CurrencyId::BTC, ALICE::get(), 0, Rate::one(), true, 0)
		.pool_total_borrowed(CurrencyId::DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_err!(
				RiskManager::liquidate_unsafe_loan(ALICE::get(), CurrencyId::DOT),
				minterest_protocol::Error::<Runtime>::NotEnoughUnderlyingsAssets
			);
		})
}
