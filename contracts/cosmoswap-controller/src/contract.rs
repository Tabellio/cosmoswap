#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    WasmMsg,
};
use cw2::set_contract_version;

use cosmoswap::msg::InstantiateMsg as CosmoswapInstantiateMsg;
use cosmoswap_packages::funds::check_single_coin;
use cosmoswap_packages::types::{FeeInfo, SwapInfo};
use cw20::Cw20ReceiveMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG, FEE_CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmoswap-controller";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
        cosmoswap_code_id: 0,
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
        ExecuteMsg::CreateSwap { swap_info } => execute_create_swap(deps, env, info, swap_info),
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
    _env: Env,
    info: MessageInfo,
    swap_info: SwapInfo,
) -> Result<Response, ContractError> {
    if info.sender.to_string() != swap_info.user1 {
        return Err(ContractError::Unauthorized {});
    }

    let config = CONFIG.load(deps.storage)?;
    let fee_config = FEE_CONFIG.load(deps.storage)?;

    if swap_info.coin1.coin.denom == swap_info.coin2.coin.denom {
        return Err(ContractError::SameDenoms {});
    }

    check_single_coin(&info, &swap_info.coin1.coin)?;

    let msg = WasmMsg::Instantiate {
        code_id: config.cosmoswap_code_id,
        msg: to_binary(&CosmoswapInstantiateMsg {
            fee_info: fee_config,
            swap_info,
        })?,
        funds: info.funds,
        admin: None,
        label: "Cosmoswap Contract".to_string(),
    };

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "create_swap"))
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
