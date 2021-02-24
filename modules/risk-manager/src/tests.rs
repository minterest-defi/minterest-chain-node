//! Tests for the risk-manager pallet.

use super::*;
use mock::{Event, *};

use frame_support::{assert_noop, assert_ok};
use sp_arithmetic::traits::Bounded;
use sp_runtime::{traits::BadOrigin, FixedPointNumber};

#[test]
fn set_max_attempts_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_max_attempts(admin(), CurrencyId::DOT, 0));
		assert_eq!(TestRiskManager::risk_manager_dates(CurrencyId::DOT).max_attempts, 0);
		let expected_event = Event::risk_manager(crate::RawEvent::MaxValueOFLiquidationAttempsHasChanged(ADMIN, 0));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set max_attempts equal 2.0
		assert_ok!(TestRiskManager::set_max_attempts(admin(), CurrencyId::DOT, 2));
		assert_eq!(TestRiskManager::risk_manager_dates(CurrencyId::DOT).max_attempts, 2);
		let expected_event = Event::risk_manager(crate::RawEvent::MaxValueOFLiquidationAttempsHasChanged(ADMIN, 2));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_max_attempts(alice(), CurrencyId::DOT, 10),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_max_attempts(admin(), CurrencyId::MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_min_sum_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_min_sum(admin(), CurrencyId::DOT, Balance::zero()));
		assert_eq!(
			TestRiskManager::risk_manager_dates(CurrencyId::DOT).min_sum,
			Balance::zero()
		);
		let expected_event = Event::risk_manager(crate::RawEvent::MinSumForPartialLiquidationHasChanged(
			ADMIN,
			Balance::zero(),
		));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set min_sum equal one hundred.
		assert_ok!(TestRiskManager::set_min_sum(
			admin(),
			CurrencyId::DOT,
			ONE_HUNDRED * DOLLARS
		));
		assert_eq!(
			TestRiskManager::risk_manager_dates(CurrencyId::DOT).min_sum,
			ONE_HUNDRED * DOLLARS
		);
		let expected_event = Event::risk_manager(crate::RawEvent::MinSumForPartialLiquidationHasChanged(
			ADMIN,
			ONE_HUNDRED * DOLLARS,
		));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_min_sum(alice(), CurrencyId::DOT, 10),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_min_sum(admin(), CurrencyId::MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_threshold_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_threshold(admin(), CurrencyId::DOT, 0, 1));
		assert_eq!(
			TestRiskManager::risk_manager_dates(CurrencyId::DOT).threshold,
			Rate::zero()
		);
		let expected_event = Event::risk_manager(crate::RawEvent::ValueOfThresholdHasChanged(ADMIN, Rate::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set min_sum equal one hundred.
		assert_ok!(TestRiskManager::set_threshold(admin(), CurrencyId::DOT, 1, 1));
		assert_eq!(
			TestRiskManager::risk_manager_dates(CurrencyId::DOT).threshold,
			Rate::one()
		);
		let expected_event = Event::risk_manager(crate::RawEvent::ValueOfThresholdHasChanged(ADMIN, Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_threshold(alice(), CurrencyId::DOT, 1, 1),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_threshold(admin(), CurrencyId::MDOT, 1, 1),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_liquidation_incentive_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 1.0
		assert_ok!(TestRiskManager::set_liquidation_incentive(
			admin(),
			CurrencyId::DOT,
			1,
			1
		));
		assert_eq!(
			TestRiskManager::risk_manager_dates(CurrencyId::DOT).liquidation_incentive,
			Rate::one()
		);
		let expected_event = Event::risk_manager(crate::RawEvent::ValueOfLiquidationFeeHasChanged(ADMIN, Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can not be set to 0.0
		assert_noop!(
			TestRiskManager::set_liquidation_incentive(admin(), CurrencyId::DOT, 0, 1),
			Error::<Test>::InvalidLiquidationIncentiveValue
		);

		// Can not be set to 2.0
		assert_noop!(
			TestRiskManager::set_liquidation_incentive(admin(), CurrencyId::DOT, 2, 1),
			Error::<Test>::InvalidLiquidationIncentiveValue
		);

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_liquidation_incentive(alice(), CurrencyId::DOT, 1, 1),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_liquidation_incentive(admin(), CurrencyId::MDOT, 1, 1),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn liquidate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Origin::signed(Alice) is wrong origin for fn liquidate.
		assert_noop!(
			TestRiskManager::liquidate(Origin::signed(ALICE), ALICE, CurrencyId::DOT),
			BadOrigin
		);

		// Origin::none is available origin for fn liquidate.
		assert_ok!(TestRiskManager::liquidate(Origin::none(), ALICE, CurrencyId::DOT));
	})
}

#[test]
fn get_user_borrow_information_should_work() {
	ExtBuilder::default()
		.pool_initial(CurrencyId::DOT)
		.pool_user_data(CurrencyId::DOT, ALICE, ONE_HUNDRED * DOLLARS, Rate::one(), true, 0)
		.build()
		.execute_with(|| {
			assert_ok!(TestRiskManager::get_user_borrow_information(&ALICE, CurrencyId::DOT));
			assert_eq!(
				TestRiskManager::get_user_borrow_information(&ALICE, CurrencyId::DOT),
				Ok((
					2 * ONE_HUNDRED * DOLLARS,
					ONE_HUNDRED * DOLLARS,
					Rate::from_inner(2 * DOLLARS),
					0
				))
			);
		})
}

#[test]
fn get_user_borrow_information_should_not_work() {
	ExtBuilder::default()
		.pool_initial(CurrencyId::DOT)
		.pool_user_data(CurrencyId::DOT, ALICE, Balance::max_value(), Rate::one(), true, 0)
		.build()
		.execute_with(|| {
			assert_noop!(
				TestRiskManager::get_user_borrow_information(&ALICE, CurrencyId::DOT),
				Error::<Test>::NumOverflow
			);
		})
}

#[test]
fn mul_balance_by_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// 987999 * 1 = 987999
		assert_eq!(
			TestRiskManager::mul_balance_by_rate(&987_999, &Rate::one()),
			Ok(987_999)
		);

		// 100 * 0 = 0
		assert_eq!(
			TestRiskManager::mul_balance_by_rate(&ONE_HUNDRED, &Rate::zero()),
			Ok(Balance::zero())
		);

		// Balance::max_value() * 1,1 = NumOverflow
		assert_noop!(
			TestRiskManager::mul_balance_by_rate(&Balance::max_value(), &Rate::saturating_from_rational(11, 10)),
			Error::<Test>::NumOverflow
		);

		// (100 * 10^18) * Rate::max_value() = NumOverflow
		assert_noop!(
			TestRiskManager::mul_balance_by_rate(&(ONE_HUNDRED * DOLLARS), &Rate::max_value()),
			Error::<Test>::NumOverflow
		);
	})
}

