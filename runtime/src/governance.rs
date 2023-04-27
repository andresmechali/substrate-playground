use crate::*;
use frame_support::{
	instances::{Instance1, Instance2},
	parameter_types,
	traits::{EitherOfDiverse, EqualPrivilegeOnly},
};
use frame_system::EnsureRoot;
use pallet_collective::EnsureProportionAtLeast;
use sp_core::ConstU32;

use pallet_preimage;

pub type NativeCouncilCollective = Instance1;
pub type NativeTechnicalCollective = Instance2;

pub type EnsureRootOrHalfNativeTechnical = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EnsureProportionAtLeast<AccountId, NativeTechnicalCollective, 1, 2>,
>;

pub type EnsureRootOrTwoThirdNativeCouncil = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EnsureProportionAtLeast<AccountId, NativeCouncilCollective, 2, 3>,
>;

impl pallet_preimage::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type BaseDeposit = ();
	type ByteDeposit = ();
	type WeightInfo = ();
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Weight::from_parts(4_u64, 5_u64);
	pub const MaxScheduledPerBlock: u32 = 50;
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type Preimages = Preimage;
	type WeightInfo = ();
}

parameter_types! {
	pub const GeneralCouncilMotionDuration: BlockNumber = 3 * DAYS;
	pub const CouncilDefaultMaxProposals: u32 = 20;
	pub const CouncilDefaultMaxMembers: u32 = 30;
}

impl pallet_collective::Config<Instance1> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = GeneralCouncilMotionDuration;
	type MaxProposals = CouncilDefaultMaxProposals;
	type MaxMembers = CouncilDefaultMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = ();
	type SetMembersOrigin = EnsureRoot<AccountId>;
}

parameter_types! {
	pub const TechnicalCouncilMotionDuration: BlockNumber = 3 * DAYS;
}

impl pallet_collective::Config<Instance2> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = TechnicalCouncilMotionDuration;
	type MaxProposals = CouncilDefaultMaxProposals;
	type MaxMembers = CouncilDefaultMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = ();
	type SetMembersOrigin = EnsureRoot<AccountId>;
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = 5 * DAYS;
	pub const EnactmentPeriod: BlockNumber = 2 * DAYS;
	pub const CooloffPeriod: BlockNumber = 7 * DAYS;
	pub const VotingPeriod: BlockNumber = 5 * DAYS;
	pub MinimumDeposit: Balance = Balance::from(100_u32);
	pub const InstantAllowed: bool = true;
	pub const MaxVotes: u32 = 100;
	pub const MaxProposals: u32 = 100;
	pub const DemocracyId: LockIdentifier = *b"democrac";
	// pub RootOrigin: RuntimeOrigin = frame_system::RawOrigin::Root.into();
}

impl pallet_democracy::Config for Runtime {
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type MinimumDeposit = MinimumDeposit;
	type ExternalOrigin = EnsureRoot<AccountId>;
	type ExternalMajorityOrigin = EnsureRoot<AccountId>;
	type ExternalDefaultOrigin = EnsureRoot<AccountId>;
	type FastTrackOrigin = EnsureRoot<AccountId>;
	type InstantOrigin = EnsureRoot<AccountId>;
	type CancellationOrigin = EnsureRoot<AccountId>;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	type CancelProposalOrigin = EnsureRoot<AccountId>;
	type VetoOrigin = EnsureSigned<AccountId>;
	type CooloffPeriod = CooloffPeriod;
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Preimages = ();
	type VoteLockingPeriod = ();
	type InstantAllowed = InstantAllowed;
	type FastTrackVotingPeriod = ();
	type MaxVotes = MaxVotes;
	type MaxProposals = MaxProposals;
	type MaxDeposits = ConstU32<100>;
	type MaxBlacklisted = ConstU32<100>;
	type Slash = ();
	type SubmitOrigin = EnsureSigned<AccountId>;
}
