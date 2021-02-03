#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{debug, decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult};

pub trait Trait: frame_system::Trait {
	type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;
}

decl_storage! {
	trait Store for Module<T: Trait> as RiskManagerStorage {}
}

decl_event!(
	pub enum Event {}
);

decl_error! {
	pub enum Error for Module<T: Trait> {}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;


	}
}

impl<T: Trait> Module<T> {}
