//! Autogenerated weights for minterest_model
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-03-15, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/minterest
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=minterest_model
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --output=./runtime/src/weights/minterest_model.rs

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for minterest_model.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> minterest_model::WeightInfo for WeightInfo<T> {
	fn set_jump_multiplier_per_block() -> Weight {
		(34_034_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn set_base_rate_per_block() -> Weight {
		(33_703_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn set_multiplier_per_block() -> Weight {
		(33_618_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn set_kink() -> Weight {
		(32_907_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
}