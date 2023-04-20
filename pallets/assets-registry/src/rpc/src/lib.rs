pub use assets_registry_runtime_api::AssetsRegistryApi as AssetsRegistryRuntimeApi;
use jsonrpsee::{
	core::{Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
// use sp_rpc::{list::ListOrValue, number::NumberOrHex};
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

#[rpc(client, server)]
pub trait AssetsRegistryApi<BlockHash> {
	#[method(name = "assetsRegistry_getValue")]
	fn get_value(&self, at: Option<BlockHash>) -> RpcResult<u32>;

	#[method(name = "assetsRegistry_getAssetsNames")]
	fn get_assets_names(&self, at: Option<BlockHash>) -> RpcResult<Vec<Vec<u8>>>;
}

pub struct AssetsRegistryPallet<C, Block> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<Block>,
}

impl<C, Block> AssetsRegistryPallet<C, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

impl<C, Block> AssetsRegistryApiServer<<Block as BlockT>::Hash> for AssetsRegistryPallet<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: AssetsRegistryRuntimeApi<Block>,
{
	fn get_value(&self, at: Option<<Block as BlockT>::Hash>) -> RpcResult<u32> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_value(&at).map_err(runtime_error_into_rpc_err)
	}

	fn get_assets_names(&self, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<Vec<u8>>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_assets_names(&at).map_err(runtime_error_into_rpc_err)
	}
}

const RUNTIME_ERROR: i32 = 1;

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> JsonRpseeError {
	CallError::Custom(ErrorObject::owned(
		RUNTIME_ERROR,
		"Runtime error",
		Some(format!("{:?}", err)),
	))
	.into()
}
