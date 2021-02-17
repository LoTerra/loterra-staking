use cosmwasm_std::{to_binary, Api, BankMsg, Binary, Coin, CosmosMsg, Env, Extern, HandleResponse, HumanAddr, InitResponse, LogAttribute, Querier, StdError, StdResult, Storage, Uint128, WasmMsg, Order, CanonicalAddr};


use crate::msg::{ConfigResponse, HandleMsg, InitMsg, QueryMsg, GetHolderResponse, GetBondedResponse};
use crate::state::{config, config_read, staking_storage, StakingInfo, State, staking_storage_read};
use std::ops::{Add, Sub};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        admin: deps.api.canonical_address(&env.message.sender)?,
        address_cw20_loterra_smart_contract: deps
            .api
            .canonical_address(&msg.address_cw20_loterra_smart_contract)?,
        unbonded_period: msg.unbonded_period,
        denom_reward: msg.denom_reward,
        safe_lock: false,

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
        HandleMsg::Stake { amount } => handle_stake(deps, env, amount),
        HandleMsg::UnStake { amount } => handle_unstake(deps, env, amount),
        HandleMsg::ClaimReward {} => handle_claim_reward(deps, env),
        HandleMsg::ClaimUnStaked {} => handle_claim_unstake(deps, env),
        HandleMsg::SafeLock {} => handle_safe_lock(deps, env),
        HandleMsg::Renounce {} => handle_renounce(deps, env),
        HandleMsg::UpdateRewardAvailable {} => handle_update_reward_available(deps, env),
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

pub fn handle_renounce<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    // Load the state
    let mut state = config(&mut deps.storage).load()?;
    let sender = deps.api.canonical_address(&env.message.sender)?;
    if state.admin != sender {
        return Err(StdError::Unauthorized { backtrace: None });
    }
    if state.safe_lock {
        return Err(StdError::generic_err("Contract is locked"));
    }

    state.admin = deps.api.canonical_address(&env.contract.address)?;
    config(&mut deps.storage).save(&state)?;
    Ok(HandleResponse::default())
}

pub fn handle_safe_lock<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    // Load the state
    let mut state = config(&mut deps.storage).load()?;
    let sender = deps.api.canonical_address(&env.message.sender)?;
    if state.admin != sender {
        return Err(StdError::Unauthorized { backtrace: None });
    }

    state.safe_lock = !state.safe_lock;
    config(&mut deps.storage).save(&state)?;

    Ok(HandleResponse::default())
}

pub fn handle_stake<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    let state = config(&mut deps.storage).load()?;

    if state.safe_lock {
        return Err(StdError::generic_err(
            "Contract deactivated for update or/and preventing security issue",
        ));
    }

    if !env.message.sent_funds.is_empty() {
        return Err(StdError::generic_err("Do not send funds with stake"));
    }
    // Prepare msg to send
    let msg = QueryMsg::TransferFrom {
        owner: env.message.sender.clone(),
        recipient: env.contract.address.clone(),
        amount,
    };
    // Convert state address of loterra cw-20
    let lottera_human = deps
        .api
        .human_address(&state.address_cw20_loterra_smart_contract.clone())?;
    // Prepare the message
    let res = encode_msg_execute(msg, lottera_human)?;

    let sender_canonical = deps.api.canonical_address(&env.message.sender)?;
    match staking_storage(&mut deps.storage).may_load(&sender_canonical.as_slice())? {
        Some(_e) => {
            staking_storage(&mut deps.storage).update::<_>(
                &sender_canonical.as_slice(),
                |stake| {
                    let mut stake_data = stake.unwrap();
                    stake_data.bonded.add(amount);

                    Ok(stake_data)
                },
            )?;
        }
        None => {
            staking_storage(&mut deps.storage).save(
                &sender_canonical.as_slice(),
                &StakingInfo {
                    bonded: amount,
                    un_bonded: Uint128::zero(),
                    period: 0,
                    available: Uint128::zero(),
                },
            )?;
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
            LogAttribute {
                key: "amount".to_string(),
                value: amount.to_string(),
            },
        ],
        data: None,
    })
}

