use cosmoswap_packages::types::{FeeInfo, SwapCoin};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw20::Expiration;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub expiration: Expiration,
}
pub const CONFIG: Item<Config> = Item::new("config");

pub const FEE_CONFIG: Item<FeeInfo> = Item::new("fee_config");

#[cw_serde]
pub struct Swap {
    pub user1: Addr,
    pub user2: Addr,
    pub coin1: SwapCoin,
    pub coin2: SwapCoin,
}
pub const SWAP: Item<Swap> = Item::new("swap");

pub const LOCK: Item<bool> = Item::new("lock");
