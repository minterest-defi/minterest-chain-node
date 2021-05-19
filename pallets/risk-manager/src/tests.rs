//! Tests for the risk-manager pallet.
/// Unit tests for liquidation functions see in unit-tests for runtime.
use super::*;
use mock::{Event, *};

use frame_support::{assert_noop, assert_ok};
use sp_runtime::{traits::BadOrigin, FixedPointNumber};

use sp_core::offchain::{
	testing::{TestOffchainExt, TestTransactionPoolExt},
	Externalities, OffchainExt, StorageKind, TransactionPoolExt,
};

use liquidation_pools;
use std::thread;

#[test]
fn test_offchain_worker_lock_expired() {
	let mut ext = ExtBuilder::default()
		.pool_init(ETH)
		.pool_init(BTC)
		.user_balance(ALICE, BTC, 100_000 * DOLLARS)
		.liquidity_pool_balance(BTC, 15_000 * DOLLARS)
		.liquidity_pool_balance(ETH, 15_000 * DOLLARS)
		.liquidation_pool_balance(ETH, 10_000 * DOLLARS)
		.liquidation_pool_balance(BTC, 10_000 * DOLLARS)
		.build();

	let (offchain, _state) = TestOffchainExt::new();
	let mut offchain_cl = offchain.clone();
	let mut offchain_cl2 = offchain.clone();

	let (pool, trans_pool_state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	// As a timestamp returns always the same value (0) in offchain worker
	// This trick is need to emulate lock expiration
	// sleep_until is used to set new timestamp value. It isn't sleep in real.
	thread::spawn(move || {
		let half_sec = std::time::Duration::from_millis(500);
		// This sleep is need to wait when offchain worker will start calculation.
		// After that we set new timestamp value and on next calling guard.extend_lock()
		// will throw expiration error.
		thread::sleep(half_sec);
		offchain_cl.sleep_until(sp_core::offchain::Timestamp::from_unix_millis(30000));
	});

	ext.execute_with(|| {
		System::set_block_number(2);
		assert_ok!(TestMinterestProtocol::deposit_underlying(
			alice(),
			BTC,
			11_000 * DOLLARS
		));
		assert_ok!(TestMinterestProtocol::enable_is_collateral(alice(), BTC));

		System::set_block_number(3);
		assert_ok!(TestMinterestProtocol::borrow(alice(), ETH, 10_500 * DOLLARS));

		System::set_block_number(4);
		// Decrease DOT price. Now alice collateral isn't enough
		// and loan shoud be liquidated
		Prices::unlock_price(admin(), BTC).unwrap();

		assert_ok!(TestRiskManager::_offchain_worker());

		// There are two transactions. One of them is liquidation of loan, another one is balancing of pool
		assert_eq!(trans_pool_state.read().transactions.len(), 2);

		// It check is balancing pool extrinsic was called.
		let transaction = trans_pool_state.write().transactions.pop().unwrap();
		let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
		match ex.call {
			crate::mock::Call::LiquidationPools(liquidation_pools::Call::balance_liquidation_pools(..)) => {}
			e => panic!("Unexpected call: {:?}", e),
		}

		// It check is liquidation extrinsic was called.
		let transaction = trans_pool_state.write().transactions.pop().unwrap();
		let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
		// Called extrinsic input params
		let (who, pool_id) = match ex.call {
			crate::mock::Call::TestRiskManager(crate::Call::liquidate(who, pool_id, ..)) => (who, pool_id),
			e => panic!("Unexpected call: {:?}", e),
		};

		assert_eq!(who, ALICE);
		assert_eq!(pool_id, ETH);
		// Get saved index from database
		let le_index_result = offchain_cl2
			.local_storage_get(StorageKind::LOCAL, OFFCHAIN_WORKER_LATEST_POOL_INDEX)
			.unwrap();
		// If you run test in processor used big-endian byte order(???), this assertion will fail, it's ok.
		// If sequence that produced by CurrencyId::get_enabled_tokens_in_protocol was changed, this
		// assertion can fail.
		assert_eq!(le_index_result[0], 3);

		// Shouldn't fail
		assert_ok!(TestRiskManager::_offchain_worker());
	});
}

#[test]
fn test_offchain_worker_simple_liquidation() {
	let mut ext = ExtBuilder::default()
		.pool_init(DOT)
		.pool_init(KSM)
		.user_balance(ALICE, DOT, 100_000 * DOLLARS)
		.liquidity_pool_balance(DOT, 10_000 * DOLLARS)
		.liquidity_pool_balance(KSM, 15_000 * DOLLARS)
		.build();

	let (offchain, _state) = TestOffchainExt::new();
	let (pool, trans_pool_state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		System::set_block_number(2);
		assert_ok!(TestMinterestProtocol::deposit_underlying(
			alice(),
			DOT,
			11_000 * DOLLARS
		));
		assert_ok!(TestMinterestProtocol::enable_is_collateral(alice(), DOT));

		System::set_block_number(3);
		assert_ok!(TestMinterestProtocol::borrow(alice(), KSM, 10_500 * DOLLARS));

		System::set_block_number(4);
		// Decrease DOT price. Now alice collateral isn't enough
		// and loan shoud be liquidated
		Prices::unlock_price(admin(), DOT).unwrap();

		assert_ok!(TestRiskManager::_offchain_worker());

		assert_eq!(trans_pool_state.read().transactions.len(), 1);
		let transaction = trans_pool_state.write().transactions.pop().unwrap();
		let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();

		// Called extrinsic input params
		let (who, pool_id) = match ex.call {
			crate::mock::Call::TestRiskManager(crate::Call::liquidate(who, pool_id, ..)) => (who, pool_id),
			e => panic!("Unexpected call: {:?}", e),
		};
		assert_eq!(who, ALICE);
		assert_eq!(pool_id, KSM);
	});
}