#[test]
fn div_balance_by_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// 340_000 / 2 = 170_000
		assert_eq!(
			TestRiskManager::div_balance_by_rate(&340_000, &Rate::saturating_from_rational(2, 1)),
			Ok(170_000)
		);

		// 0 / 1 = 0
		assert_eq!(
			TestRiskManager::div_balance_by_rate(&Balance::zero(), &Rate::one()),
			Ok(Balance::zero())
		);

		// Balance::max_value() / 0.9 = NumOverflow
		assert_noop!(
			TestRiskManager::div_balance_by_rate(&Balance::max_value(), &Rate::saturating_from_rational(9, 10)),
			Error::<Test>::NumOverflow
		);

		// 100 / Rate::zero() = NumOverflow
		assert_noop!(
			TestRiskManager::div_balance_by_rate(&100, &Rate::zero()),
			Error::<Test>::NumOverflow
		);
	})
}

#[test]
fn sub_a_from_b_u128_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// 340_000 - 90_000 = 250_000
		assert_eq!(TestRiskManager::sub_a_from_b_u128(&340_000, &90_000), Ok(250_000));

		// 40 - 45 = NumOverflow
		assert_noop!(TestRiskManager::sub_a_from_b_u128(&40, &45), Error::<Test>::NumOverflow);
	})
}