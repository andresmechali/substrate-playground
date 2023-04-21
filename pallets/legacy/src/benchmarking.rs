#![cfg(feature = "runtime-benchmarks")]
mod benchmarking {
	use crate::*;
	use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
	use frame_system::RawOrigin;

	benchmarks! {
		create_secret {
			let b in 1 .. 10;
			let caller = whitelisted_caller();
			let mut count = 0_u32;
		}: create_secret(RawOrigin::Signed(caller), whitelisted_caller(), SecretDuration::Seconds(1))
		verify {
			count = count + 1;
			assert_eq!(SecretMap::<T>::iter().count() as u32, count);
		}
	}

	impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::mock::Test);
}
