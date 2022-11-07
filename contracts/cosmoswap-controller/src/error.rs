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

    #[error("{0}")]
    Funds(#[from] FundsError),
}
