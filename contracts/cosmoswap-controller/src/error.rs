use cosmoswap_packages::funds::FundsError;
use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Error while instantiating swap contract")]
    SwapInstantiateError {},

    #[error("Denoms cannot be the same")]
    SameDenoms {},

    #[error("Users cannot be the same")]
    SameUsers {},

    #[error("Invalid cw20 address")]
    InvalidCw20Addr {},

    #[error("Invalid expiration time")]
    InvalidExpiration {},

    #[error("{0}")]
    Funds(#[from] FundsError),
}
