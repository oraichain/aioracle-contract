use cosmwasm_std::StdError;
use hex::FromHexError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Hex(#[from] FromHexError),

    #[error("Unauthorized")]
    Unauthorized {},
    #[error("Service name exists. Cannot add new")]
    ServiceExists {},
    #[error("Insufficient funds")]
    InsufficientFunds {},
    #[error("Already submitted")]
    AlreadySubmitted {},

    #[error("No request to process")]
    NoRequest {},

    #[error("Invalid input")]
    InvalidInput {},
    #[error("Invalid threshold")]
    InvalidThreshold {},
    #[error("Invalid signature")]
    InvalidSignature {},

    #[error("Request already finished")]
    AlreadyFinished {},

    #[error("Service not found")]
    ServiceNotFound {},

    #[error("Wrong length")]
    WrongLength {},

    #[error("Verification failed")]
    VerificationFailed {},

    #[error("Cannot migrate from different contract type: {previous_contract}")]
    CannotMigrate { previous_contract: String },
}
