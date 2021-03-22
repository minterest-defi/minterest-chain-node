//  Integration-tests for minterest model pallet.

#[cfg(test)]

mod tests {
	use crate::tests::*;

	// Function `calculate_borrow_interest_rate + calculate_utilization_rate` scenario #1:
	#[test]
	fn calculate_borrow_interest_rate_deposit() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				// Alice deposit 40 DOT in pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				// utilization_rate = 0 / (40_000 - 0 + 0) = 0 < 0.8
				// borrow_rate = 0 * 0.000_000_009 + 0 = 0
				let (borrow_rate, _) =
					TestController::get_liquidity_pool_borrow_and_supply_rates(CurrencyId::DOT).unwrap_or_default();

				// Checking if real borrow interest rate is equal to the expected
				assert_eq!(Rate::zero(), borrow_rate);
			});
	}

	// Function `calculate_borrow_interest_rate + calculate_utilization_rate` scenario #2:
	#[test]
	fn calculate_borrow_interest_rate_deposit_and_borrow() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				// utilization_rate = 20_000 / (20_000 - 0 + 20_000) = 0.5 < kink = 0.8
				// borrow_rate = 0.5 * 0.000_000_009 + 0 = 45 * 10^(-10)
				let expected_borrow_rate_mock = Rate::saturating_from_rational(45_u128, 10_000_000_000_u128);

				let (borrow_rate, _) =
					TestController::get_liquidity_pool_borrow_and_supply_rates(CurrencyId::DOT).unwrap_or_default();

				// Checking if real borrow interest rate is equal to the expected
				assert_eq!(expected_borrow_rate_mock, borrow_rate);
			});
	}

	// Function `calculate_borrow_interest_rate + calculate_utilization_rate` scenario #3:
	#[test]
	fn calculate_borrow_interest_rate_few_deposits_and_borrows() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, BALANCE_ZERO, RATE_ZERO, true, 0)
			.pool_user_data(CurrencyId::DOT, BOB, BALANCE_ZERO, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				System::set_block_number(3);

				// Bob deposit to DOT pool
				let bob_deposited_amount = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_deposited_amount
				));

				System::set_block_number(4);

				// Alice try to borrow from DOT pool
				let bob_borrowed_amount_in_dot = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_borrowed_amount_in_dot
				));

				// utilization_rate = 70_000 / (130_000 - 100_000 + 70_000) = 0.7 < kink = 0.8
				// borrow_rate = 0.7 * 0.000_000_009 + 0 = 63 * 10^(-10) + accumulated_borrow
				let expected_borrow_rate_mock = Rate::from_inner(6_300_000_004);

				// Checking if real borrow interest rate is equal to the expected
				let (borrow_rate, _) =
					TestController::get_liquidity_pool_borrow_and_supply_rates(CurrencyId::DOT).unwrap_or_default();

				// Checking if real borrow interest rate is equal to the expected
				assert_eq!(expected_borrow_rate_mock, borrow_rate);
			});
	}
}
