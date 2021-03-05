//! Tests for the liquidation-pools pallet.

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Event, *};
use sp_runtime::traits::{BadOrigin, Zero};

#[test]
fn set_balancing_period_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestLiquidationPools::set_balancing_period(admin(), u32::zero()));
		assert_eq!(
			TestLiquidationPools::liquidation_pool_params().balancing_period,
			u32::zero()
		);
		let expected_event = Event::liquidation_pools(crate::Event::BalancingPeriodChanged(ADMIN, u32::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Admin set period equal amount of blocks per year.
		assert_ok!(TestLiquidationPools::set_balancing_period(admin(), 5256000));
		assert_eq!(
			TestLiquidationPools::liquidation_pool_params().balancing_period,
			5256000
		);
		let expected_event = Event::liquidation_pools(crate::Event::BalancingPeriodChanged(ADMIN, 5256000));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestLiquidationPools::set_balancing_period(alice(), 10),
			Error::<Test>::RequireAdmin
		);
	});
}

#[test]
fn set_deviation_threshold_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestLiquidationPools::set_deviation_threshold(
			admin(),
			CurrencyId::DOT,
			0
		));
		assert_eq!(
			TestLiquidationPools::liquidation_pools(CurrencyId::DOT).deviation_threshold,
			Rate::zero()
		);
		let expected_event = Event::liquidation_pools(crate::Event::DeviationThresholdChanged(ADMIN, Rate::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to 1.0
		assert_ok!(TestLiquidationPools::set_deviation_threshold(
			admin(),
			CurrencyId::DOT,
			1_000_000_000_000_000_000u128
		));
		assert_eq!(
			TestLiquidationPools::liquidation_pools(CurrencyId::DOT).deviation_threshold,
			Rate::one()
		);
		let expected_event = Event::liquidation_pools(crate::Event::DeviationThresholdChanged(ADMIN, Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can not be set grater than 1.0
		assert_noop!(
			TestLiquidationPools::set_deviation_threshold(admin(), CurrencyId::DOT, 2_000_000_000_000_000_000u128),
			Error::<Test>::NotValidDeviationThresholdValue
		);

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestLiquidationPools::set_deviation_threshold(alice(), CurrencyId::DOT, 10),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestLiquidationPools::set_deviation_threshold(admin(), CurrencyId::MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_balance_ratio_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestLiquidationPools::set_balance_ratio(admin(), CurrencyId::DOT, 0));
		assert_eq!(
			TestLiquidationPools::liquidation_pools(CurrencyId::DOT).balance_ratio,
			Rate::zero()
		);
		let expected_event = Event::liquidation_pools(crate::Event::BalanceRatioChanged(ADMIN, Rate::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to 1.0
		assert_ok!(TestLiquidationPools::set_balance_ratio(
			admin(),
			CurrencyId::DOT,
			1_000_000_000_000_000_000u128
		));
		assert_eq!(
			TestLiquidationPools::liquidation_pools(CurrencyId::DOT).balance_ratio,
			Rate::one()
		);
		let expected_event = Event::liquidation_pools(crate::Event::BalanceRatioChanged(ADMIN, Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can not be set grater than 1.0
		assert_noop!(
			TestLiquidationPools::set_balance_ratio(admin(), CurrencyId::DOT, 2_000_000_000_000_000_000u128),
			Error::<Test>::NotValidBalanceRatioValue
		);

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestLiquidationPools::set_balance_ratio(alice(), CurrencyId::DOT, 10),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestLiquidationPools::set_balance_ratio(admin(), CurrencyId::MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn balancing_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		// Origin::signed(Alice) is wrong origin for fn balancing.
		assert_noop!(TestLiquidationPools::balancing(Origin::signed(ALICE)), BadOrigin);

		// Origin::none is available origin for fn balancing.
		assert_ok!(TestLiquidationPools::balancing(Origin::none()));
	});
}

#[test]
fn calculate_deadline_should_work() {
	ExternalityBuilder::default()
		.pool_timestamp_and_period(1, 600)
		.build()
		.execute_with(|| {
			assert_eq!(TestLiquidationPools::calculate_deadline(), Ok(601));

			TestLiquidationPools::set_balancing_period(admin(), u32::MAX).unwrap_or_default();

			assert_noop!(TestLiquidationPools::calculate_deadline(), Error::<Test>::NumOverflow);
		});
}

#[test]
fn balancing_attempt_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		assert_eq!(TestLiquidationPools::balancing_attempt(), Ok(().into()));
	});
}

#[test]
fn calculate_sum_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		assert_eq!(
			TestLiquidationPools::calculate_sum(
				&vec![(CurrencyId::DOT, 4_000 * DOLLARS), (CurrencyId::KSM, 6_000 * DOLLARS)],
				&vec![(CurrencyId::BTC, 20_000 * DOLLARS)]
			),
			Ok((10_000 * DOLLARS, 20_000 * DOLLARS))
		);
		assert_noop!(
			TestLiquidationPools::calculate_sum(
				&vec![
					(CurrencyId::DOT, Balance::max_value()),
					(CurrencyId::KSM, 6_000 * DOLLARS)
				],
				&vec![(CurrencyId::BTC, 20_000 * DOLLARS)]
			),
			Error::<Test>::NumOverflow
		);
		assert_noop!(
			TestLiquidationPools::calculate_sum(
				&vec![(CurrencyId::DOT, 4_000 * DOLLARS), (CurrencyId::KSM, 6_000 * DOLLARS)],
				&vec![
					(CurrencyId::BTC, Balance::max_value()),
					(CurrencyId::ETH, 1_000 * DOLLARS)
				]
			),
			Error::<Test>::NumOverflow
		);
	});
}

#[test]
fn sort_by_balance_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		assert_eq!(
			TestLiquidationPools::sort_by_balance(vec![
				(CurrencyId::DOT, 4_000 * DOLLARS),
				(CurrencyId::ETH, 12_000 * DOLLARS),
				(CurrencyId::KSM, 6_000 * DOLLARS)
			]),
			Ok(vec![
				(CurrencyId::ETH, 12_000 * DOLLARS),
				(CurrencyId::KSM, 6_000 * DOLLARS),
				(CurrencyId::DOT, 4_000 * DOLLARS)
			])
		);
	});
}
