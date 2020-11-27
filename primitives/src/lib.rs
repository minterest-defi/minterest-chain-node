#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use sp_runtime::{
    RuntimeDebug, MultiSignature, generic,
    traits::{Verify, IdentifyAccount},
};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

/// Signed version of Balance
pub type Amount = i128;

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
    MINT = 0,
    DOT,
    KSM,
    BTC,
    ETH,
    MDOT,
    MKSM,
    MBTC,
    METH,
}
// #[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
// pub enum UnderlineAssets {
//     DOT,
//     DAI,
//     KSM,
// }
// #[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, PartialOrd, Ord)]
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
// pub struct Pool {
//     pub id: UnderlineAssets,
//     // pub symbol: Vec<u8>,
//     pub balance: Balance,
// }


