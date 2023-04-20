#![cfg_attr(not(feature = "std"), no_std)]

sp_api::decl_runtime_apis! {
    pub trait AssetsRegistryApi {
        fn get_value() -> u32;
    }
}