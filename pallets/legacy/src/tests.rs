use crate::{mock::*, pallet::OwnerMap, Error, Event, Secret, SecretDuration, SecretMap};
use frame_support::{assert_noop, assert_ok};
use sp_core::bounded::BoundedVec;

const ALICE: u64 = 1;
const ALICE_INITIAL_BALANCE: u64 = 1_000;
const BOB: u64 = 2;
const BOB_INITIAL_BALANCE: u64 = 2_000;

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	let genesis = pallet_balances::GenesisConfig::<Test> {
		balances: vec![(ALICE, ALICE_INITIAL_BALANCE), (BOB, BOB_INITIAL_BALANCE)],
	};
	genesis.assimilate_storage(&mut t).unwrap();
	t.into()
}

#[test]
fn adds_secret() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		// Add a secret
		let new_secret_1 = Secret { id: 1, expiration_timestamp: 60 };
		assert_ok!(Legacy::create_secret(
			RuntimeOrigin::signed(ALICE),
			BOB,
			SecretDuration::Minutes(1)
		));

		// Assert that storage is updated
		assert_eq!(SecretMap::<Test>::get(ALICE), Some(new_secret_1.clone()));
		assert_eq!(OwnerMap::<Test>::get(ALICE, BOB), Some(BoundedVec::truncate_from(vec![1_u64])));

		// Assert that the correct event was deposited
		System::assert_last_event(
			Event::SecretCreated {
				id: new_secret_1.id,
				owner: ALICE,
				to: BOB,
				expiration_timestamp: new_secret_1.expiration_timestamp,
			}
			.into(),
		);

		// Add a second secret
		let new_secret_2 = Secret { id: 2, expiration_timestamp: 30 };
		assert_ok!(Legacy::create_secret(
			RuntimeOrigin::signed(ALICE),
			BOB,
			SecretDuration::Seconds(30)
		));

		// Assert that the correct event was deposited
		System::assert_last_event(
			Event::SecretCreated {
				id: new_secret_2.id,
				owner: ALICE,
				to: BOB,
				expiration_timestamp: new_secret_2.expiration_timestamp,
			}
			.into(),
		);

		// Assert that storage is updated
		assert_eq!(SecretMap::<Test>::get(BOB), Some(new_secret_2.clone()));
		assert_eq!(
			OwnerMap::<Test>::get(ALICE, BOB),
			Some(BoundedVec::truncate_from(vec![1_u64, 2_u64]))
		);
	});
}

#[test]
fn cannot_add_more_than_max_secrets() {
	new_test_ext().execute_with(|| {
		// Add 2 secrets
		assert_ok!(Legacy::create_secret(
			RuntimeOrigin::signed(ALICE),
			BOB,
			SecretDuration::Minutes(1)
		));
		assert_ok!(Legacy::create_secret(
			RuntimeOrigin::signed(ALICE),
			BOB,
			SecretDuration::Minutes(1)
		));

		// Assert than an error is thrown when trying to add a third
		assert_noop!(
			Legacy::create_secret(RuntimeOrigin::signed(ALICE), BOB, SecretDuration::Minutes(1)),
			Error::<Test>::MaximumSecretsStored
		);
	});
}

#[test]
fn removes_secrets() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Add 2 secrets
		assert_ok!(Legacy::create_secret(
			RuntimeOrigin::signed(ALICE),
			BOB,
			SecretDuration::Minutes(1)
		));
		assert_ok!(Legacy::create_secret(
			RuntimeOrigin::signed(ALICE),
			BOB,
			SecretDuration::Seconds(30)
		));

		// Assert that they are stored
		assert_eq!(
			OwnerMap::<Test>::get(ALICE, BOB),
			Some(BoundedVec::truncate_from(vec![1_u64, 2_u64]))
		);
		assert_eq!(SecretMap::<Test>::get(ALICE), Some(Secret { id: 1, expiration_timestamp: 60 }));
		assert_eq!(SecretMap::<Test>::get(BOB), Some(Secret { id: 2, expiration_timestamp: 30 }));

		// Remove the first secret
		assert_ok!(Legacy::delete_secret(RuntimeOrigin::signed(ALICE), BOB, 1_u64));

		// Assert that the correct event was deposited
		System::assert_last_event(Event::SecretDeleted { id: 1 }.into());

		// Assert that storage is updated
		assert_eq!(OwnerMap::<Test>::get(ALICE, BOB), Some(BoundedVec::truncate_from(vec![2_u64])));
		assert_eq!(SecretMap::<Test>::get(ALICE), None);

		// Remove the second secret
		assert_ok!(Legacy::delete_secret(RuntimeOrigin::signed(ALICE), BOB, 2_u64));

		// Assert that the correct event was deposited
		System::assert_last_event(Event::SecretDeleted { id: 2 }.into());

		// Assert that storage is updated
		assert_eq!(OwnerMap::<Test>::get(ALICE, BOB), None);
		assert_eq!(SecretMap::<Test>::get(BOB), None);
	});
}

#[test]
fn locks_capital() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Lock capital
		assert_ok!(Legacy::lock_capital(RuntimeOrigin::signed(ALICE), 100));

		// Assert that the correct event was deposited
		System::assert_last_event(Event::CapitalLocked { user: ALICE, amount: 100 }.into());

		assert_eq!(
			<Test as super::Config>::StakeCurrency::free_balance(ALICE),
			ALICE_INITIAL_BALANCE - 100
		);
	});
}
