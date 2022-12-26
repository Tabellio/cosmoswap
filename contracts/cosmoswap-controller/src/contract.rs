use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;

use cosmoswap::msg::InstantiateMsg as CosmoswapInstantiateMsg;
use cosmoswap_packages::funds::{check_single_coin, FundsError};
use cosmoswap_packages::types::{FeeInfo, SwapInfo};
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, Cw20ReceiveMsg, Expiration, TokenInfoResponse};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{Config, CONFIG, FEE_CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmoswap-controller";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Save the code id cosmoswap contract instantiation
    let config = Config {
        admin: info.sender,
        cosmoswap_code_id: msg.cosmoswap_code_id,
    };
    CONFIG.save(deps.storage, &config)?;

    // Save the fee config for setting it on cosmoswap contract instantiation
    let fee_config = FeeInfo {
        percentage: msg.fee_percentage,
        payment_address: deps.api.addr_validate(&msg.fee_payment_address)?,
    };
    FEE_CONFIG.save(deps.storage, &fee_config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", config.admin)
        .add_attribute("cosmoswap_code_id", config.cosmoswap_code_id.to_string())
        .add_attribute("fee_percentage", fee_config.percentage.to_string())
        .add_attribute("fee_percentage", fee_config.payment_address.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { cosmoswap_code_id } => {
            execute_update_config(deps, env, info, cosmoswap_code_id)
        }
        ExecuteMsg::UpdateFeeConfig {
            fee_percentage,
            fee_payment_address,
        } => execute_update_fee_config(deps, env, info, fee_percentage, fee_payment_address),
        ExecuteMsg::CreateSwap {
            swap_info,
            expiration,
        } => {
            if info.sender.to_string() != swap_info.user1 {
                return Err(ContractError::Unauthorized {});
            }
            execute_create_swap(deps, env, info, swap_info, expiration)
        }
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
    }
}

fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    cosmoswap_code_id: u64,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    config.cosmoswap_code_id = cosmoswap_code_id;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("cosmoswap_code_id", config.cosmoswap_code_id.to_string()))
}

fn execute_update_fee_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    fee_percentage: Decimal,
    fee_payment_address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let fee_config = FeeInfo {
        percentage: fee_percentage,
        payment_address: deps.api.addr_validate(&fee_payment_address)?,
    };
    FEE_CONFIG.save(deps.storage, &fee_config)?;

    Ok(Response::new()
        .add_attribute("action", "update_fee_config")
        .add_attribute("fee_percentage", fee_config.percentage.to_string())
        .add_attribute(
            "fee_payment_address",
            fee_config.payment_address.to_string(),
        ))
}

fn execute_create_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    swap_info: SwapInfo,
    expiration: Expiration,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let fee_config = FEE_CONFIG.load(deps.storage)?;

    if expiration.is_expired(&env.block) {
        return Err(ContractError::InvalidExpiration {});
    }

    if swap_info.coin1.coin.denom == swap_info.coin2.coin.denom {
        return Err(ContractError::SameDenoms {});
    }

    if swap_info.coin1.is_native {
        check_single_coin(&info, &swap_info.coin1.coin)?;
    };

    let wasm_msg = WasmMsg::Instantiate {
        code_id: config.cosmoswap_code_id,
        msg: to_binary(&CosmoswapInstantiateMsg {
            fee_info: fee_config,
            swap_info: swap_info.clone(),
            expiration,
        })?,
        funds: info.funds.clone(),
        admin: None,
        label: "Cosmoswap Contract".to_string(),
    };

    let msg = match swap_info.coin1.is_native {
        true => {
            check_single_coin(&info, &swap_info.coin1.coin)?;
            SubMsg::new(wasm_msg)
        }
        false => SubMsg::reply_on_success(wasm_msg, INSTANTIATE_REPLY_ID),
    };

    Ok(Response::new()
        .add_submessage(msg)
        .add_attribute("action", "create_swap"))
}

fn execute_receive(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    cw20_recieve_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg: ReceiveMsg = from_binary(&cw20_recieve_msg.msg)?;
    match msg {
        ReceiveMsg::CreateSwap {
            swap_info,
            expiration,
        } => {
            // Check if the sender is the same as the user1
            if cw20_recieve_msg.sender != swap_info.user1 {
                return Err(ContractError::Unauthorized {});
            };

            // Check if coins are not native and the cw20 info is correct
            if !swap_info.coin1.is_native {
                if swap_info.coin1.cw20_address.is_none() {
                    return Err(ContractError::InvalidCw20Addr {});
                };
                let res: TokenInfoResponse = deps.querier.query_wasm_smart(
                    swap_info.coin1.cw20_address.as_ref().unwrap(),
                    &Cw20QueryMsg::TokenInfo {},
                )?;
                if res.symbol != swap_info.coin1.coin.denom {
                    return Err(FundsError::InvalidDenom {
                        got: swap_info.coin1.coin.denom,
                        expected: res.symbol,
                    }
                    .into());
                };
                if cw20_recieve_msg.amount != swap_info.coin1.coin.amount {
                    return Err(FundsError::InvalidFunds {
                        got: cw20_recieve_msg.amount.to_string(),
                        expected: swap_info.coin1.coin.amount.to_string(),
                    }
                    .into());
                };
            };
            if !swap_info.coin2.is_native {
                if swap_info.coin2.cw20_address.is_none() {
                    return Err(ContractError::InvalidCw20Addr {});
                };
                let res: TokenInfoResponse = deps.querier.query_wasm_smart(
                    swap_info.coin2.cw20_address.as_ref().unwrap(),
                    &Cw20QueryMsg::TokenInfo {},
                )?;
                if res.symbol != swap_info.coin2.coin.denom {
                    return Err(FundsError::InvalidDenom {
                        got: swap_info.coin2.coin.denom,
                        expected: res.symbol,
                    }
                    .into());
                };
            };

            execute_create_swap(deps, _env, info, swap_info, expiration)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps, env)?),
        QueryMsg::FeeConfig {} => to_binary(&query_fee_config(deps, env)?),
    }
}

fn query_config(deps: Deps, _env: Env) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

fn query_fee_config(deps: Deps, _env: Env) -> StdResult<FeeInfo> {
    let fee_config = FEE_CONFIG.load(deps.storage)?;
    Ok(fee_config)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.id != INSTANTIATE_REPLY_ID {
        return Err(ContractError::Unauthorized {});
    }

    let res = msg.result.into_result();
    if res.is_err() {
        return Err(ContractError::SwapInstantiateError {});
    };

    let sub_msg_response = res.unwrap();

    let mut contract_addr = String::from("");
    let mut coin1_cw20_addr = String::from("");
    let mut amount = Uint128::zero();

    sub_msg_response.events.iter().for_each(|e| {
        if e.ty == "wasm" {
            e.attributes.iter().for_each(|attr| {
                if attr.key == "_contract_address" {
                    contract_addr = attr.value.clone();
                };
                if attr.key == "coin1_cw20_address" {
                    coin1_cw20_addr = attr.value.clone();
                };
                if attr.key == "coin1_amount" {
                    amount = Uint128::from_str(&attr.value).unwrap();
                };
            });
        }
    });

    let msg = WasmMsg::Execute {
        contract_addr: coin1_cw20_addr,
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: contract_addr,
            amount,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "cosmoswap_instantiate_reply"))
}