pub fn handle_unstake<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    let state = config(&mut deps.storage).load()?;

    if state.safe_lock {
        return Err(StdError::generic_err(
            "Contract deactivated for update or/and preventing security issue",
        ));
    }

    if !env.message.sent_funds.is_empty() {
        return Err(StdError::generic_err("Do not send funds with stake"));
    }

    let sender_canonical = deps.api.canonical_address(&env.message.sender)?;
    match staking_storage(&mut deps.storage).may_load(&sender_canonical.as_slice())? {
        Some(_e) => {
            staking_storage(&mut deps.storage).update::<_>(
                &sender_canonical.as_slice(),
                |stake| {
                    let mut stake_data = stake.unwrap();
                    if stake_data.bonded < amount {
                        return Err(StdError::generic_err(format!(
                            "You can't unStake more than you have ({})",
                            stake_data.bonded.u128().to_string()
                        )));
                    }
                    stake_data.bonded.sub(amount);
                    stake_data.un_bonded.add(amount);
                    stake_data.period = env.block.height + state.unbonded_period;
                    Ok(stake_data)
                },
            )?;
        }
        None => {
            return Err(StdError::Unauthorized { backtrace: None });
        }
    };

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            LogAttribute {
                key: "action".to_string(),
                value: "unbond lota".to_string(),
            },
            LogAttribute {
                key: "amount".to_string(),
                value: amount.to_string(),
            },
        ],
        data: None,
    })
}

pub fn handle_claim_unstake<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let state = config(&mut deps.storage).load()?;

    if state.safe_lock {
        return Err(StdError::generic_err(
            "Contract deactivated for update or/and preventing security issue",
        ));
    }

    if !env.message.sent_funds.is_empty() {
        return Err(StdError::generic_err("Do not send funds with stake"));
    }

    let sender_canonical = deps.api.canonical_address(&env.message.sender)?;
    let store = staking_storage(&mut deps.storage).load(&sender_canonical.as_slice())?;

    if store.period > env.block.height {
        return Err(StdError::generic_err(format!(
            "Your unBonded token will be released at block {}",
            store.period
        )));
    }
    // Prepare msg to send
    let msg = QueryMsg::Transfer {
        recipient: env.contract.address.clone(),
        amount: store.un_bonded,
    };
    // Convert state address of loterra cw-20
    let lottera_human = deps
        .api
        .human_address(&state.address_cw20_loterra_smart_contract.clone())?;
    // Prepare the message
    let res = encode_msg_execute(msg, lottera_human)?;

    staking_storage(&mut deps.storage).update::<_>(&sender_canonical.as_slice(), |stake| {
        let mut stake_data = stake.unwrap();
        stake_data.un_bonded = Uint128::zero();
        Ok(stake_data)
    })?;

    Ok(HandleResponse {
        messages: vec![res.into()],
        log: vec![
            LogAttribute {
                key: "action".to_string(),
                value: "claim unstake".to_string(),
            },
            LogAttribute {
                key: "from".to_string(),
                value: env.contract.address.to_string(),
            },
            LogAttribute {
                key: "to".to_string(),
                value: env.message.sender.to_string(),
            },
            LogAttribute {
                key: "amount".to_string(),
                value: store.un_bonded.to_string(),
            },
        ],
        data: None,
    })
}

