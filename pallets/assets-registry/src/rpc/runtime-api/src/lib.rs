#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::inherent::Vec;

sp_api::decl_runtime_apis! {
	#[api_version(2)]
	pub trait AssetsRegistryApi {
		fn get_value() -> u32;

		fn get_assets_names() -> Vec<Vec<u8>>;
	}
}