#[test]
fn set_max_attempts_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_max_attempts(admin(), DOT, 0));
		assert_eq!(TestRiskManager::risk_manager_dates(DOT).max_attempts, 0);
		let expected_event = Event::risk_manager(crate::Event::MaxValueOFLiquidationAttempsHasChanged(0));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set max_attempts equal 2.0
		assert_ok!(TestRiskManager::set_max_attempts(admin(), DOT, 2));
		assert_eq!(TestRiskManager::risk_manager_dates(DOT).max_attempts, 2);
		let expected_event = Event::risk_manager(crate::Event::MaxValueOFLiquidationAttempsHasChanged(2));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(TestRiskManager::set_max_attempts(alice(), DOT, 10), BadOrigin);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_max_attempts(admin(), MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_min_partial_liquidation_sum_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_min_partial_liquidation_sum(
			admin(),
			DOT,
			Balance::zero()
		));
		assert_eq!(
			TestRiskManager::risk_manager_dates(DOT).min_partial_liquidation_sum,
			Balance::zero()
		);
		let expected_event = Event::risk_manager(crate::Event::MinSumForPartialLiquidationHasChanged(Balance::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set min_partial_liquidation_sum equal to one hundred.
		assert_ok!(TestRiskManager::set_min_partial_liquidation_sum(
			admin(),
			DOT,
			ONE_HUNDRED * DOLLARS
		));
		assert_eq!(
			TestRiskManager::risk_manager_dates(DOT).min_partial_liquidation_sum,
			ONE_HUNDRED * DOLLARS
		);
		let expected_event = Event::risk_manager(crate::Event::MinSumForPartialLiquidationHasChanged(
			ONE_HUNDRED * DOLLARS,
		));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_min_partial_liquidation_sum(alice(), DOT, 10),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_min_partial_liquidation_sum(admin(), MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_threshold_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_threshold(admin(), DOT, Rate::zero()));
		assert_eq!(TestRiskManager::risk_manager_dates(DOT).threshold, Rate::zero());
		let expected_event = Event::risk_manager(crate::Event::ValueOfThresholdHasChanged(Rate::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set min_partial_liquidation_sum equal one hundred.
		assert_ok!(TestRiskManager::set_threshold(admin(), DOT, Rate::one()));
		assert_eq!(TestRiskManager::risk_manager_dates(DOT).threshold, Rate::one());
		let expected_event = Event::risk_manager(crate::Event::ValueOfThresholdHasChanged(Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(TestRiskManager::set_threshold(alice(), DOT, Rate::one()), BadOrigin);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_threshold(admin(), MDOT, Rate::one()),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_liquidation_fee_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 1.0
		assert_ok!(TestRiskManager::set_liquidation_fee(admin(), DOT, Rate::one()));
		assert_eq!(TestRiskManager::risk_manager_dates(DOT).liquidation_fee, Rate::one());
		let expected_event = Event::risk_manager(crate::Event::ValueOfLiquidationFeeHasChanged(Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can not be set to 0.0
		assert_noop!(
			TestRiskManager::set_liquidation_fee(admin(), DOT, Rate::zero()),
			Error::<Test>::InvalidLiquidationIncentiveValue
		);

		// Can not be set to 2.0
		assert_noop!(
			TestRiskManager::set_liquidation_fee(admin(), DOT, Rate::saturating_from_integer(2)),
			Error::<Test>::InvalidLiquidationIncentiveValue
		);

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_liquidation_fee(alice(), DOT, Rate::one()),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_liquidation_fee(admin(), MDOT, Rate::one()),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn liquidate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Origin::signed(Alice) is wrong origin for fn liquidate.
		assert_noop!(TestRiskManager::liquidate(Origin::signed(ALICE), ALICE, DOT), BadOrigin);

		// Origin::none is available origin for fn liquidate.
		assert_noop!(
			TestRiskManager::liquidate(Origin::none(), ALICE, DOT),
			minterest_protocol::Error::<Test>::ZeroBalanceTransaction
		);
	})
}

#[test]
fn mutate_liquidation_attempts_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		TestRiskManager::mutate_liquidation_attempts(DOT, &ALICE, true);
		assert_eq!(
			liquidity_pools::PoolUserParams::<Test>::get(DOT, ALICE).liquidation_attempts,
			u8::one()
		);
		TestRiskManager::mutate_liquidation_attempts(DOT, &ALICE, true);
		assert_eq!(
			liquidity_pools::PoolUserParams::<Test>::get(DOT, ALICE).liquidation_attempts,
			2_u8
		);
		TestRiskManager::mutate_liquidation_attempts(DOT, &ALICE, false);
		assert_eq!(
			liquidity_pools::PoolUserParams::<Test>::get(DOT, ALICE).liquidation_attempts,
			u8::zero()
		);
	})
}
