use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{HumanAddr, Uint128, CanonicalAddr};
use crate::state::State;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub address_loterra_smart_contract: HumanAddr,
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get config state
    Config{},
    /// Get tokens holders, address and balance
    GetAllHolders {},
    /// Get specific holder, address and balance
    GetHolder { address: HumanAddr},
    /// Not used to be called directly
    TransferFrom {
        owner: HumanAddr,
        recipient: HumanAddr,
        amount: Uint128,
    },

    /*/// Not used to call
    Allowance{
        owner: HumanAddr,
        spender: HumanAddr,
    }*/
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetAllHoldersResponse {
    pub address: CanonicalAddr,
    pub bonded: Uint128
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetHolderResponse {
    pub address: CanonicalAddr,
    pub bonded: Uint128,
    pub unBonded: Uint128,
    pub available: Uint128
}

pub type ConfigResponse = State;
