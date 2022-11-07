use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Swap is not active")]
    SwapLocked {},

    #[error("Cw20 address is not valid")]
    InvalidCw20Addr {},

    #[error("Funds are not found")]
    FundsNotFound {},

    #[error("Denom is not the same")]
    InvalidDenom {},

    #[error("Amount is not the same")]
    InvalidAmount {},

    #[error("{0}")]
    Owerflow(#[from] OverflowError),
}
