use cosmoswap_packages::funds::FundsError;
use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Denoms cannot be the same")]
    SameDenoms {},

    #[error("Invalid cw20 address")]
    InvalidCw20Addr {},

    #[error("Invalid expiration time")]
    InvalidExpiration {},

    #[error("{0}")]
    Funds(#[from] FundsError),
}
