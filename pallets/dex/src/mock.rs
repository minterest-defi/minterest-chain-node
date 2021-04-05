//! Mocks for dex module.

#![cfg(test)]

use super::*;
use crate as dex;
use frame_support::{construct_runtime, ord_parameter_types, parameter_types};
use frame_system as system;
use frame_system::offchain::SendTransactionTypes;
use frame_system::EnsureSignedBy;
pub(crate) use minterest_primitives::{Balance, CurrencyId, CurrencyPair, Price, Rate};
use orml_traits::parameter_type_with_key;
pub(crate) use pallet_traits::{PoolsManager, PriceProvider};
use sp_core::H256;
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
	FixedPointNumber
};
use helper::{
	mock_impl_system_config,
	mock_impl_orml_tokens_config,
	mock_impl_liquidity_pools_config,
};

pub type AccountId = u64;
type Amount = i128;

mock_impl_system_config!(Runtime);
mock_impl_orml_tokens_config!(Runtime);

parameter_types! {
	pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/lqdy");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsModuleId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
	pub EnabledCurrencyPair: Vec<CurrencyPair> = vec![
		CurrencyPair::new(CurrencyId::DOT, CurrencyId::MDOT),
		CurrencyPair::new(CurrencyId::KSM, CurrencyId::MKSM),
		CurrencyPair::new(CurrencyId::BTC, CurrencyId::MBTC),
		CurrencyPair::new(CurrencyId::ETH, CurrencyId::METH),
	];
	pub EnabledUnderlyingAssetId: Vec<CurrencyId> = EnabledCurrencyPair::get().iter()
			.map(|currency_pair| currency_pair.underlying_id)
			.collect();
	pub EnabledWrappedTokensId: Vec<CurrencyId> = EnabledCurrencyPair::get().iter()
			.map(|currency_pair| currency_pair.wrapped_id)
			.collect();
}

pub struct MockPriceSource;

impl PriceProvider<CurrencyId> for MockPriceSource {
	fn get_underlying_price(_currency_id: CurrencyId) -> Option<Price> {
		Some(Price::one())
	}

	fn lock_price(_currency_id: CurrencyId) {}

	fn unlock_price(_currency_id: CurrencyId) {}
}

mock_impl_liquidity_pools_config!(Runtime);

ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
}

parameter_types! {
	pub const LiquidationPoolsModuleId: ModuleId = ModuleId(*b"min/lqdn");
	pub LiquidationPoolAccountId: AccountId = LiquidationPoolsModuleId::get().into_account();
	pub const LiquidityPoolsPriority: TransactionPriority = TransactionPriority::max_value() - 1;
}

impl liquidation_pools::Config for Runtime {
	type Event = Event;
	type UnsignedPriority = LiquidityPoolsPriority;
	type LiquidationPoolsModuleId = LiquidationPoolsModuleId;
	type LiquidationPoolAccountId = LiquidationPoolAccountId;
	type LiquidityPoolsManager = liquidity_pools::Module<Runtime>;
	type UpdateOrigin = EnsureSignedBy<ZeroAdmin, AccountId>;
	type Dex = dex::Module<Runtime>;
	type LiquidationPoolsWeightInfo = ();
}

parameter_types! {
	pub const DexModuleId: ModuleId = ModuleId(*b"min/dexs");
	pub DexAccountId: AccountId = DexModuleId::get().into_account();
}

impl dex::Config for Runtime {
	type Event = Event;
	type MultiCurrency = orml_tokens::Module<Runtime>;
	type DexModuleId = DexModuleId;
	type DexAccountId = DexAccountId;
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Event<T>},
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		TestPools: liquidity_pools::{Module, Storage, Call, Config<T>},
		LiquidationPools: liquidation_pools::{Module, Storage, Call, Event<T>, Config<T>, ValidateUnsigned},
		TestDex: dex::{Module, Storage, Call, Event<T>},
	}
);

/// An extrinsic type used for tests.
pub type Extrinsic = TestXt<Call, ()>;

impl<LocalCall> SendTransactionTypes<LocalCall> for Runtime
where
	Call: From<LocalCall>,
{
	type OverarchingCall = Call;
	type Extrinsic = Extrinsic;
}

pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
pub fn dollars<T: Into<u128>>(d: T) -> Balance {
	DOLLARS.saturating_mul(d.into())
}

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
		}
	}
}

impl ExtBuilder {
	pub fn _liquidation_pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((LiquidationPools::pools_account_id(), currency_id, balance));
		self
	}

	pub fn dex_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestDex::dex_account_id(), currency_id, balance));
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

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
