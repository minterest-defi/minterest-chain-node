#![cfg(test)]

use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use sp_core::H256;
use sp_runtime::{
testing::Header, Perbill, traits::{IdentityLookup},
};
use orml_currencies::Currency;
pub use minterest_primitives::{Balance, CurrencyId};

use super::*;

pub use crate as liquidity_pool;

mod liquidity_pools {
    pub use crate::Event;
}

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

impl_outer_event! {
    pub enum TestEvent for Runtime {
        frame_system<T>,
        orml_currencies<T>, orml_tokens<T>,
        liquidity_pools,
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Runtime`) which `impl`s each of the
// configuration traits of modules we want to use.
parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

impl frame_system::Trait for Runtime {
    type BaseCallFilter = ();
    type Origin = Origin;
    type Call = ();
    type Index = u64;
    type BlockNumber = u64;
    // type Hash = Hash;
    type Hash = H256;
    type Hashing = ::sp_runtime::traits::BlakeTwo256;
    type AccountId = u32;
    // type Lookup = IdentityLookup<AccountId>;
    type Lookup = IdentityLookup<Self::AccountId>;
    // type Header = generic::Header<BlockNumber, BlakeTwo256>;
    type Header = Header;
    type Event = TestEvent;
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    // type DbWeight = RocksDbWeight;
    type DbWeight = ();
    // type BlockExecutionWeight = BlockExecutionWeight;
    type BlockExecutionWeight = ();
    // type ExtrinsicBaseWeight = ExtrinsicBaseWeight;
    type ExtrinsicBaseWeight = ();
    type MaximumExtrinsicWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    // type Version = Version;
    type Version = ();
    // type PalletInfo = PalletInfo;
    type PalletInfo = ();
    // type AccountData = pallet_balances::AccountData<Balance>;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
}

pub type System = frame_system::Module<Runtime>;

type Amount = i128;
impl orml_tokens::Trait for Runtime {
    type Event = TestEvent;
    type Balance = Balance;
    type Amount = Amount;
    type CurrencyId = CurrencyId;
    type OnReceived = ();
    type WeightInfo = ();
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::MINT;
}

type NativeCurrency = Currency<Runtime, GetNativeCurrencyId>;

impl orml_currencies::Trait for Runtime {
    type Event = TestEvent;
    // type MultiCurrency = Tokens;
    type MultiCurrency = orml_tokens::Module<Runtime>;
    type NativeCurrency = NativeCurrency;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type WeightInfo = ();
}

impl Trait for Runtime {
    type Event = TestEvent;
    type MultiCurrency = orml_currencies::Module<Runtime>;
}

pub type LiquidityPool = Module<Runtime>;

pub struct ExtBuilder {}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {}
    }
}

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        liquidity_pool::GenesisConfig {
            pools: vec![
                (CurrencyId::ETH, 0),
                (CurrencyId::DOT, 0),
                (CurrencyId::KSM, 0),
                (CurrencyId::BTC, 0),],
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}