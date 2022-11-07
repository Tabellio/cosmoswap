use cosmoswap_packages::types::FeeInfo;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub admin: Addr,
}
pub const CONFIG: Item<Config> = Item::new("config");

pub const FEE_CONFIG: Item<FeeInfo> = Item::new("fee_config");

// Swap represents an exchange that is pre defined by the user
// User1 is the user who created the swap
// User2 is the user who accepted the swap
// Coin1 is the coin that user1 is offering
// Coin2 is the coin that user1 wants
#[cw_serde]
pub struct Swap {
    pub user1: Addr,
    pub user2: Addr,
    pub coin1: Coin,
    pub coin2: Coin,
}
pub const SWAP: Item<Swap> = Item::new("swap");

pub const LOCK: Item<bool> = Item::new("lock");
