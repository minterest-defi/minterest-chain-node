#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok, error::BadOrigin};

#[test]
fn update_state_on_deposit_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::ETH));
		assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::DOT));
		assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::KSM));
		assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::BTC));
	});
}

#[test]
fn pool_should_exists() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(LiquidityPools::pool_exists(&CurrencyId::DOT), true);
		assert_eq!(LiquidityPools::pool_exists(&CurrencyId::MDOT), false);
	});
}

#[test]
fn pool_not_found() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			LiquidityPools::update_state_on_deposit(100, CurrencyId::MBTC),
			Error::<Runtime>::ReserveNotFound
		);
	});
}

#[test]
fn not_enough_balance() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::DOT));
		assert_noop!(
			LiquidityPools::update_state_on_redeem(101, CurrencyId::DOT),
			Error::<Runtime>::NotEnoughBalance
		);
	});
}

#[test]
fn balance_overflowed() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::DOT));
		assert_noop!(
			LiquidityPools::update_state_on_deposit(Balance::max_value(), CurrencyId::DOT),
			Error::<Runtime>::BalanceOverflowed
		);
	});
}

#[test]
fn add_and_without_liquidity() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::ETH));
		assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::DOT));
		assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::KSM));
		assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::BTC));
		assert_ok!(LiquidityPools::update_state_on_redeem(100, CurrencyId::ETH));
		assert_ok!(LiquidityPools::update_state_on_redeem(100, CurrencyId::DOT));
		assert_ok!(LiquidityPools::update_state_on_redeem(100, CurrencyId::KSM));
		assert_ok!(LiquidityPools::update_state_on_redeem(100, CurrencyId::BTC));
	});
}

#[test]
fn lock_reserve_transactions_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(LiquidityPools::reserves(&CurrencyId::DOT).is_lock, false);
		assert_ok!(LiquidityPools::lock_reserve_transactions(
			Origin::root(),
			CurrencyId::DOT
		));
		assert_eq!(LiquidityPools::reserves(&CurrencyId::DOT).is_lock, true);
		assert_noop!(
			LiquidityPools::lock_reserve_transactions(Origin::signed(ALICE), CurrencyId::DOT),
			BadOrigin
		);
		assert_noop!(
			LiquidityPools::lock_reserve_transactions(Origin::root(), CurrencyId::MDOT),
			Error::<Runtime>::ReserveNotFound
		);
	});
}

#[test]
fn unlock_reserve_transactions_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(LiquidityPools::reserves(&CurrencyId::ETH).is_lock, true);
		assert_ok!(LiquidityPools::unlock_reserve_transactions(
			Origin::root(),
			CurrencyId::ETH
		));
		assert_eq!(LiquidityPools::reserves(&CurrencyId::ETH).is_lock, false);
		assert_noop!(
			LiquidityPools::lock_reserve_transactions(Origin::signed(ALICE), CurrencyId::ETH),
			BadOrigin
		);
		assert_noop!(
			LiquidityPools::lock_reserve_transactions(Origin::root(), CurrencyId::METH),
			Error::<Runtime>::ReserveNotFound
		);
	});
}

#[test]
fn deposit_insurance_should_work() {
	ExtBuilder::default()
		.one_hundred_dots_for_alice()
		.build()
		.execute_with(|| {
			// FIXME This dispatch should only be called as an _Root_.
			assert_noop!(
				LiquidityPools::deposit_insurance(Origin::signed(ALICE), CurrencyId::DOT, 101),
				Error::<Runtime>::NotEnoughBalance
			);
			assert_noop!(
				LiquidityPools::deposit_insurance(Origin::signed(ALICE), CurrencyId::MDOT, 5),
				Error::<Runtime>::ReserveNotFound
			);
			assert_ok!(LiquidityPools::deposit_insurance(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_eq!(LiquidityPools::get_reserve_total_insurance(CurrencyId::DOT), 60);
			assert_eq!(TestMTokens::free_balance(CurrencyId::DOT, &ALICE), 40);
		});
}
