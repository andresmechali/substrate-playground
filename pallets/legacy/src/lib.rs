#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::pallet_prelude::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod secrets;

use chrono::Utc;

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub(crate) struct Secret {
	pub(crate) id: u64,
	// pub(crate) service: String,
	// pub(crate) username: String,
	// pub(crate) password: String,
	pub(crate) expiration_timestamp: u64,
}

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum SecretDuration {
	Seconds(u64),
	Minutes(u64),
	Hours(u64),
	Days(u64),
	Weeks(u64),
	Months(u64),
	Years(u64),
}

impl SecretDuration {
	fn to_timestamp(&self) -> u64 {
		let now = Utc::now().timestamp() as u64;
		match self {
			SecretDuration::Seconds(seconds) => now + seconds,
			SecretDuration::Minutes(minutes) => now + minutes * 60,
			SecretDuration::Hours(hours) => now + hours * 60 * 60,
			SecretDuration::Days(days) => now + days * 60 * 60 * 24,
			SecretDuration::Weeks(weeks) => now + weeks * 60 * 60 * 24 * 7,
			SecretDuration::Months(months) => now + months * 60 * 60 * 24 * 30,
			SecretDuration::Years(years) => now + years * 60 * 60 * 24 * 365,
		}
	}
}

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use crate::{Secret, SecretDuration};
	use frame_support::{pallet_prelude::*, traits::Currency};
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
		fn do_create_secret(
			owner: &T::AccountId,
			to: &T::AccountId,
			unique_id: u64,
			duration: SecretDuration,
		) -> DispatchResult {
			let expiration_timestamp = SecretDuration::to_timestamp(&duration);
			let new_secret = Secret { id: unique_id, expiration_timestamp };
			SecretMap::<T>::insert(unique_id, new_secret.clone());
			// Try appending into the bounded vec, or create a new one
			OwnerMap::<T>::try_mutate(owner, to, |maybe_secrets| -> DispatchResult {
				if let Some(mut secrets) = maybe_secrets.clone() {
					secrets
						.try_push(new_secret.id)
						.map_err(|_| Error::<T>::MaximumSecretsStored)?;
					// *maybe_secrets = Some(secrets);
					Ok(())
				} else {
					let mut secrets = BoundedVec::<Secret, T::MaximumStored>::default();
					secrets.try_push(new_secret.clone()).map_err(|_| Error::<T>::BoundsOverflow)?;
					Ok(())
				}
			})?;
			Pallet::<T>::deposit_event(Event::SecretCreated {
				id: unique_id,
				owner: owner.clone(),
				to: to.clone(),
				expiration_timestamp,
			});
			Ok(())
		}

		/// Deletes secret
		fn do_delete_secret(
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
			Pallet::<T>::deposit_event(Event::SecretDeleted { id: unique_id });
			Ok(())
		}

		/// Renovates secret by extending the expiration timestamp
		fn do_extend_secret(unique_id: u64, duration: SecretDuration) -> DispatchResult {
			let expiration_timestamp = SecretDuration::to_timestamp(&duration);
			SecretMap::<T>::try_mutate(unique_id, |maybe_secret| -> DispatchResult {
				if let Some(secret) = maybe_secret {
					secret.expiration_timestamp = expiration_timestamp;
					Ok(())
				} else {
					Ok(())
				}
			})?;
			Pallet::<T>::deposit_event(Event::SecretExtended {
				id: unique_id,
				expiration_timestamp,
			});
			Ok(())
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn create_secret(
			origin: OriginFor<T>,
			to: T::AccountId,
			duration: SecretDuration,
		) -> DispatchResultWithPostInfo {
			let owner = ensure_signed(origin)?;
			let unique_id = Pallet::<T>::gen_unique_id();
			Pallet::<T>::do_create_secret(&owner, &to, unique_id, duration)?;
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn delete_secret(
			origin: OriginFor<T>,
			to: T::AccountId,
			unique_id: u64,
		) -> DispatchResultWithPostInfo {
			let owner = ensure_signed(origin)?;
			Pallet::<T>::do_delete_secret(&owner, &to, unique_id)?;
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn extend_secret(
			origin: OriginFor<T>,
			unique_id: u64,
			duration: SecretDuration,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin)?;
			Pallet::<T>::do_extend_secret(unique_id, duration)?;
			Ok(().into())
		}
	}
}
