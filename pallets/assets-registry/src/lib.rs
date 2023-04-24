#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod traits;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use crate::traits::{Asset, AssetRegistryReader};
	use frame_support::{inherent::Vec, pallet_prelude::*, traits::tokens::Balance, Twox64Concat};
	use frame_system::{ensure_root, pallet_prelude::OriginFor};
	use xcm::VersionedMultiLocation;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub(super) type AssetsMap<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::RegisteredAssetId,
		Asset<T::RegisteredAssetId, T::Balance>,
		OptionQuery,
	>;

	#[pallet::storage]
	pub(super) type ExistentialDeposits<T: Config> =
		StorageMap<_, Twox64Concat, T::RegisteredAssetId, T::Balance, OptionQuery>;

	#[pallet::storage]
	pub(super) type LocationToAssetId<T: Config> =
		StorageMap<_, Twox64Concat, VersionedMultiLocation, T::RegisteredAssetId, OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// Asset has already been registered.
		AssetAlreadyRegistered,
		/// Asset does not exist.
		AssetDoesNotExist,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New asset has been registered.
		AssetRegistered(T::RegisteredAssetId),
		/// Asset has been updated.
		AssetUpdated(T::RegisteredAssetId),
		/// Asset has been deleted.
		AssetDeleted(T::RegisteredAssetId),
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type RegisteredAssetId: Parameter
			+ Member
			+ Copy
			+ MaybeSerializeDeserialize
			+ Ord
			+ Clone
			+ Default;

		type Balance: Balance;
	}

	// Pallet internal functions
	impl<T: Config> Pallet<T> {
		pub fn get_value() -> Option<u32> {
			Some(123_u32)
		}

		pub fn get_asset_by_location(
			location: VersionedMultiLocation,
		) -> Option<Asset<T::RegisteredAssetId, T::Balance>> {
			let asset_id = LocationToAssetId::<T>::get(location)?;
			AssetsMap::<T>::get(asset_id)
		}

		pub fn get_assets_names() -> Vec<Vec<u8>> {
			AssetsMap::<T>::iter().map(|(_, asset)| asset.name).collect()
		}

		/// Checks if asset is registered.
		pub fn is_asset_registered(asset_id: &T::RegisteredAssetId) -> bool {
			AssetsMap::<T>::contains_key(asset_id)
		}

		/// Registers asset.
		pub fn do_register_asset(
			asset: Asset<T::RegisteredAssetId, T::Balance>,
		) -> DispatchResultWithPostInfo {
			let asset_id = asset.asset_id;
			ensure!(!Self::is_asset_registered(&asset_id), Error::<T>::AssetAlreadyRegistered);
			ExistentialDeposits::<T>::insert(asset_id, &asset.existential_deposit);
			AssetsMap::<T>::insert(asset_id, asset.clone());
			if let Some(location) = asset.location {
				LocationToAssetId::<T>::insert(location, asset_id);
			}

			Self::deposit_event(Event::AssetRegistered(asset_id));

			Ok(().into())
		}

		/// Update asset.
		pub fn do_update_asset(
			asset: Asset<T::RegisteredAssetId, T::Balance>,
		) -> DispatchResultWithPostInfo {
			let asset_id = asset.asset_id;
			ensure!(Self::is_asset_registered(&asset_id), Error::<T>::AssetDoesNotExist);
			ExistentialDeposits::<T>::insert(asset_id, &asset.existential_deposit);
			let old_asset =
				AssetsMap::<T>::get(asset_id).expect("Asset must exist for previous check");
			AssetsMap::<T>::insert(asset_id, asset.clone());
			if let Some(location) = asset.clone().location {
				if let Some(old_location) = old_asset.location {
					LocationToAssetId::<T>::remove(old_location);
				}
				LocationToAssetId::<T>::insert(location, asset_id);
			}

			Self::deposit_event(Event::AssetUpdated(asset_id));

			Ok(().into())
		}

		/// Delete asset.
		pub fn do_delete_asset(asset_id: T::RegisteredAssetId) -> DispatchResultWithPostInfo {
			ensure!(Self::is_asset_registered(&asset_id), Error::<T>::AssetDoesNotExist);
			ExistentialDeposits::<T>::remove(asset_id);
			AssetsMap::<T>::remove(asset_id);

			Self::deposit_event(Event::AssetDeleted(asset_id));

			Ok(().into())
		}
	}

	impl<T: Config> AssetRegistryReader<T::RegisteredAssetId, T::Balance> for Pallet<T> {
		fn get_asset(
			asset_id: T::RegisteredAssetId,
		) -> Option<Asset<T::RegisteredAssetId, T::Balance>> {
			AssetsMap::<T>::get(asset_id)
		}

		fn get_asset_name(asset_id: T::RegisteredAssetId) -> Option<Vec<u8>> {
			AssetsMap::<T>::get(asset_id).map(|asset| asset.name)
		}

		fn get_asset_decimals(asset_id: T::RegisteredAssetId) -> Option<u8> {
			AssetsMap::<T>::get(asset_id).map(|asset| asset.decimals)
		}

		fn get_asset_existential_deposit(asset_id: T::RegisteredAssetId) -> Option<T::Balance> {
			ExistentialDeposits::<T>::get(asset_id)
		}
	}

	// Pallet extrinsics
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		#[pallet::call_index(0)]
		pub fn register_asset(
			origin: OriginFor<T>,
			asset: Asset<T::RegisteredAssetId, T::Balance>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::do_register_asset(asset)
		}

		#[pallet::weight(0)]
		#[pallet::call_index(1)]
		pub fn update_asset(
			origin: OriginFor<T>,
			asset: Asset<T::RegisteredAssetId, T::Balance>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::do_update_asset(asset)
		}

		#[pallet::weight(0)]
		#[pallet::call_index(2)]
		pub fn delete_asset(
			origin: OriginFor<T>,
			asset_id: T::RegisteredAssetId,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::do_delete_asset(asset_id)
		}
	}
}
