use crate::state::Config;
use cosmoswap_packages::types::{FeeInfo, SwapInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Decimal;
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub fee_percentage: Decimal,
    pub fee_payment_address: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {
        cosmoswap_code_id: u64,
    },
    UpdateFeeConfig {
        fee_percentage: Decimal,
        fee_payment_address: String,
    },
    CreateSwap {
        swap_info: SwapInfo,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},
    #[returns(FeeInfo)]
    FeeConfig {},
}
