#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::pallet_prelude::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod secrets;

pub use pallet::*;

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub(crate) struct Secret {
	pub(crate) id: u64,
	// pub(crate) service: String,
	// pub(crate) username: String,
	// pub(crate) password: String,
	pub(crate) expiration_timestamp: u64,
}

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use crate::Secret;
	use chrono::Utc;
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, Randomness},
	};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub(super) type Nonce<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Maps the Secret struct to the unique_id.
	#[pallet::storage]
	pub(super) type SecretMap<T: Config> = StorageMap<_, Twox64Concat, u64, Secret>;

	#[pallet::storage]
	pub(super) type OwnerMap<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		T::AccountId,
		BoundedVec<u64, T::MaximumStored>,
	>;

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::error]
	pub enum Error<T> {
		/// Each secret must have a unique identifier
		DuplicateSecret,
		/// An account can't exceed the `MaximumStored` constant
		MaximumSecretsStored,
		/// The total secrets stored can't exceed the u64 limit
		BoundsOverflow,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new secret was successfully created
		SecretCreated { id: u64, owner: T::AccountId, to: T::AccountId, expiration_timestamp: u64 },
		/// A secret was successfully deleted
		SecretDeleted { id: u64 },
		/// A secret was successfully extended
		SecretExtended { id: u64, expiration_timestamp: u64 },
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type Currency: Currency<Self::AccountId>;

		#[pallet::constant]
		type MaximumStored: Get<u32>;
	}

	// Pallet internal functions
	impl<T: Config> Pallet<T> {
		// Generates and returns the unique_id
		fn gen_unique_id() -> u64 {
			let current_nonce = Nonce::<T>::get();
			let next_nonce = current_nonce.checked_add(1).expect("Should not overflow");
			Nonce::<T>::put(next_nonce);
			next_nonce
		}

		/// Creates new secret
		fn create_secret(
			owner: &T::AccountId,
			to: &T::AccountId,
			unique_id: u64,
			duration_ms: u64,
		) -> DispatchResult {
			let expiration_timestamp = Utc::now().timestamp() as u64 + duration_ms;
			let new_secret = Secret { id: unique_id, expiration_timestamp };
			SecretMap::<T>::insert(unique_id, new_secret.clone());
			// Try appending into the bounded vec, or create a new one
			OwnerMap::<T>::try_mutate(owner, to, |maybe_secrets| -> DispatchResult {
				if let Some(mut secrets) = maybe_secrets.clone() {
					secrets
						.try_push(new_secret.id)
						.map_err(|_| Error::<T>::MaximumSecretsStored)?;
					*maybe_secrets = Some(secrets);
					Ok(())
				} else {
					let mut secrets = BoundedVec::<Secret, T::MaximumStored>::default();
					secrets.try_push(new_secret.clone()).map_err(|_| Error::<T>::BoundsOverflow)?;
					Ok(())
				}
			})?;
			Self::deposit_event(Event::SecretCreated {
				id: unique_id,
				owner: owner.clone(),
				to: to.clone(),
				expiration_timestamp,
			});
			Ok(())
		}

		/// Deletes secret
		fn delete_secret(
			owner: &T::AccountId,
			to: &T::AccountId,
			unique_id: u64,
		) -> DispatchResult {
			SecretMap::<T>::remove(unique_id);
			// Try appending into the bounded vec
			OwnerMap::<T>::try_mutate(owner, to, |maybe_secret_ids| -> DispatchResult {
				if let Some(secret_ids) = maybe_secret_ids {
					secret_ids.into_iter().position(|id| id == &unique_id).map(|index| {
						secret_ids.remove(index);
					});
					Ok(())
				} else {
					Ok(())
				}
			})?;
			Self::deposit_event(Event::SecretDeleted { id: unique_id });
			Ok(())
		}
	}
}
