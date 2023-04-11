#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::pallet_prelude::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod secrets;

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub(crate) struct Secret<T: Config> {
	pub(crate) id: T::Nonce,
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
	fn to_timestamp(&self, now: u64) -> u64 {
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
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		traits::{Currency, LockIdentifier, LockableCurrency, Randomness, WithdrawReasons},
	};
	use frame_system::pallet_prelude::*;
	use pallet_timestamp::{self as timestamp};
	use sp_runtime::traits::{CheckedAdd, SaturatedConversion};

	pub const LEGACY_ID: LockIdentifier = *b"//legacy";

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub initial_nonce: T::Nonce,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { initial_nonce: 8_u64.into() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			Nonce::<T>::put(self.initial_nonce);
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub(super) type Nonce<T: Config> = StorageValue<_, T::Nonce, ValueQuery>;

	/// Maps the Secret struct to the unique_id.
	#[pallet::storage]
	pub(super) type SecretMap<T: Config> = StorageMap<_, Twox64Concat, T::Nonce, Secret<T>>;

	#[pallet::storage]
	pub(super) type OwnerMap<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		T::AccountId,
		BoundedVec<T::Nonce, T::MaximumStored>,
	>;

	// type BalanceOf<T> =
	// 	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::error]
	pub enum Error<T> {
		/// Each secret must have a unique identifier
		DuplicateSecret,
		/// An account can't exceed the `MaximumStored` constant
		MaximumSecretsStored,
		/// The total secrets stored can't exceed the nonce limit
		BoundsOverflow,
		/// Not enough balance
		InsufficientBalance,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new secret was successfully created
		SecretCreated {
			id: T::Nonce,
			owner: T::AccountId,
			to: T::AccountId,
			expiration_timestamp: u64,
		},
		/// A secret was successfully deleted
		SecretDeleted { id: T::Nonce },
		/// A secret was successfully extended
		SecretExtended { id: T::Nonce, expiration_timestamp: u64 },
		/// Capital has been locked
		CapitalLocked { user: T::AccountId, amount: BalanceOf<T> },
		/// Lock has been extended
		LockExtended { user: T::AccountId, amount: BalanceOf<T> },
		/// Lock has been removed
		LockRemoved { user: T::AccountId },
		/// RandomNumber
		RandomNumber(T::Hash),
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + timestamp::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		// type Currency: Currency<Self::AccountId>;

		#[pallet::constant]
		type MaximumStored: Get<u32>;

		#[pallet::constant]
		type InitialNonce: Get<u64>;

		type Nonce: Parameter
			+ Member
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ From<u64>
			+ Into<u64>
			+ CheckedAdd
			+ MaxEncodedLen;

		type StakeCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

		type RandomGenerator: Randomness<Self::Hash, Self::BlockNumber>;
	}

	type BalanceOf<T> =
		<<T as Config>::StakeCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	// Pallet internal functions
	impl<T: Config> Pallet<T> {
		// Generates and returns the unique_id
		fn gen_unique_id() -> T::Nonce {
			let current_nonce = Nonce::<T>::get();
			let next_nonce =
				current_nonce.checked_add(&T::Nonce::from(1_u64)).expect("Should not overflow");
			Nonce::<T>::put(next_nonce);
			next_nonce
		}

		/// Creates new secret
		fn do_create_secret(
			owner: &T::AccountId,
			to: &T::AccountId,
			duration: SecretDuration,
		) -> DispatchResult {
			let now = <timestamp::Pallet<T>>::get().saturated_into();
			let expiration_timestamp = SecretDuration::to_timestamp(&duration, now);
			let unique_id = Pallet::<T>::gen_unique_id();
			let new_secret = Secret { id: unique_id, expiration_timestamp };
			SecretMap::<T>::insert(unique_id, new_secret.clone());
			// Try appending into the bounded vec, or create a new one
			OwnerMap::<T>::try_mutate(owner, to, |maybe_secrets| -> DispatchResult {
				if let Some(ref mut secrets) = maybe_secrets {
					secrets
						.try_push(new_secret.id)
						.map_err(|_| Error::<T>::MaximumSecretsStored)?;
					Ok(())
				} else {
					let mut secrets = BoundedVec::<T::Nonce, T::MaximumStored>::default();
					secrets.try_push(unique_id).map_err(|_| Error::<T>::BoundsOverflow)?;
					*maybe_secrets = Some(secrets);
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
			owner: T::AccountId,
			to: T::AccountId,
			unique_id: T::Nonce,
		) -> DispatchResult {
			SecretMap::<T>::remove(unique_id);
			OwnerMap::<T>::try_mutate(
				owner.clone(),
				to.clone(),
				|maybe_secret_ids| -> DispatchResult {
					if let Some(secret_ids) = maybe_secret_ids {
						secret_ids.iter().position(|id| id == &unique_id).map(|index| {
							secret_ids.remove(index);
						});
						if secret_ids.len() == 0 {
							*maybe_secret_ids = None;
							// TODO: Remove the owner-beneficiary pair from the OwnerMap
							// OwnerMap::<T>::remove(owner, to);
						}
						Ok(())
					} else {
						Ok(())
					}
				},
			)?;
			Pallet::<T>::deposit_event(Event::SecretDeleted { id: unique_id });
			Ok(())
		}

		/// Renovates secret by extending the expiration timestamp
		fn do_extend_secret(unique_id: T::Nonce, duration: SecretDuration) -> DispatchResult {
			let now = <timestamp::Pallet<T>>::get().saturated_into();
			let expiration_timestamp = SecretDuration::to_timestamp(&duration, now);
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
			Pallet::<T>::do_create_secret(&owner, &to, duration)?;
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn delete_secret(
			origin: OriginFor<T>,
			to: T::AccountId,
			unique_id: T::Nonce,
		) -> DispatchResultWithPostInfo {
			let owner = ensure_signed(origin)?;
			Pallet::<T>::do_delete_secret(owner, to, unique_id)?;
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn extend_secret(
			origin: OriginFor<T>,
			unique_id: T::Nonce,
			duration: SecretDuration,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin)?;
			Pallet::<T>::do_extend_secret(unique_id, duration)?;
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn lock_capital(
			origin: OriginFor<T>,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;
			ensure!(
				T::StakeCurrency::free_balance(&user) >= amount,
				Error::<T>::InsufficientBalance
			);
			T::StakeCurrency::set_lock(LEGACY_ID, &user, amount, WithdrawReasons::all());

			Self::deposit_event(Event::CapitalLocked { user, amount });
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn extend_lock(
			origin: OriginFor<T>,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;
			ensure!(
				T::StakeCurrency::free_balance(&user) >= amount,
				Error::<T>::InsufficientBalance
			);
			T::StakeCurrency::extend_lock(LEGACY_ID, &user, amount, WithdrawReasons::all());

			Self::deposit_event(Event::LockExtended { user, amount });
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn remove_lock(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;
			T::StakeCurrency::remove_lock(LEGACY_ID, &user);

			Self::deposit_event(Event::LockRemoved { user });
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn get_random_number(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin)?;
			let nonce = Self::gen_unique_id();
			let encoded_nonce = nonce.encode();

			let (random_number, _) = T::RandomGenerator::random(&encoded_nonce);

			Self::deposit_event(Event::RandomNumber(random_number));
			Ok(().into())
		}
	}
}
