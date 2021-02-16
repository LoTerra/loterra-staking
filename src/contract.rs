use cosmwasm_std::{to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdError, StdResult, Storage, Uint128, HumanAddr, CosmosMsg, WasmMsg, LogAttribute};

use crate::msg::{HandleMsg, InitMsg, QueryMsg, ConfigResponse};
use crate::state::{config, config_read, State, staking_storage, StakingInfo};
use std::ops::Add;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        address_cw20_loterra_smart_contract: deps.api.canonical_address(&msg.address_cw20_loterra_smart_contract)?,
    };

    config(&mut deps.storage).save(&state)?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Stake {amount} => handle_stake(deps, env, amount),
        HandleMsg::UnStake {amount} => handle_unstake(deps, env, amount),
        HandleMsg::ClaimReward {} => handle_claim_reward(deps, env),
        HandleMsg::ClaimUnStaked {} => handle_claim_unstake(deps, env),
    }
}
fn encode_msg_execute(msg: QueryMsg, address: HumanAddr) -> StdResult<CosmosMsg> {
    Ok(WasmMsg::Execute {
        contract_addr: address,
        msg: to_binary(&msg)?,
        send: vec![],
    }
        .into())
}
pub fn handle_stake<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Uint128
) -> StdResult<HandleResponse> {
    let state = config(&mut deps.storage).load()?;

    if !env.message.sent_funds.is_empty(){
        return Err(StdError::generic_err("Do not send funds with stake"));
    }
    // Prepare msg to send
    let msg = QueryMsg::TransferFrom {
        owner: env.message.sender,
        recipient: env.contract.address,
        amount
    };
    // Convert state address of loterra cw-20
    let lottera_human = deps
        .api
        .human_address(&state.address_cw20_loterra_smart_contract.clone())?;
    // Prepare the message
    let res = encode_msg_query(msg, lottera_human)?;

    let sender_canonical = deps.api.canonical_address(&env.message.sender)?;
    match staking_storage(&mut deps.storage).may_load(&sender_canonical.as_slice())? {
        Some(e) => {
            staking_storage(&mut deps.storage).update::<_>(&sender_canonical.as_slice(), |stake| {
                let mut stake_data = stake.unwrap();
                stake_data.bonded.add(amount);

                Ok(stake_data)
            })?;
        },
        None => {
            staking_storage(&mut deps.storage).save(&sender_canonical.as_slice(), &StakingInfo{
                bonded: amount,
                un_bonded: Uint128::zero(),
                period: 0,
                available: Uint128::zero()
            });
        }
    };

    Ok(HandleResponse {
        messages: vec![res.into()],
        log: vec![
            LogAttribute {
                key: "action".to_string(),
                value: "bond lota".to_string(),
            },
            LogAttribute {
                key: "from".to_string(),
                value: env.message.sender.to_string(),
            },
            LogAttribute {
                key: "to".to_string(),
                value: env.contract.address.to_string(),
            },
        ],
        data: None,
    })

}


pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::GetAllHolders {} => to_binary(&query_all_holders(deps)?),
        QueryMsg::GetHolder {address} => to_binary(&query_holder(deps, address)?),
        QueryMsg::TransferFrom {..} => to_binary(&query_transfer_from(deps)?),
    }
}

fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let state = config_read(&deps.storage).load()?;
    Ok(state)
}
fn query_all_holders<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let state = config_read(&deps.storage).load()?;
    Ok(state)
}
fn query_holder<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: HumanAddr,
) -> StdResult<ConfigResponse> {
    let state = config_read(&deps.storage).load()?;
    Ok(state)
}
fn query_transfer_from<S: Storage, A: Api, Q: Querier>(
    _deps: &Extern<S, A, Q>,
) -> StdResult<StdError> {
    Err(StdError::Unauthorized { backtrace: None })
}


#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, StdError};

    struct BeforeAll {
        default_length: usize,
        default_sender: HumanAddr,
        default_sender_two: HumanAddr,
        default_sender_owner: HumanAddr,
        default_contract_address: HumanAddr,
    }
    fn before_all() -> BeforeAll {
        BeforeAll {
            default_length: HumanAddr::from("terra1q88h7ewu6h3am4mxxeqhu3srt7zw4z5s20qu3k").len(),
            default_sender: HumanAddr::from("terra1q88h7ewu6h3am4mxxeqhu3srt7zw4z5s20q007"),
            default_sender_two: HumanAddr::from("terra1q88h7ewu6h3am4mxxeqhu3srt7zw4z5s20q008"),
            default_sender_owner: HumanAddr::from("terra1q88h7ewu6h3am4mxxeqhu3srt7zw4z5s20qu3k"),
            default_contract_address: HumanAddr::from("terra1q88h7ewu6h3am4mxxeqhu3srt7zw4z5s20LOTA"),
        }
    }

    fn default_init<S: Storage, A: Api, Q: Querier>(mut deps: &mut Extern<S, A, Q>){
        let before_all = before_all();
        let init = InitMsg{ address_loterra_smart_contract: before_all.default_contract_address };
        init(
            &mut deps,
            mock_env("terra1q88h7ewu6h3am4mxxeqhu3srt7zw4z5s20qu3k", &[]),
            init_msg,
        )
            .unwrap();
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);
        default_init(&mut deps);
        let env = mock_env("creator", &coins(1000, "earth"));
        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

}
