/// Mocks for the RiskManager pallet.
use frame_support::{impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types};
use minterest_primitives::{Balance, CurrencyId, CurrencyPair, Rate};
use orml_currencies::Currency;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, FixedPointNumber, ModuleId, Perbill};

use super::*;
use liquidity_pools::{Pool, PoolUserData};
use sp_runtime::testing::TestXt;

impl_outer_origin! {
	pub enum Origin for Test {}
}

mod risk_manager {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Test {
		frame_system<T>,
		orml_tokens<T>,
		orml_currencies<T>,
		accounts<T>,
		liquidity_pools,
		liquidation_pools,
		risk_manager<T>,
		controller,
		minterest_model,
		oracle,

	}
}

impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		risk_manager::TestRiskManager,
	}
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

type AccountId = u32;

impl frame_system::Trait for Test {
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type PalletInfo = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type AccountData = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl orml_tokens::Trait for Test {
	type Event = TestEvent;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type OnReceived = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const MaxMembers: u32 = MAX_MEMBERS;
}

impl accounts::Trait for Test {
	type Event = TestEvent;
	type MaxMembers = MaxMembers;
}

parameter_types! {
	pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/lqdy");
	pub const InitialExchangeRate: Rate = Rate::from_inner(1_000_000_000_000_000_000);
	pub EnabledCurrencyPair: Vec<CurrencyPair> = vec![
		CurrencyPair::new(CurrencyId::DOT, CurrencyId::MDOT),
		CurrencyPair::new(CurrencyId::KSM, CurrencyId::MKSM),
		CurrencyPair::new(CurrencyId::BTC, CurrencyId::MBTC),
		CurrencyPair::new(CurrencyId::ETH, CurrencyId::METH),
	];
}

impl liquidity_pools::Trait for Test {
	type Event = TestEvent;
	type MultiCurrency = orml_tokens::Module<Test>;
	type ModuleId = LiquidityPoolsModuleId;
	type InitialExchangeRate = InitialExchangeRate;
	type EnabledCurrencyPair = EnabledCurrencyPair;
}

impl controller::Trait for Test {
	type Event = TestEvent;
	type LiquidityPoolsManager = liquidity_pools::Module<Test>;
}

impl oracle::Trait for Test {
	type Event = TestEvent;
}

parameter_types! {
	pub const BlocksPerYear: u128 = BLOCKS_PER_YEAR;
}

impl minterest_model::Trait for Test {
	type Event = TestEvent;
	type BlocksPerYear = BlocksPerYear;
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::MINT;
}

type NativeCurrency = Currency<Test, GetNativeCurrencyId>;

impl orml_currencies::Trait for Test {
	type Event = TestEvent;
	type MultiCurrency = orml_tokens::Module<Test>;
	type NativeCurrency = NativeCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}

parameter_types! {
	pub const LiquidationPoolsModuleId: ModuleId = ModuleId(*b"min/lqdn");
}

impl liquidation_pools::Trait for Test {
	type Event = TestEvent;
	type ModuleId = LiquidationPoolsModuleId;
	type MultiCurrency = orml_tokens::Module<Test>;
}

pub struct LiquidationPoolsManager;
impl PoolsManager<AccountId> for LiquidationPoolsManager {
	fn pools_account_id() -> AccountId {
		MOCK_LIQUIDATION_POOL_ACCOUNT
	}

	fn get_pool_available_liquidity(pool_id: CurrencyId) -> Balance {
		Currencies::free_balance(pool_id, &MOCK_LIQUIDATION_POOL_ACCOUNT)
	}

	fn pool_exists(_underlying_asset_id: &CurrencyId) -> bool {
		unimplemented!()
	}
}
pub struct LiquidityPoolsManager;
impl PoolsManager<AccountId> for LiquidityPoolsManager {
	fn pools_account_id() -> AccountId {
		MOCK_LIQUIDITY_POOL_ACCOUNT
	}

