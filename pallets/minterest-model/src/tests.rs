//! Tests for the minterest-model pallet.

use super::*;
use mock::{Event, *};

use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError::BadOrigin;

fn multiplier_per_block_equal_max_value() -> MinterestModelData {
	MinterestModelData {
		kink: Rate::saturating_from_rational(12, 10),
		base_rate_per_block: Rate::from_inner(0),
		multiplier_per_block: Rate::from_inner(u128::MAX),
		jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
	}
}

fn base_rate_per_block_equal_max_value() -> MinterestModelData {
	MinterestModelData {
		kink: Rate::saturating_from_rational(12, 10),
		base_rate_per_block: Rate::from_inner(u128::MAX),
		multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
		jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
	}
}

#[test]
fn set_base_rate_per_year_should_work() {
	new_test_ext().execute_with(|| {
		// Set Base rate per block equal 2.0: (10_512_000 / 1) / 5_256_000
		assert_ok!(TestMinterestModel::set_base_rate_per_year(
			alice(),
			CurrencyId::DOT,
			Rate::saturating_from_rational(10_512_000, 1)
		));
		assert_eq!(
			TestMinterestModel::minterest_model_dates(CurrencyId::DOT).base_rate_per_block,
			Rate::saturating_from_rational(2, 1)
		);
		let expected_event = Event::minterest_model(crate::Event::BaseRatePerBlockHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to 0.0: (0 / 10) / 5_256_000
		assert_ok!(TestMinterestModel::set_base_rate_per_year(
			alice(),
			CurrencyId::DOT,
			Rate::zero()
		));
		assert_eq!(
			TestMinterestModel::minterest_model_dates(CurrencyId::DOT).base_rate_per_block,
			Rate::zero()
		);

		// ALICE set Baser rate per block equal 0,000000009: (47_304 / 1_000_000) / 5_256_000
		assert_ok!(TestMinterestModel::set_base_rate_per_year(
			alice(),
			CurrencyId::DOT,
			Rate::saturating_from_rational(47304, 1_000_000)
		));
		assert_eq!(
			TestMinterestModel::minterest_model_dates(CurrencyId::DOT).base_rate_per_block,
			Rate::from_inner(9_000_000_000)
		);

		// Base rate per block cannot be set to 0 at the same time as Multiplier per block.
		assert_ok!(TestMinterestModel::set_multiplier_per_year(
			alice(),
			CurrencyId::DOT,
			Rate::zero()
		));
		assert_noop!(
			TestMinterestModel::set_base_rate_per_year(alice(), CurrencyId::DOT, Rate::zero()),
			Error::<Test>::BaseRatePerBlockCannotBeZero
		);

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			TestMinterestModel::set_base_rate_per_year(bob(), CurrencyId::DOT, Rate::from_inner(2)),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestMinterestModel::set_base_rate_per_year(alice(), CurrencyId::MDOT, Rate::from_inner(2)),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_multiplier_per_year_should_work() {
	new_test_ext().execute_with(|| {
		// Set Multiplier per block equal 2.0: (10_512_000 / 1) / 5_256_000
		assert_ok!(TestMinterestModel::set_multiplier_per_year(
			alice(),
			CurrencyId::DOT,
			Rate::saturating_from_rational(10_512_000, 1)
		));
		assert_eq!(
			TestMinterestModel::minterest_model_dates(CurrencyId::DOT).multiplier_per_block,
			Rate::saturating_from_rational(2, 1)
		);
		let expected_event = Event::minterest_model(crate::Event::MultiplierPerBlockHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to 0.0 if Base rate per block grater than zero: (0 / 10) / 5_256_000
		assert_ok!(TestMinterestModel::set_base_rate_per_year(
			alice(),
			CurrencyId::DOT,
			Rate::one()
		));
		assert_ok!(TestMinterestModel::set_multiplier_per_year(
			alice(),
			CurrencyId::DOT,
			Rate::zero()
		));
		assert_eq!(
			TestMinterestModel::minterest_model_dates(CurrencyId::DOT).multiplier_per_block,
			Rate::zero()
		);

		// Alice set Multiplier per block equal 0,000_000_009: (47_304 / 1_000_000) / 5_256_000
		assert_ok!(TestMinterestModel::set_multiplier_per_year(
			alice(),
			CurrencyId::DOT,
			Rate::saturating_from_rational(47304, 1_000_000)
		));
		assert_eq!(
			TestMinterestModel::minterest_model_dates(CurrencyId::DOT).multiplier_per_block,
			Rate::from_inner(9_000_000_000)
		);

		//  Multiplier per block cannot be set to 0 at the same time as Base rate per block.
		assert_ok!(TestMinterestModel::set_base_rate_per_year(
			alice(),
			CurrencyId::DOT,
			Rate::zero()
		));
		assert_noop!(
			TestMinterestModel::set_multiplier_per_year(alice(), CurrencyId::DOT, Rate::zero()),
			Error::<Test>::MultiplierPerBlockCannotBeZero
		);

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			TestMinterestModel::set_multiplier_per_year(bob(), CurrencyId::DOT, Rate::from_inner(2)),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestMinterestModel::set_base_rate_per_year(alice(), CurrencyId::MDOT, Rate::from_inner(2)),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_jump_multiplier_per_year_should_work() {
	new_test_ext().execute_with(|| {
		// Set Jump multiplier per block equal 2.0: (10_512_000 / 1) / 5_256_000
		assert_ok!(TestMinterestModel::set_jump_multiplier_per_year(
			alice(),
			CurrencyId::DOT,
			Rate::saturating_from_rational(10_512_000, 1)
		));
		assert_eq!(
			TestMinterestModel::minterest_model_dates(CurrencyId::DOT).jump_multiplier_per_block,
			Rate::saturating_from_rational(2, 1)
		);
		let expected_event = Event::minterest_model(crate::Event::JumpMultiplierPerBlockHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to 0.0: (0 / 10) / 5_256_000
		assert_ok!(TestMinterestModel::set_jump_multiplier_per_year(
			alice(),
			CurrencyId::DOT,
			Rate::zero()
		));
		assert_eq!(
			TestMinterestModel::minterest_model_dates(CurrencyId::DOT).jump_multiplier_per_block,
			Rate::zero()
		);

		// Alice set Jump multiplier per block equal 0,000_000_009: (47_304 / 1_000_000) / 5_256_000
		assert_ok!(TestMinterestModel::set_jump_multiplier_per_year(
			alice(),
			CurrencyId::DOT,
			Rate::saturating_from_rational(47_304, 1_000_000)
		));
		assert_eq!(
			TestMinterestModel::minterest_model_dates(CurrencyId::DOT).jump_multiplier_per_block,
			Rate::from_inner(9_000_000_000)
		);

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			TestMinterestModel::set_jump_multiplier_per_year(bob(), CurrencyId::DOT, Rate::from_inner(2)),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestMinterestModel::set_base_rate_per_year(alice(), CurrencyId::MDOT, Rate::from_inner(2)),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_kink_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(TestMinterestModel::set_kink(
			alice(),
			CurrencyId::DOT,
			Rate::saturating_from_rational(8, 10)
		));
		assert_eq!(
			TestMinterestModel::minterest_model_dates(CurrencyId::DOT).kink,
			Rate::saturating_from_rational(8, 10)
		);
		let expected_event = Event::minterest_model(crate::Event::KinkHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			TestMinterestModel::set_kink(bob(), CurrencyId::DOT, Rate::saturating_from_rational(8, 10)),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestMinterestModel::set_kink(alice(), CurrencyId::MDOT, Rate::saturating_from_rational(8, 10)),
			Error::<Test>::NotValidUnderlyingAssetId
		);

		// Parameter `kink` cannot be more than one.
		assert_noop!(
			TestMinterestModel::set_kink(alice(), CurrencyId::DOT, Rate::saturating_from_rational(11, 10)),
			Error::<Test>::KinkCannotBeMoreThanOne
		);
	});
}

#[test]
fn calculate_borrow_interest_rate_should_work() {
	new_test_ext().execute_with(|| {
		// Utilization rate less or equal than kink:
		// utilization_rate = 0.42
		// borrow_interest_rate = 0,42 * multiplier_per_block + base_rate_per_block
		assert_eq!(
			TestMinterestModel::calculate_borrow_interest_rate(
				CurrencyId::DOT,
				Rate::saturating_from_rational(42, 100)
			),
			Ok(Rate::from_inner(3_780_000_000))
		);

		// Utilization rate larger than kink:
		// utilization_rate = 0.9
		// borrow_interest_rate = 0.9 * 0.8 * jump_multiplier_per_block +
		// + (0.8 * multiplier_per_block) + base_rate_per_block
		assert_eq!(
			TestMinterestModel::calculate_borrow_interest_rate(CurrencyId::DOT, Rate::saturating_from_rational(9, 10)),
			Ok(Rate::from_inner(156_240_000_000))
		);
	});
}

#[test]
fn calculate_borrow_interest_rate_fails_if_overflow_kink_mul_multiplier() {
	new_test_ext().execute_with(|| {
		let minterest_model_data = multiplier_per_block_equal_max_value();
		<MinterestModelParams<Test>>::insert(CurrencyId::KSM, minterest_model_data.clone());
		// utilization_rate > kink.
		// Overflow in calculation: kink * multiplier_per_block = 1.01 * max_value()
		assert_noop!(
			TestMinterestModel::calculate_borrow_interest_rate(
				CurrencyId::KSM,
				Rate::saturating_from_rational(101, 100)
			),
			Error::<Test>::BorrowRateCalculationError
		);
	});
}

#[test]
fn calculate_borrow_interest_rate_fails_if_overflow_add_base_rate_per_block() {
	new_test_ext().execute_with(|| {
		let minterest_model_data = base_rate_per_block_equal_max_value();
		<MinterestModelParams<Test>>::insert(CurrencyId::KSM, minterest_model_data.clone());
		// utilization_rate > kink.
		// Overflow in calculation: kink_mul_multiplier + base_rate_per_block = ... + max_value()
		assert_noop!(
			TestMinterestModel::calculate_borrow_interest_rate(CurrencyId::KSM, Rate::saturating_from_rational(9, 10)),
			Error::<Test>::BorrowRateCalculationError
		);
	});
}
