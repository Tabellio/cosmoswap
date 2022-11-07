use cosmoswap_packages::types::FeeInfo;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub cosmoswap_code_id: u64,
}
pub const CONFIG: Item<Config> = Item::new("config");

pub const FEE_CONFIG: Item<FeeInfo> = Item::new("fee_config");
