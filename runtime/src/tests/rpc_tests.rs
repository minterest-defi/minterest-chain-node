use super::*;

#[test]
fn demo_scenario_n2_without_interest_using_rpc_should_work() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
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
				controller::Error::<Runtime>::InsufficientLiquidity
			);
			System::set_block_number(4100);
			assert_ok!(MinterestProtocol::enable_as_collateral(charlie(), DOT));
			System::set_block_number(4200);
			assert_ok!(MinterestProtocol::enable_as_collateral(charlie(), ETH));
			System::set_block_number(4300);
			assert_ok!(Controller::pause_specific_operation(
				<Runtime as frame_system::Config>::Origin::root(),
				DOT,
				Operation::Borrow
			));
			System::set_block_number(4400);
			assert_noop!(
				MinterestProtocol::borrow(charlie(), DOT, 20_000 * DOLLARS),
				minterest_protocol::Error::<Runtime>::OperationPaused
			);
			System::set_block_number(5000);
			assert_ok!(Controller::unpause_specific_operation(
				<Runtime as frame_system::Config>::Origin::root(),
				DOT,
				Operation::Borrow
			));

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

#[test]
fn test_rates_using_rpc() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
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

/// Test that returned values are changed after some blocks passed
#[test]
fn test_user_balance_using_rpc() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_eq!(
				get_total_supply_and_borrowed_usd_balance_rpc(ALICE::get()),
				Some(UserPoolBalanceData {
					total_supply: dollars(0),
					total_borrowed: dollars(0)
				})
			);
			assert_eq!(
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()),
				Some(UserPoolBalanceData {
					total_supply: dollars(0),
					total_borrowed: dollars(0)
				})
			);

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, dollars(70_000)));

			assert_eq!(
				get_total_supply_and_borrowed_usd_balance_rpc(ALICE::get()),
				Some(UserPoolBalanceData {
					total_supply: dollars(0),
					total_borrowed: dollars(0)
				})
			);
			assert_eq!(
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()),
				Some(UserPoolBalanceData {
					total_supply: dollars(240_000),
					total_borrowed: dollars(0)
				})
			);

			assert_ok!(MinterestProtocol::enable_as_collateral(bob(), DOT));
			assert_ok!(MinterestProtocol::enable_as_collateral(bob(), ETH));
			System::set_block_number(20);

			assert_ok!(MinterestProtocol::borrow(bob(), DOT, dollars(50_000)));
			assert_eq!(
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()),
				Some(UserPoolBalanceData {
					total_supply: dollars(240_000),
					total_borrowed: dollars(100_000)
				})
			);

			assert_ok!(MinterestProtocol::repay(bob(), DOT, dollars(30_000)));
			assert_eq!(
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()),
				Some(UserPoolBalanceData {
					total_supply: dollars(240_000),
					total_borrowed: dollars(40_000)
				})
			);

			System::set_block_number(30);
			let account_data = get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()).unwrap_or_default();
			assert!(account_data.total_supply > dollars(240_000));
			assert!(account_data.total_borrowed > dollars(40_000));
		});
}

/// Test that free balance has increased by a (total_supply - total_borrowed) after repay all and
/// redeem
#[test]
fn test_free_balance_is_ok_after_repay_all_and_redeem_using_balance_rpc() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
			System::set_block_number(50);
			assert_ok!(MinterestProtocol::enable_as_collateral(bob(), DOT));
			System::set_block_number(100);
			assert_ok!(MinterestProtocol::borrow(bob(), DOT, dollars(30_000)));
			System::set_block_number(150);
			assert_ok!(MinterestProtocol::repay(bob(), DOT, dollars(10_000)));
			System::set_block_number(200);

			let account_data_before_repay_all =
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()).unwrap_or_default();

			let oracle_price = Prices::get_underlying_price(DOT).unwrap();

			let bob_balance_before_repay_all = Currencies::free_balance(DOT, &BOB::get());

			let expected_free_balance_bob = bob_balance_before_repay_all
				+ (Rate::from_inner(
					account_data_before_repay_all.total_supply - account_data_before_repay_all.total_borrowed,
				) / oracle_price)
					.into_inner();

			assert_ok!(MinterestProtocol::repay_all(bob(), DOT));
			assert_ok!(MinterestProtocol::redeem(bob(), DOT));

			assert_eq!(Currencies::free_balance(DOT, &BOB::get()), expected_free_balance_bob);
		})
}

