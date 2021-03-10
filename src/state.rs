use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};

pub static CONFIG_KEY: &[u8] = b"config";
const STAKING_KEY: &[u8] = b"staking";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub admin: CanonicalAddr,
    pub address_cw20_loterra_smart_contract: CanonicalAddr,
    pub unbonded_period: u64,
    pub denom_reward: String,
    pub safe_lock: bool,
}

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakingInfo {
    pub bonded: Uint128,
    pub un_bonded: Uint128,
    pub period: u64,
    pub available: Uint128,
}

pub fn staking_storage<T: Storage>(storage: &mut T) -> Bucket<T, StakingInfo> {
    bucket(STAKING_KEY, storage)
}

pub fn staking_storage_read<T: Storage>(storage: &T) -> ReadonlyBucket<T, StakingInfo> {
    bucket_read(STAKING_KEY, storage)
}
