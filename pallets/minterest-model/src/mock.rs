/// Mocks for the minterest-model pallet.
use super::*;
use crate as minterest_model;
use frame_support::{ord_parameter_types, parameter_types};
use frame_system as system;
use frame_system::EnsureSignedBy;
use minterest_primitives::currency::CurrencyType::{UnderlyingAsset, WrappedToken};
use minterest_primitives::{Balance, CurrencyId, Price, Rate};
use orml_traits::parameter_type_with_key;
use pallet_traits::PriceProvider;
use sp_core::H256;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	ModuleId,
};
pub use test_helper::*;

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
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		TestMinterestModel: minterest_model::{Module, Storage, Call, Event, Config},
	}
);

ord_parameter_types! {
	pub const OneAlice: AccountId = 1;
}

mock_impl_system_config!(Test);
mock_impl_orml_tokens_config!(Test);
mock_impl_minterest_model_config!(Test, OneAlice);

parameter_types! {
	pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/lqdy");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsModuleId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
	pub EnabledUnderlyingAssetsIds: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset);
	pub EnabledWrappedTokensId: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(WrappedToken);
}

pub struct MockPriceSource;

impl PriceProvider<CurrencyId> for MockPriceSource {
	fn get_underlying_price(_currency_id: CurrencyId) -> Option<Price> {
		Some(Price::one())
	}

	fn lock_price(_currency_id: CurrencyId) {}

	fn unlock_price(_currency_id: CurrencyId) {}
}

pub const ALICE: AccountId = 1;
pub fn alice() -> Origin {
	Origin::signed(ALICE)
}
pub const BOB: AccountId = 2;
pub fn bob() -> Origin {
	Origin::signed(BOB)
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

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
				KSM,
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
	.assimilate_storage::<Test>(&mut t)
	.unwrap();

	let mut ext: sp_io::TestExternalities = t.into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}
