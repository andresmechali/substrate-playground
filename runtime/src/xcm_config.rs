use super::{
	parameter_types, AccountId, AssetsRegistry, Balance, Balances, ConstU32, DmpQueue,
	ParachainInfo, ParachainSystem, RuntimeCall, XcmPallet, XcmpQueue,
};
use crate::{
	governance::{EnsureRootOrHalfNativeTechnical, EnsureRootOrTwoThirdNativeCouncil},
	CurrencyId, ExistentialDeposits, GetNativeCurrencyId, Runtime, RuntimeEvent, RuntimeOrigin,
};
use assets_registry::traits::AssetRegistryReader;
// use cumulus_pallet_xcm::Origin as CumulusXcmOrigin;
use cumulus_primitives_utility::ParentAsUmp;
use frame_support::{
	dispatch::Weight,
	inherent::Vec,
	match_types,
	traits::{Everything, Get, Nothing},
};
use orml_traits::{
	location::{AbsoluteReserveProvider, RelativeReserveProvider},
	parameter_type_with_key, GetByKey, WeightToFeeConverter,
};
use orml_xcm_support::MultiNativeAsset;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use polkadot_primitives::Id as ParaId;
use sp_runtime::traits::Convert;
use sp_std::{marker::PhantomData, vec};
use xcm::{
	v3::{prelude::*, MultiLocation, Weight as XcmWeight},
	VersionedMultiLocation,
};
use xcm_builder::{
	AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, ChildParachainConvertsVia,
	CurrencyAdapter as XcmCurrencyAdapter, EnsureXcmOrigin, FixedRateOfFungible, FixedWeightBounds,
	IsConcrete, MintLocation, ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative,
	SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
	SovereignSignedViaLocation, TakeRevenue, TakeWeightCredit,
};
use xcm_executor::{traits::DropAssets, Assets, XcmExecutor};

parameter_types! {
	pub NativeTokenExistentialDeposit: Balance = 0; // TODO: get proper ED
	pub const BaseXcmWeight: Weight = Weight::from_ref_time(100_000_000);
	pub const XcmMaxAssetsForTransfer: usize = 2;
	pub const TokenLocation: MultiLocation = Here.into_location();
	pub const ThisNetwork: NetworkId = NetworkId::Rococo;
	pub const RelayNetwork: NetworkId = NetworkId::Rococo;
	pub const UnitWeightCost: Weight = Weight::from_ref_time(200_000_000);
	pub const MaxInstructions: u32 = 100;
	pub const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	pub DotPerSecond: (AssetId, u128, u128) = (MultiLocation::parent().into(), 1, 0);
	pub RelayOrigin: cumulus_pallet_xcm::Origin = cumulus_pallet_xcm::Origin::Relay;
	pub CheckAccount: AccountId = XcmPallet::check_account();
	pub LocalCheckAccount: (AccountId, MintLocation) =(CheckAccount::get(), MintLocation::Local);
	pub UniversalLocation: InteriorMultiLocation = X2(GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into()));
}

match_types! {
	pub type ParentOrSiblings: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(_) }
	};
}

pub type Barrier = (
	AllowKnownQueryResponses<XcmPallet>,
	AllowSubscriptionsFrom<ParentOrSiblings>,
	AllowTopLevelPaidExecutionFrom<Everything>,
	TakeWeightCredit,
);

// type AssetsIdConverter =
// 	CurrencyIdConvert<ForeignXcm, primitives::topology::Picasso, ParachainInfo>;

pub struct PriceConverter<AssetsRegistry, ForeignToNative>(
	PhantomData<(AssetsRegistry, ForeignToNative)>,
);

// pub struct WellKnownForeignToNativePriceConverter;
// impl ForeignToNativePriceConverter for WellKnownForeignToNativePriceConverter {
// 	fn get_ratio(asset_id: CurrencyId) -> Option<Rational64> {
// 		match asset_id {
// 			CurrencyId::MECH => Some(rational!(1 / 1)),
// 			_ => None,
// 		}
// 	}
// }

pub type Trader = FixedRateOfFungible<DotPerSecond, ToTreasury>;

pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
);

pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	// Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
	// recognized.
	RelayChainAsNative<RelayOrigin, RuntimeOrigin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognized.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `Origin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<RuntimeOrigin>,
);

// TODO: necessary?
// TODO: move
impl From<cumulus_pallet_xcm::Origin> for RuntimeOrigin {
	fn from(value: cumulus_pallet_xcm::Origin) -> Self {
		RuntimeOrigin::from(value)
	}
}

// TODO: necessary?
// TODO: move
impl From<cumulus_pallet_xcm::Event<Runtime>> for RuntimeEvent {
	fn from(event: cumulus_pallet_xcm::Event<Runtime>) -> Self {
		RuntimeEvent::from(event)
	}
}

// pub type XcmExecutor = runtime_common::XcmExecutor<
// 	XcmConfig,
// 	AccountId,
// 	Balance,
// 	LocationToAccountId,
// 	module_evm_bridge::EVMBridge<Runtime>,
// >;

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type VersionWrapper = XcmPallet;
	type ChannelInfo = ParachainSystem;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = cumulus_pallet_xcmp_queue::weights::SubstrateWeight<Self>;
	type ControllerOrigin = EnsureRootOrHalfNativeTechnical;
	type ExecuteOverweightOrigin = EnsureRootOrHalfNativeTechnical;
	type PriceForSiblingDelivery = ();
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRootOrTwoThirdNativeCouncil;
}

parameter_types! {
	pub const ThisLocal: MultiLocation = MultiLocation { parents: 0, interior: Here };
}

parameter_type_with_key! {
	pub ParachainMinFee: |location: MultiLocation| -> Option<Balance> {
		#[allow(clippy::match_ref_pats)] // false positive
		#[allow(clippy::match_single_binding)]
		let parents = location.parents;
		let interior = location.first_interior();

		let location = VersionedMultiLocation::V3(*location);
		if let Some(Parachain(id)) = interior {
			// TODO: do properly
			// if let Some(amount) = AssetsRegistry::min_xcm_fee(*id, location.into()) {
			// 	return Some(amount)
			// }
			return Some(0)
		}

		match (parents, interior) {
			(1, None) => Some(400_000),
			_ => None,
		}
	};
}

pub struct CurrencyIdConvert;
impl Convert<CurrencyId, Option<MultiLocation>> for CurrencyIdConvert {
	fn convert(_id: CurrencyId) -> Option<MultiLocation> {
		// TODO: implement properly
		Some(MultiLocation::parent())
	}
}
impl Convert<MultiLocation, Option<CurrencyId>> for CurrencyIdConvert {
	fn convert(_location: MultiLocation) -> Option<CurrencyId> {
		// TODO: implement properly
		Some(CurrencyId::MECH)
	}
}

pub struct ToTreasury;
impl TakeRevenue for ToTreasury {
	fn take_revenue(revenue: MultiAsset) {
		// if let MultiAsset { id: Concrete(location), fun: Fungible(amount) } = revenue {
		// 	if let Some(currency_id) = CurrencyIdConvert::convert(location) {
		// 		// let _ = Currencies::deposit(currency_id, &AcalaTreasuryAccount::get(), amount);
		// 	}
		// }
	}
}

pub struct AccountIdToMultiLocation;
impl Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
	fn convert(account: AccountId) -> MultiLocation {
		//  considers any other network using globally unique ids
		X1(AccountId32 { network: None, id: account.into() }).into()
	}
}

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = CurrencyIdConvert;
	type AccountIdToMultiLocation = AccountIdToMultiLocation;
	type SelfLocation = ThisLocal;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type MinXcmFee = ParachainMinFee;
	type MultiLocationsFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type BaseXcmWeight = BaseXcmWeight;
	type MaxAssetsForTransfer = XcmMaxAssetsForTransfer;
	type ReserveProvider = RelativeReserveProvider;
	type UniversalLocation = UniversalLocation;
}

pub type XcmRouter = (ParentAsUmp<ParachainSystem, XcmPallet, ()>, XcmpQueue);

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

// TODO: get proper weight with benchmark
pub struct XcmWeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_xcm::WeightInfo for XcmWeightInfo<T> {
	fn send() -> Weight {
		Weight::from_parts(0, 0)
	}
	fn teleport_assets() -> Weight {
		Weight::from_parts(0, 0)
	}
	fn reserve_transfer_assets() -> Weight {
		Weight::from_parts(0, 0)
	}
	fn execute() -> Weight {
		Weight::from_parts(0, 0)
	}
	fn force_xcm_version() -> Weight {
		Weight::from_parts(0, 0)
	}
	fn force_default_xcm_version() -> Weight {
		Weight::from_parts(0, 0)
	}
	fn force_subscribe_version_notify() -> Weight {
		Weight::from_parts(0, 0)
	}
	fn force_unsubscribe_version_notify() -> Weight {
		Weight::from_parts(0, 0)
	}
	fn migrate_supported_version() -> Weight {
		Weight::from_parts(0, 0)
	}
	fn migrate_version_notifiers() -> Weight {
		Weight::from_parts(0, 0)
	}
	fn already_notified_target() -> Weight {
		Weight::from_parts(0, 0)
	}
	fn notify_current_targets() -> Weight {
		Weight::from_parts(0, 0)
	}
	fn notify_target_migration_fail() -> Weight {
		Weight::from_parts(0, 0)
	}
	fn migrate_version_notify_targets() -> Weight {
		Weight::from_parts(0, 0)
	}
	fn migrate_and_notify_old_targets() -> Weight {
		Weight::from_parts(0, 0)
	}
}

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, ()>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Nothing;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Nothing;
	type XcmReserveTransferFilter = Everything;
	type WeightInfo = XcmWeightInfo<Runtime>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = ();
	type MaxLockers = ConstU32<8>;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type UniversalLocation = UniversalLocation;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	#[cfg(feature = "runtime-benchmarks")]
	type ReachableDest = ReachableDest;
}

/// `DropAssets` implementation support asset amount lower thant ED handled by `TakeRevenue`.
///
/// parameters type:
/// - `NC`: native currency_id type.
/// - `NB`: the ExistentialDeposit amount of native currency_id.
/// - `GK`: the ExistentialDeposit amount of tokens.
pub struct CustomDropAssets<X, T, C, NC, NB, GK>(PhantomData<(X, T, C, NC, NB, GK)>);
impl<X, T, C, NC, NB, GK> DropAssets for CustomDropAssets<X, T, C, NC, NB, GK>
where
	X: DropAssets,
	T: TakeRevenue,
	C: Convert<MultiLocation, Option<CurrencyId>>,
	NC: Get<CurrencyId>,
	NB: Get<Balance>,
	GK: GetByKey<CurrencyId, Balance>,
{
	fn drop_assets(origin: &MultiLocation, assets: Assets, context: &XcmContext) -> XcmWeight {
		let multi_assets: Vec<MultiAsset> = assets.into();
		let mut asset_traps: Vec<MultiAsset> = vec![];
		for asset in multi_assets {
			if let MultiAsset { id: Concrete(location), fun: Fungible(amount) } = asset.clone() {
				let currency_id = C::convert(location);
				// burn asset(do nothing here) if convert result is None
				if let Some(currency_id) = currency_id {
					// let ed = ExistentialDepositsForDropAssets::<NC, NB, GK>::get(&currency_id);
					// TODO: get proper ED
					let ed = AssetsRegistry::get_asset_existential_deposit(currency_id as u32);
					let ed = 0;
					if amount < ed {
						T::take_revenue(asset);
					} else {
						asset_traps.push(asset);
					}
				}
			}
		}
		if !asset_traps.is_empty() {
			X::drop_assets(origin, asset_traps.into(), context);
		}
		// TODO #2492: Put the real weight in there.
		XcmWeight::from_ref_time(0)
	}
}

impl<X, T, C, NC, NB, GK> Convert<CurrencyId, Option<MultiLocation>>
	for CustomDropAssets<X, T, C, NC, NB, GK>
{
	fn convert(_id: CurrencyId) -> Option<MultiLocation> {
		// TODO: implement properly
		Some(MultiLocation::parent())
	}
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = MultiNativeAsset<AbsoluteReserveProvider>;
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type Trader = Trader;
	type ResponseHandler = XcmPallet;
	// TODO:
	type AssetTrap = CustomDropAssets<
		XcmPallet,
		ToTreasury,
		CurrencyIdConvert,
		GetNativeCurrencyId,
		NativeTokenExistentialDeposit,
		ExistentialDeposits,
	>;
	type AssetLocker = ();
	type AssetExchanger = ();
	type AssetClaims = XcmPallet;
	type SubscriptionService = XcmPallet;
	type PalletInstancesInfo = ();
	type MaxAssetsIntoHolding = ConstU32<64>;
	type FeeManager = ();
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
}

pub type AssetTransactor = LocalAssetTransactor;

pub type LocationConverter =
	(ChildParachainConvertsVia<ParaId, AccountId>, AccountId32Aliases<ThisNetwork, AccountId>);

pub type LocalAssetTransactor = XcmCurrencyAdapter<
	Balances,
	IsConcrete<TokenLocation>,
	LocationConverter,
	AccountId,
	LocalCheckAccount,
>;
