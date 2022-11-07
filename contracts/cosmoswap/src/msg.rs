use crate::state::Swap;
use cosmoswap_packages::types::{FeeInfo, SwapInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {
    pub fee_info: FeeInfo,
    pub swap_info: SwapInfo,
    // pub is_native: bool,
    // pub cw20_addr: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Accept {},
    Cancel {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Swap)]
    Swap {},
}
