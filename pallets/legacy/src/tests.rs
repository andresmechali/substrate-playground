use crate::{mock::*, pallet::OwnerMap, Error, Event, Secret, SecretDuration, SecretMap};
use frame_support::{assert_noop, assert_ok};
use sp_core::bounded::BoundedVec;

#[test]
fn adds_secret() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		// Add a secret
		let new_secret_1 = Secret { id: 1, expiration_timestamp: 60 };
		assert_ok!(Legacy::create_secret(RuntimeOrigin::signed(1), 2, SecretDuration::Minutes(1)));

		// Assert that storage is updated
		assert_eq!(SecretMap::<Test>::get(1), Some(new_secret_1.clone()));
		assert_eq!(OwnerMap::<Test>::get(1, 2), Some(BoundedVec::truncate_from(vec![1_u64])));

		// Assert that the correct event was deposited
		System::assert_last_event(
			Event::SecretCreated {
				id: new_secret_1.id,
				owner: 1,
				to: 2,
				expiration_timestamp: new_secret_1.expiration_timestamp,
			}
			.into(),
		);

		// Add a second secret
		let new_secret_2 = Secret { id: 2, expiration_timestamp: 30 };
		assert_ok!(Legacy::create_secret(RuntimeOrigin::signed(1), 2, SecretDuration::Seconds(30)));

		// Assert that the correct event was deposited
		System::assert_last_event(
			Event::SecretCreated {
				id: new_secret_2.id,
				owner: 1,
				to: 2,
				expiration_timestamp: new_secret_2.expiration_timestamp,
			}
			.into(),
		);

		// Assert that storage is updated
		assert_eq!(SecretMap::<Test>::get(2), Some(new_secret_2.clone()));
		assert_eq!(
			OwnerMap::<Test>::get(1, 2),
			Some(BoundedVec::truncate_from(vec![1_u64, 2_u64]))
		);
	});
}

#[test]
fn cannot_add_more_than_max_secrets() {
	new_test_ext().execute_with(|| {
		// Add 2 secrets
		assert_ok!(Legacy::create_secret(RuntimeOrigin::signed(1), 2, SecretDuration::Minutes(1)));
		assert_ok!(Legacy::create_secret(RuntimeOrigin::signed(1), 2, SecretDuration::Minutes(1)));

		// Assert than an error is thrown when trying to add a third
		assert_noop!(
			Legacy::create_secret(RuntimeOrigin::signed(1), 2, SecretDuration::Minutes(1)),
			Error::<Test>::MaximumSecretsStored
		);
	});
}

#[test]
fn removes_secrets() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Add 2 secrets
		assert_ok!(Legacy::create_secret(RuntimeOrigin::signed(1), 2, SecretDuration::Minutes(1)));
		assert_ok!(Legacy::create_secret(RuntimeOrigin::signed(1), 2, SecretDuration::Seconds(30)));

		// Assert that they are stored
		assert_eq!(
			OwnerMap::<Test>::get(1, 2),
			Some(BoundedVec::truncate_from(vec![1_u64, 2_u64]))
		);
		assert_eq!(SecretMap::<Test>::get(1), Some(Secret { id: 1, expiration_timestamp: 60 }));
		assert_eq!(SecretMap::<Test>::get(2), Some(Secret { id: 2, expiration_timestamp: 30 }));

		// Remove the first secret
		assert_ok!(Legacy::delete_secret(RuntimeOrigin::signed(1), 2, 1_u64));

		// Assert that the correct event was deposited
		System::assert_last_event(Event::SecretDeleted { id: 1 }.into());

		// Assert that storage is updated
		assert_eq!(OwnerMap::<Test>::get(1, 2), Some(BoundedVec::truncate_from(vec![2_u64])));
		assert_eq!(SecretMap::<Test>::get(1), None);

		// Remove the second secret
		assert_ok!(Legacy::delete_secret(RuntimeOrigin::signed(1), 2, 2_u64));

		// Assert that the correct event was deposited
		System::assert_last_event(Event::SecretDeleted { id: 2 }.into());

		// Assert that storage is updated
		assert_eq!(OwnerMap::<Test>::get(1, 2), None);
		assert_eq!(SecretMap::<Test>::get(2), None);
		// assert_eq!(1, 2);
	});
}
