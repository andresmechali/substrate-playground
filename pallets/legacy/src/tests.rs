use crate::{
	mock::*, pallet::OwnerMap, Error, Event, Secret, SecretDuration, SecretMap, LEGACY_ID,
};
use frame_support::{assert_noop, assert_ok, traits::WithdrawReasons};
use pallet_balances::BalanceLock;
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
fn locks_extends_and_unlocks() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Lock capital
		assert_ok!(Legacy::lock_capital(RuntimeOrigin::signed(ALICE), 100));

		// Assert that the correct event was deposited
		System::assert_last_event(Event::CapitalLocked { user: ALICE, amount: 100 }.into());

		// Assert that the lock exists
		assert_eq!(
			<Test as super::Config>::Currency::locks(ALICE),
			vec![BalanceLock {
				id: LEGACY_ID,
				amount: 100,
				reasons: WithdrawReasons::all().into()
			}]
		);

		System::set_block_number(2);

		// Lock more capital
		assert_ok!(Legacy::lock_capital(RuntimeOrigin::signed(ALICE), 200));

		// Assert that the correct event was deposited
		System::assert_last_event(Event::CapitalLocked { user: ALICE, amount: 200 }.into());

		// Assert that the lock has been updated
		assert_eq!(
			<Test as super::Config>::Currency::locks(ALICE),
			vec![BalanceLock {
				id: LEGACY_ID,
				amount: 200,
				reasons: WithdrawReasons::all().into()
			}]
		);

		// Extend lock
		assert_ok!(Legacy::extend_lock(RuntimeOrigin::signed(ALICE), 300));

		// Assert that the correct event was deposited
		System::assert_last_event(Event::LockExtended { user: ALICE, amount: 300 }.into());

		// Assert that the lock has been updated
		assert_eq!(
			<Test as super::Config>::Currency::locks(ALICE),
			vec![BalanceLock {
				id: LEGACY_ID,
				amount: 300,
				reasons: WithdrawReasons::all().into()
			}]
		);

		// Remove lock
		assert_ok!(Legacy::remove_lock(RuntimeOrigin::signed(ALICE)));

		// Assert that the correct event was deposited
		System::assert_last_event(Event::LockRemoved { user: ALICE }.into());

		// Assert that the lock has been updated
		assert_eq!(<Test as super::Config>::Currency::locks(ALICE), vec![]);
	});
}

#[test]
fn cannot_lock_or_extend_if_insufficient_balance() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Attempts to lock more capital that free_balance
		assert_noop!(
			Legacy::lock_capital(RuntimeOrigin::signed(ALICE), 10_000),
			Error::<Test>::InsufficientBalance
		);

		// Locks correct amount of balance
		assert_ok!(Legacy::lock_capital(RuntimeOrigin::signed(ALICE), 100));

		// Assert that the correct event was deposited
		System::assert_last_event(Event::CapitalLocked { user: ALICE, amount: 100 }.into());

		// Attempts to extend lock with more capital than free_balance
		assert_noop!(
			Legacy::extend_lock(RuntimeOrigin::signed(ALICE), 10_000),
			Error::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn generate_random_number() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Generates random number
		assert_ok!(Legacy::get_random_number(RuntimeOrigin::signed(ALICE)),);
	});
}
