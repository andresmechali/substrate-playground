pub use crate::*;

use frame_support::{inherent::Vec, pallet_prelude::*};
use xcm::VersionedMultiLocation;

#[derive(Encode, Decode, Default, PartialEq, Eq, TypeInfo, Clone, Debug)]
pub struct Asset<RegisteredAssetId, Balance> {
	pub asset_id: RegisteredAssetId,
	pub decimals: u8,
	pub name: Vec<u8>,
	pub existential_deposit: Balance,
	pub location: Option<VersionedMultiLocation>,
}

pub trait AssetRegistryReader<RegisteredAssetId, Balance> {
	fn get_asset(asset_id: RegisteredAssetId) -> Option<Asset<RegisteredAssetId, Balance>>;
	fn get_asset_name(asset_id: RegisteredAssetId) -> Option<Vec<u8>>;
	fn get_asset_decimals(asset_id: RegisteredAssetId) -> Option<u8>;
	fn get_asset_existential_deposit(asset_id: RegisteredAssetId) -> Option<Balance>;
}
