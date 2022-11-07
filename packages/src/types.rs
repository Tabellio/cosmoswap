use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal};

#[cw_serde]
pub struct FeeInfo {
    pub percentage: Decimal,
    pub payment_address: Addr,
}

#[cw_serde]
pub struct SwapInfo {
    pub user1: String,
    pub user2: String,
    pub coin1: SwapCoin,
    pub coin2: SwapCoin,
}

#[cw_serde]
pub struct SwapCoin {
    pub is_native: bool,
    pub coin: Coin,
    pub cw20_address: Option<String>,
}
