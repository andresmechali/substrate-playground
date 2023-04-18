pub use crate::*;

use frame_support::{inherent::Vec, pallet_prelude::*};

#[derive(Encode, Decode, Default, PartialEq, Eq, TypeInfo, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Asset<RegisteredAssetId> {
	pub asset_id: RegisteredAssetId,
	pub decimals: u8,
	pub name: Vec<u8>,
}
