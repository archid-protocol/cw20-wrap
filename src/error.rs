use cosmwasm_std::StdError;
use cw20_base::ContractError as Cw20ContractError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Cw20(#[from] Cw20ContractError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("No {denom} tokens sent")]
    InvalidDeposit { denom: String },

    #[error("{withdrawal} exceeds balance of {balance}")]
    InvalidWithdrawal { withdrawal: String, balance: String },
}
