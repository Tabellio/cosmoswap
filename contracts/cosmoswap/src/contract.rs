use cosmoswap_packages::funds::check_single_coin;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{coin, BankMsg, Coin, CosmosMsg};
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use std::ops::Mul;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, Swap, CONFIG, FEE_CONFIG, LOCK, SWAP};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmoswap";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // TODO: Make sure the sender is cosmoswap-controller contract

    if msg.swap_info.coin1.is_native {
        check_single_coin(&info, &msg.swap_info.coin1.coin)?;
    };

    let config = Config { admin: info.sender };
    CONFIG.save(deps.storage, &config)?;

    FEE_CONFIG.save(deps.storage, &msg.fee_info)?;

    let user1 = deps.api.addr_validate(&msg.swap_info.user1)?;
    let user2 = deps.api.addr_validate(&msg.swap_info.user2)?;
    let swap = Swap {
        user1,
        user2,
        coin1: msg.swap_info.coin1.coin,
        coin2: msg.swap_info.coin2.coin,
    };
    SWAP.save(deps.storage, &swap)?;

    // Set swap lock to false
    LOCK.save(deps.storage, &false)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("user1", swap.user1)
        .add_attribute("user2", swap.user2)
        .add_attribute("coin1", swap.coin1.to_string())
        .add_attribute("coin2", swap.coin2.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Accept {} => execute_accept(deps, env, info),
        ExecuteMsg::Cancel {} => execute_cancel(deps, env, info),
    }
}

pub fn execute_accept(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Return error if swap is locked by user1
    let lock = LOCK.load(deps.storage)?;
    if lock {
        return Err(ContractError::SwapLocked {});
    }

    let fee_config = FEE_CONFIG.load(deps.storage)?;
    let swap = SWAP.load(deps.storage)?;

    // Return error if the sender is not user2
    if info.sender != swap.user2 {
        return Err(ContractError::Unauthorized {});
    };

    check_funds(&info, swap.coin2.clone())?;

    // Calculate swap fees
    let coin1_fee = swap.coin1.amount.mul(fee_config.percentage);
    let coin2_fee = swap.coin2.amount.mul(fee_config.percentage);

    let mut msgs: Vec<CosmosMsg> = vec![];

    // Swap fees
    msgs.push(CosmosMsg::Bank(BankMsg::Send {
        to_address: fee_config.payment_address.to_string(),
        amount: vec![
            coin(coin1_fee.u128(), swap.coin1.denom.clone()),
            coin(coin2_fee.u128(), swap.coin2.denom.clone()),
        ],
    }));
    // User1 coin
    msgs.push(CosmosMsg::Bank(BankMsg::Send {
        to_address: swap.user2.to_string(),
        amount: vec![coin(
            swap.coin1.amount.checked_sub(coin1_fee)?.u128(),
            swap.coin1.denom,
        )],
    }));
    // User2 coin
    msgs.push(CosmosMsg::Bank(BankMsg::Send {
        to_address: swap.user1.to_string(),
        amount: vec![coin(
            swap.coin2.amount.checked_sub(coin2_fee)?.u128(),
            swap.coin2.denom,
        )],
    }));

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "accept"))
}

pub fn execute_cancel(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let swap = SWAP.load(deps.storage)?;
    if info.sender != swap.user1 {
        return Err(ContractError::Unauthorized {});
    };

    LOCK.save(deps.storage, &true)?;

    Ok(Response::new().add_attribute("action", "cancel"))
}

fn check_funds(info: &MessageInfo, coin: Coin) -> Result<(), ContractError> {
    // Check for funds list length
    if info.funds.len() != 1 {
        return Err(ContractError::FundsNotFound {});
    }
    // Check for the exact coin
    if let Some(funds) = info.funds.get(0) {
        if funds.amount != coin.amount {
            return Err(ContractError::InvalidAmount {});
        };
        if funds.denom != coin.denom {
            return Err(ContractError::InvalidDenom {});
        };
    };
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Swap {} => to_binary(&query_swap(deps, env)?),
    }
}

fn query_swap(deps: Deps, _env: Env) -> StdResult<Swap> {
    let swap = SWAP.load(deps.storage)?;
    Ok(swap)
}