pub fn handle_claim_reward<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let state = config(&mut deps.storage).load()?;

    if state.safe_lock {
        return Err(StdError::generic_err(
            "Contract deactivated for update or/and preventing security issue",
        ));
    }

    if !env.message.sent_funds.is_empty() {
        return Err(StdError::generic_err("Do not send funds with stake"));
    }

    let sender_canonical = deps.api.canonical_address(&env.message.sender)?;
    let store = staking_storage(&mut deps.storage).load(&sender_canonical.as_slice())?;
    let contract_balance = deps
        .querier
        .query_balance(env.contract.address.clone(), &state.denom_reward)?;

    if contract_balance.amount < store.available {
        return Err(StdError::generic_err("Contract balance too low"));
    }

    let msg = BankMsg::Send {
        from_address: env.contract.address.clone(),
        to_address: env.message.sender.clone(),
        amount: vec![Coin {
            denom: state.denom_reward,
            amount: store.available,
        }],
    };

    staking_storage(&mut deps.storage).update::<_>(&sender_canonical.as_slice(), |stake| {
        let mut stake_data = stake.unwrap();
        stake_data.available = Uint128::zero();
        Ok(stake_data)
    })?;

    Ok(HandleResponse {
        messages: vec![msg.into()],
        log: vec![
            LogAttribute {
                key: "action".to_string(),
                value: "claim reward".to_string(),
            },
            LogAttribute {
                key: "from".to_string(),
                value: env.contract.address.to_string(),
            },
            LogAttribute {
                key: "to".to_string(),
                value: env.message.sender.to_string(),
            },
            LogAttribute {
                key: "amount".to_string(),
                value: store.available.to_string(),
            },
        ],
        data: None,
    })
}

pub fn handle_update_reward_available<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let state = config(&mut deps.storage).load()?;
    if state.safe_lock {
        return Err(StdError::generic_err(
            "Contract deactivated for update or/and preventing security issue",
        ));
    }

    let sent = match env.message.sent_funds.len() {
        0 => Err(StdError::generic_err(format!(
            "You need to send funds for share holders"
        ))),
        1 => {
            if env.message.sent_funds[0].denom == state.denom_reward {
                Ok(env.message.sent_funds[0].amount)
            } else {
                Err(StdError::generic_err(format!(
                    "Only {} is accepted",
                    state.denom_reward.clone()
                )))
            }
        }
        _ => Err(StdError::generic_err(format!(
            "Send only {}, no extra denom",
            state.denom_reward.clone()
        ))),
    }?;

    let mut total_staked = Uint128::zero();
    let staking = staking_storage(&mut deps.storage)
        .range(None, None, Order::Descending)
        .flat_map(|item| {
            item.and_then(|(k, staker)| {
                if !staker.bonded.is_zero(){
                    total_staked.add(staker.bonded);
                }

                Ok(GetBondedResponse{ address: CanonicalAddr::from(k), bonded: staker.bonded})
            })
        })
        .collect::<Vec<GetBondedResponse>>();

    let mut claimed_amount = Uint128::zero();

    if total_staked.is_zero(){
        return Err(StdError::generic_err("No amount staked"))
    }

    for staker in staking {
        if !staker.bonded.is_zero(){
            let reward = staker.bonded.multiply_ratio(sent, total_staked);
            if !reward.is_zero(){
                claimed_amount.add(reward);
                staking_storage(&mut deps.storage).update::<_>(&staker.address.as_slice(), |stake| {
                    let mut stake_data = stake.unwrap();
                    stake_data.available.add(reward);
                    Ok(stake_data)
                })?;
            }
        }
    }

    let final_refund_balance = sent.sub(claimed_amount)?;

    if final_refund_balance.is_zero(){
        return Ok(HandleResponse::default());
    }

    let msg = BankMsg::Send {
        from_address: env.contract.address.clone(),
        to_address: env.message.sender.clone(),
        amount: vec![Coin {
            denom: state.denom_reward,
            amount: final_refund_balance,
        }],
    };

    Ok(HandleResponse{
        messages: vec![msg.into()],
        log: vec![],
        data: None
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::GetHolder { address } => to_binary(&query_holder(deps, address)?),
        QueryMsg::TransferFrom { .. } => to_binary(&query_transfer_from(deps)?),
        QueryMsg::Transfer { .. } => to_binary(&query_transfer(deps)?),
    }
}

fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let state = config_read(&deps.storage).load()?;
    Ok(state)
}

fn query_holder<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: HumanAddr,
) -> StdResult<GetHolderResponse> {
    let address_to_canonical = deps.api.canonical_address(&address)?;
    let store = match staking_storage_read(&deps.storage).may_load(&address_to_canonical.as_slice())? {
        Some(stake) => Some(stake),
        None => {
            return Err(StdError::NotFound {
                kind: "not found".to_string(),
                backtrace: None,
            })
        }
    }.unwrap();

    Ok(GetHolderResponse{
        address,
        bonded: store.bonded,
        un_bonded: store.un_bonded,
        available: store.available
    })
}

fn query_transfer_from<S: Storage, A: Api, Q: Querier>(
    _deps: &Extern<S, A, Q>,
) -> StdResult<StdError> {
    Err(StdError::Unauthorized { backtrace: None })
}
fn query_transfer<S: Storage, A: Api, Q: Querier>(_deps: &Extern<S, A, Q>) -> StdResult<StdError> {
    Err(StdError::Unauthorized { backtrace: None })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins};
    use cosmwasm_std::StdError::{GenericErr};

    struct BeforeAll {
        default_length: usize,
        default_sender: HumanAddr,
        default_sender_two: HumanAddr,
        default_sender_owner: HumanAddr,
        default_contract_address: HumanAddr,
        default_contract_address_two: HumanAddr,
    }
    fn before_all() -> BeforeAll {
        BeforeAll {
            default_length: HumanAddr::from("terra1q88h7ewu6h3am4mxxeqhu3srt7zw4z5s20qu3k").len(),
            default_sender: HumanAddr::from("terra1q88h7ewu6h3am4mxxeqhu3srt7zw4z5s20q007"),
            default_sender_two: HumanAddr::from("terra1q88h7ewu6h3am4mxxeqhu3srt7zw4z5s20q008"),
            default_sender_owner: HumanAddr::from("terra1q88h7ewu6h3am4mxxeqhu3srt7zw4z5s20qu3k"),
            default_contract_address: HumanAddr::from(
                "terra1q88h7ewu6h3am4mxxeqhu3srt7zw4z5s20LOTA",
            ),
            default_contract_address_two: HumanAddr::from(
                "terra1q88h7ewu6h3am4mxxeqhu3srt7zw4z5LOTERRA",
            ),
        }
    }

    fn default_init<S: Storage, A: Api, Q: Querier>(mut deps: &mut Extern<S, A, Q>) {
        let before_all = before_all();
        let init_msg = InitMsg {
            address_cw20_loterra_smart_contract: before_all.default_contract_address,
            unbonded_period: 100,
            denom_reward: "uusd".to_string(),
        };
        let res = init(
            &mut deps,
            mock_env("terra1q88h7ewu6h3am4mxxeqhu3srt7zw4z5s20qu3k", &[]),
            init_msg,
        ).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn proper_initialization() {
        let before_all = before_all();
        let mut deps = mock_dependencies(before_all.default_length, &[]);
        let env = mock_env("creator", &coins(1000, "earth"));
        default_init(&mut deps);
    }
    mod safe_lock {
        use super::*;
        // handle_switch

        #[test]
        fn only_admin() {
            let before_all = before_all();
            let mut deps = mock_dependencies(before_all.default_length, &[]);
            default_init(&mut deps);
            let env = mock_env(before_all.default_sender_two, &[]);

            let res = handle_safe_lock(&mut deps, env);
            match res {
                Err(StdError::Unauthorized { .. }) => {}
                _ => panic!("Unexpected error"),
            }
        }
        #[test]
        fn success() {
            let before_all = before_all();
            let mut deps = mock_dependencies(before_all.default_length, &[]);
            default_init(&mut deps);
            let env = mock_env(before_all.default_sender_owner, &[]);

            // Switch to Off
            let res = handle_safe_lock(&mut deps, env.clone()).unwrap();
            assert_eq!(res.messages.len(), 0);
            let state = config(&mut deps.storage).load().unwrap();
            assert!(state.safe_lock);
            // Switch to On
            let res = handle_safe_lock(&mut deps, env).unwrap();
            println!("{:?}", res);
            let state = config(&mut deps.storage).load().unwrap();
            assert!(!state.safe_lock);
        }
    }
    mod renounce {
        use super::*;
        // handle_renounce
        #[test]
        fn only_admin() {
            let before_all = before_all();
            let mut deps = mock_dependencies(before_all.default_length, &[]);
            default_init(&mut deps);
            let env = mock_env(before_all.default_sender_two, &[]);

            let res = handle_renounce(&mut deps, env);
            match res {
                Err(StdError::Unauthorized { .. }) => {}
                _ => panic!("Unexpected error"),
            }
        }
        #[test]
        fn safe_lock_on() {
            let before_all = before_all();
            let mut deps = mock_dependencies(before_all.default_length, &[]);
            default_init(&mut deps);
            let env = mock_env(before_all.default_sender_owner, &[]);

            let mut state = config(&mut deps.storage).load().unwrap();
            state.safe_lock = true;
            config(&mut deps.storage).save(&state).unwrap();

            let res = handle_renounce(&mut deps, env);
            match res {
                Err(GenericErr {
                        msg,
                        backtrace: None,
                    }) => {
                    assert_eq!(msg, "Contract is locked");
                }
                _ => panic!("Unexpected error"),
            }
        }
        #[test]
        fn success() {
            let before_all = before_all();
            let mut deps = mock_dependencies(before_all.default_length, &[]);
            default_init(&mut deps);
            let env = mock_env(before_all.default_sender_owner.clone(), &[]);

            // Transfer power to admin
            let res = handle_renounce(&mut deps, env.clone()).unwrap();
            assert_eq!(res.messages.len(), 0);
            let state = config(&mut deps.storage).load().unwrap();
            assert_ne!(
                state.admin,
                deps.api
                    .canonical_address(&before_all.default_sender_owner)
                    .unwrap()
            );
            assert_eq!(
                state.admin,
                deps.api.canonical_address(&env.contract.address).unwrap()
            );
        }
    }

    mod stake {
        use super::*;
        // handle_stake
        #[test]
        fn do_not_send_funds(){
            let before_all = before_all();
            let mut deps = mock_dependencies(before_all.default_length, &[]);
            default_init(&mut deps);
            let env = mock_env(before_all.default_sender_owner.clone(), &[Coin{ denom: "x".to_string(), amount: Uint128(2_000) }]);
            let msg = HandleMsg::Stake { amount: Uint128(2_000) };
            let res = handle(&mut deps, env.clone(), msg.clone());
            match res {
                Err(GenericErr {
                        msg,
                        backtrace: None,
                    }) => {
                    assert_eq!(msg, "Do not send funds with stake");
                }
                _ => panic!("Unexpected error"),
            }
        }
        #[test]
        fn success(){
            let before_all = before_all();
            let mut deps = mock_dependencies(before_all.default_length, &[]);
            default_init(&mut deps);
            let env = mock_env(before_all.default_sender_owner.clone(), &[]);
            let msg = HandleMsg::Stake { amount: Uint128(2_000) };
            let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
            println!("{:?}", res);
            assert_eq!(res.messages.len(), 1);
            let store = staking_storage(&mut deps.storage).load(&deps.api.canonical_address(&before_all.default_sender_owner).unwrap().as_slice()).unwrap();
            println!("{:?}", store);
            assert_eq!(store.bonded, Uint128(2_000));
        }
    }
}
