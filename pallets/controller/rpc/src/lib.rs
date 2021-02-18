//! RPC interface for the controller module.

pub use controller_rpc_runtime_api::{ControllerApi as ControllerRuntimeApi, PoolState};
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use minterest_primitives::{AccountId, Balance, CurrencyId};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

#[rpc]
pub trait ControllerApi<BlockHash> {
	#[rpc(name = "controller_liquidityPoolState")]
	fn liquidity_pool_state(&self, pool_id: CurrencyId, at: Option<BlockHash>) -> Result<Option<PoolState>>;

	#[rpc(name = "controller_underlyingBalance")]
	fn get_underlying_balance(
		&self,
		account_id: AccountId,
		pool_id: CurrencyId,
		at: Option<BlockHash>,
	) -> Result<Option<Balance>>;

	#[rpc(name = "controller_borrowBalance")]
	fn get_borrow_balance(
		&self,
		account_id: AccountId,
		underlying_asset_id: CurrencyId,
		at: Option<BlockHash>,
	) -> Result<Option<Balance>>;
}

/// A struct that implements the [`ControllerApi`].
pub struct Controller<C, B> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> Controller<C, B> {
	/// Create new `LiquidityPool` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_marker: Default::default(),
		}
	}
}

pub enum Error {
	RuntimeError,
}

impl From<Error> for i64 {
	fn from(e: Error) -> i64 {
		match e {
			Error::RuntimeError => 1,
		}
	}
}

impl<C, Block> ControllerApi<<Block as BlockT>::Hash> for Controller<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: ControllerRuntimeApi<Block>,
{
	fn liquidity_pool_state(
		&self,
		pool_id: CurrencyId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Option<PoolState>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));
		api.liquidity_pool_state(&at, pool_id).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to get pool state.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}

	fn get_underlying_balance(
		&self,
		account_id: AccountId,
		pool_id: CurrencyId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Option<Balance>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));
		api.get_underlying_balance(&at, account_id, pool_id)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get underlying balance.".into(),
				data: Some(format!("{:?}", e).into()),
			})
	}

	fn get_borrow_balance(
		&self,
		account_id: AccountId,
		underlying_asset_id: CurrencyId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Option<Balance>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));
		api.get_borrow_balance(&at, account_id, underlying_asset_id)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get borrow balance.".into(),
				data: Some(format!("{:?}", e).into()),
			})
	}
}
