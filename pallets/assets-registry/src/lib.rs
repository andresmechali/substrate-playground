#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod traits;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use crate::traits::Asset;
	use frame_support::{pallet_prelude::*, Twox64Concat};
	use frame_system::{ensure_root, pallet_prelude::OriginFor};

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub(super) type AssetsMap<T: Config> =
		StorageMap<_, Twox64Concat, T::RegisteredAssetId, Asset<T::RegisteredAssetId>, OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// Asset has already been registered
		AssetAlreadyRegistered,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New asset has been registered
		AssetRegistered(T::RegisteredAssetId),
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
	}

	// Pallet internal functions
	impl<T: Config> Pallet<T> {
		/// Checks if asset is registered.
		pub fn is_asset_registered(asset_id: &T::RegisteredAssetId) -> bool {
			AssetsMap::<T>::contains_key(asset_id)
		}

		/// Registers asset.
		pub fn do_register_asset(asset: Asset<T::RegisteredAssetId>) -> DispatchResultWithPostInfo {
			let asset_id = asset.asset_id;
			ensure!(!Self::is_asset_registered(&asset_id), Error::<T>::AssetAlreadyRegistered);
			AssetsMap::<T>::insert(asset_id, asset);

			Self::deposit_event(Event::AssetRegistered(asset_id));

			Ok(().into())
		}
	}

	// Pallet extrinsics
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		#[pallet::call_index(0)]
		pub fn register_asset(
			origin: OriginFor<T>,
			asset: Asset<T::RegisteredAssetId>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::do_register_asset(asset)
		}
	}
}