/// Test that difference between total_borrowed returned by RPC before and after repay is equal to
/// repay amount
#[test]
fn test_total_borrowed_difference_is_ok_before_and_after_repay_using_balance_rpc() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
			System::set_block_number(50);
			assert_ok!(MinterestProtocol::enable_as_collateral(bob(), DOT));
			System::set_block_number(100);
			assert_ok!(MinterestProtocol::borrow(bob(), DOT, dollars(30_000)));
			System::set_block_number(150);

			let account_data_before_repay =
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()).unwrap_or_default();

			let oracle_price = Prices::get_underlying_price(DOT).unwrap();

			assert_ok!(MinterestProtocol::repay(bob(), DOT, dollars(10_000)));
			let account_data_after_repay =
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()).unwrap_or_default();

			assert_eq!(
				LiquidityPools::pool_user_data(DOT, BOB::get()).total_borrowed,
				(Rate::from_inner(account_data_after_repay.total_borrowed) / oracle_price).into_inner()
			);
			assert_eq!(
				dollars(10_000),
				(Rate::from_inner(account_data_before_repay.total_borrowed - account_data_after_repay.total_borrowed)
					/ oracle_price)
					.into_inner()
			);
		})
}

/// Test that difference between total_borrowed returned by RPC before and after borrow is equal to
/// borrow amount
#[test]
fn test_total_borrowed_difference_is_ok_before_and_after_borrow_using_balance_rpc() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
			System::set_block_number(50);
			assert_ok!(MinterestProtocol::enable_as_collateral(bob(), DOT));
			System::set_block_number(100);

			let account_data_before_borrow =
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()).unwrap_or_default();

			let oracle_price = Prices::get_underlying_price(DOT).unwrap();

			assert_ok!(MinterestProtocol::borrow(bob(), DOT, dollars(30_000)));
			let account_data_after_borrow =
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()).unwrap_or_default();

			assert_eq!(
				LiquidityPools::pool_user_data(DOT, BOB::get()).total_borrowed,
				(Rate::from_inner(account_data_after_borrow.total_borrowed) / oracle_price).into_inner()
			);
			assert_eq!(
				dollars(30_000),
				(Rate::from_inner(
					account_data_after_borrow.total_borrowed - account_data_before_borrow.total_borrowed
				) / oracle_price)
					.into_inner()
			);
		})
}

/// Test that difference between total_supply returned by RPC before and after deposit_underlying is
/// equal to deposit amount
#[test]
fn test_total_borrowed_difference_is_ok_before_and_after_deposit_using_balance_rpc() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
			System::set_block_number(50);
			assert_ok!(MinterestProtocol::enable_as_collateral(bob(), DOT));
			System::set_block_number(100);

			let account_data_before_deposit =
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()).unwrap_or_default();

			let oracle_price = Prices::get_underlying_price(DOT).unwrap();

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(30_000)));
			let account_data_after_deposit =
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()).unwrap_or_default();

			assert_eq!(
				dollars(30_000),
				(Rate::from_inner(account_data_after_deposit.total_supply - account_data_before_deposit.total_supply)
					/ oracle_price)
					.into_inner()
			);
		})
}

#[test]
fn is_admin_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(is_admin_rpc(ALICE::get()), Some(false));
		assert_ok!(MinterestCouncilMembership::add_member(
			<Runtime as frame_system::Config>::Origin::root(),
			ALICE::get()
		));
		assert_eq!(is_admin_rpc(ALICE::get()), Some(true));
		assert_eq!(is_admin_rpc(BOB::get()), Some(false));
	})
}