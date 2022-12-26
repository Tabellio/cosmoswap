use cosmoswap_packages::funds::{check_single_coin, FundsError};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{coin, from_binary, Attribute, BankMsg, CosmosMsg, WasmMsg};
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use std::ops::Mul;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
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

    let config = Config {
        admin: info.sender,
        expiration: msg.expiration,
    };
    CONFIG.save(deps.storage, &config)?;

    FEE_CONFIG.save(deps.storage, &msg.fee_info)?;

    let user1 = deps.api.addr_validate(&msg.swap_info.user1)?;
    let user2 = deps.api.addr_validate(&msg.swap_info.user2)?;
    let swap = Swap {
        user1,
        user2,
        coin1: msg.swap_info.coin1,
        coin2: msg.swap_info.coin2,
    };
    SWAP.save(deps.storage, &swap)?;

    // Set swap lock to false
    LOCK.save(deps.storage, &false)?;

    let mut attrs: Vec<Attribute> = vec![];
    if let Some(cw20_addr) = swap.coin1.cw20_address {
        attrs.push(Attribute::new("coin1_cw20_address", cw20_addr));
    };
    if let Some(cw20_addr) = swap.coin2.cw20_address {
        attrs.push(Attribute::new("coin2_cw20_address", cw20_addr));
    };

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("user1", swap.user1)
        .add_attribute("user2", swap.user2)
        .add_attribute("coin1_amount", swap.coin1.coin.amount.to_string())
        .add_attribute("coin2_amount", swap.coin2.coin.amount.to_string())
        .add_attribute("coin1_denom", swap.coin1.coin.denom.to_string())
        .add_attribute("coin2_denom", swap.coin2.coin.denom.to_string())
        .add_attributes(attrs))
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
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
    }
}

pub fn execute_accept(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.expiration.is_expired(&env.block) {
        return Err(ContractError::SwapLocked {});
    };

    // Return error if swap is locked by user1
    let lock = LOCK.load(deps.storage)?;
    if lock {
        return Err(ContractError::SwapLocked {});
    }
    LOCK.save(deps.storage, &true)?;

    let swap = SWAP.load(deps.storage)?;

    // Return error if the sender is not user2
    if info.sender != swap.user2 {
        return Err(ContractError::Unauthorized {});
    };

    check_single_coin(&info, &swap.coin2.coin.clone())?;

    _accept(&deps, swap)
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

    let lock = LOCK.load(deps.storage)?;
    if lock {
        return Err(ContractError::SwapLocked {});
    }

    _cancel(deps, swap)
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    cw20_receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg: ReceiveMsg = from_binary(&cw20_receive_msg.msg)?;
    match msg {
        ReceiveMsg::Accept {} => {
            let config = CONFIG.load(deps.storage)?;
            if config.expiration.is_expired(&env.block) {
                return Err(ContractError::SwapLocked {});
            };

            let lock = LOCK.load(deps.storage)?;
            if lock {
                return Err(ContractError::SwapLocked {});
            }

            let swap = SWAP.load(deps.storage)?;

            if cw20_receive_msg.sender != swap.user2 {
                return Err(ContractError::Unauthorized {});
            };

            if !swap.coin2.is_native {
                if cw20_receive_msg.amount != swap.coin2.coin.amount {
                    return Err(FundsError::InvalidFunds {
                        got: cw20_receive_msg.amount.to_string(),
                        expected: swap.coin2.coin.amount.to_string(),
                    }
                    .into());
                };
            };

            _accept(&deps, swap)
        }
        ReceiveMsg::Cancel {} => {
            let swap = SWAP.load(deps.storage)?;

            if cw20_receive_msg.sender != swap.user1 {
                return Err(ContractError::Unauthorized {});
            };

            _cancel(deps, swap)
        }
    }
}

fn _accept(deps: &DepsMut, swap: Swap) -> Result<Response, ContractError> {
    let fee_config = FEE_CONFIG.load(deps.storage)?;

    // Calculate swap fees
    let coin1_fee = swap.coin1.coin.amount.mul(fee_config.percentage);
    let coin2_fee = swap.coin2.coin.amount.mul(fee_config.percentage);

    let mut msgs: Vec<CosmosMsg> = vec![];

    if swap.coin1.is_native {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: fee_config.payment_address.to_string(),
            amount: vec![coin(coin1_fee.u128(), swap.coin1.coin.denom.clone())],
        }));
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: swap.user2.to_string(),
            amount: vec![coin(
                swap.coin1.coin.amount.checked_sub(coin1_fee)?.u128(),
                swap.coin1.coin.denom,
            )],
        }));
    } else {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: swap.coin1.cw20_address.as_ref().unwrap().to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: fee_config.payment_address.to_string(),
                amount: coin1_fee,
            })?,
            funds: vec![],
        }));
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: swap.coin1.cw20_address.unwrap(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: swap.user2.to_string(),
                amount: swap.coin1.coin.amount.checked_sub(coin1_fee)?,
            })?,
            funds: vec![],
        }))
    }

    if swap.coin2.is_native {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: fee_config.payment_address.to_string(),
            amount: vec![coin(coin2_fee.u128(), swap.coin2.coin.denom.clone())],
        }));
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: swap.user1.to_string(),
            amount: vec![coin(
                swap.coin2.coin.amount.checked_sub(coin2_fee)?.u128(),
                swap.coin2.coin.denom,
            )],
        }));
    } else {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: swap.coin2.cw20_address.as_ref().unwrap().to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: fee_config.payment_address.to_string(),
                amount: coin2_fee,
            })?,
            funds: vec![],
        }));
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: swap.coin2.cw20_address.unwrap(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: swap.user1.to_string(),
                amount: swap.coin2.coin.amount.checked_sub(coin2_fee)?,
            })?,
            funds: vec![],
        }))
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "accept"))
}

fn _cancel(deps: DepsMut, swap: Swap) -> Result<Response, ContractError> {
    LOCK.save(deps.storage, &true)?;

    let msg: CosmosMsg;

    if swap.coin1.is_native {
        msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: swap.user1.to_string(),
            amount: vec![swap.coin1.coin.clone()],
        });
    } else {
        msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: swap.coin1.cw20_address.unwrap(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: swap.user1.to_string(),
                amount: swap.coin1.coin.amount,
            })?,
            funds: vec![],
        })
    }

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "cancel"))
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
