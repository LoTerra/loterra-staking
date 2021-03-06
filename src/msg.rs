use crate::state::State;
use cosmwasm_std::{CanonicalAddr, HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub address_cw20_loterra_smart_contract: HumanAddr,
    pub unbonded_period: u64,
    pub denom_reward: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// Stake tokens
    Stake { amount: Uint128 },
    /// UnStake tokens,
    UnStake { amount: Uint128 },
    /// Claim reward
    ClaimReward {},
    /// Claim unStaked tokens, available after unBonded period
    ClaimUnStaked {},
    /// LoTerra payout staking reward
    PayoutReward {},
    /// Admin
    /// Security owner can switch on off to prevent exploit
    SafeLock {},
    /// Admin renounce and restore contract address to admin for full decentralization
    Renounce {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get config state
    Config {},
    /// Get specific holder, address and balance
    GetHolder { address: HumanAddr },
    /// Get specific all bonded tokens
    GetAllBonded {},
    /// Not used to be called directly
    TransferFrom {
        owner: HumanAddr,
        recipient: HumanAddr,
        amount: Uint128,
    },
    /// Not used to be called directly
    Transfer {
        recipient: HumanAddr,
        amount: Uint128,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetBondedResponse {
    pub address: CanonicalAddr,
    pub bonded: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetHolderResponse {
    pub address: HumanAddr,
    pub bonded: Uint128,
    pub un_bonded: Uint128,
    pub available: Uint128,
    pub period: u64,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetAllBondedResponse {
    pub total_bonded: Uint128,
}

pub type ConfigResponse = State;
