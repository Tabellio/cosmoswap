use crate::state::Swap;
use cosmoswap_packages::types::{FeeInfo, SwapInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub fee_info: FeeInfo,
    pub swap_info: SwapInfo,
}

#[cw_serde]
pub enum ExecuteMsg {
    Accept {},
    Cancel {},
    Receive(Cw20ReceiveMsg),
}

#[cw_serde]
pub enum ReceiveMsg {
    Accept {},
    Cancel {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Swap)]
    Swap {},
}