	fn get_pool_available_liquidity(pool_id: CurrencyId) -> Balance {
		Currencies::free_balance(pool_id, &MOCK_LIQUIDITY_POOL_ACCOUNT)
	}

	fn pool_exists(_underlying_asset_id: &CurrencyId) -> bool {
		unimplemented!()
	}
}

parameter_types! {
	pub const RiskManagerPriority: TransactionPriority = TransactionPriority::max_value();
}

impl Trait for Test {
	type Event = TestEvent;
	type UnsignedPriority = RiskManagerPriority;
	type MultiCurrency = orml_tokens::Module<Test>;
	type LiquidationPoolsManager = LiquidationPoolsManager;
	type LiquidityPoolsManager = LiquidityPoolsManager;
}

/// An extrinsic type used for tests.
pub type Extrinsic = TestXt<Call, ()>;

impl<LocalCall> SendTransactionTypes<LocalCall> for Test
where
	Call: From<LocalCall>,
{
	type OverarchingCall = Call;
	type Extrinsic = Extrinsic;
}

type Amount = i128;

pub type TestRiskManager = Module<Test>;
pub type System = frame_system::Module<Test>;
pub type Currencies = orml_currencies::Module<Test>;
pub const BLOCKS_PER_YEAR: u128 = 5_256_000;
pub const MAX_MEMBERS: u32 = 16;
pub const ONE_HUNDRED: Balance = 100;
pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
pub const MOCK_LIQUIDITY_POOL_ACCOUNT: AccountId = 2;
pub const MOCK_LIQUIDATION_POOL_ACCOUNT: AccountId = 3;
pub const ADMIN: AccountId = 0;
pub fn admin() -> Origin {
	Origin::signed(ADMIN)
}
pub const ALICE: AccountId = 1;
pub fn alice() -> Origin {
	Origin::signed(ALICE)
}

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	pools: Vec<(CurrencyId, Pool)>,
	pool_user_data: Vec<(CurrencyId, AccountId, PoolUserData)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
			pools: vec![],
			pool_user_data: vec![],
		}
	}
}

impl ExtBuilder {
	pub fn liquidity_pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((LiquidityPoolsManager::pools_account_id(), currency_id, balance));
		self
	}

	pub fn liquidation_pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((LiquidationPoolsManager::pools_account_id(), currency_id, balance));
		self
	}

	pub fn pool_initial(mut self, pool_id: CurrencyId) -> Self {
		self.pools.push((
			pool_id,
			Pool {
				total_borrowed: Balance::zero(),
				borrow_index: Rate::saturating_from_rational(1, 1),
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
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		orml_tokens::GenesisConfig::<Test> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		liquidity_pools::GenesisConfig::<Test> {
			pools: self.pools,
			pool_user_data: self.pool_user_data,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		accounts::GenesisConfig::<Test> {
			allowed_accounts: vec![(ADMIN, ())],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		GenesisConfig {
			risk_manager_dates: vec![
				(
					CurrencyId::DOT,
					RiskManagerData {
						max_attempts: 3,
						min_sum: ONE_HUNDRED * DOLLARS,
						threshold: Rate::saturating_from_rational(103, 100),
						liquidation_fee: Rate::saturating_from_rational(105, 100),
					},
				),
				(
					CurrencyId::BTC,
					RiskManagerData {
						max_attempts: 3,
						min_sum: ONE_HUNDRED * DOLLARS,
						threshold: Rate::saturating_from_rational(103, 100),
						liquidation_fee: Rate::saturating_from_rational(105, 100),
					},
				),
				(
					CurrencyId::ETH,
					RiskManagerData {
						max_attempts: 3,
						min_sum: ONE_HUNDRED * DOLLARS,
						threshold: Rate::saturating_from_rational(103, 100),
						liquidation_fee: Rate::saturating_from_rational(105, 100),
					},
				),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
