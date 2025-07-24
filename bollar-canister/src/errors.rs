//! Error handling for Bollar Money protocol

use crate::types::*;

/// Protocol error types
#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolError {
    /// Insufficient collateral for the requested operation
    InsufficientCollateral {
        required: u32,
        actual: u32,
    },
    
    /// CDP not found with given ID
    CDPNotFound(u64),
    
    /// CDP has already been liquidated
    CDPAlreadyLiquidated(u64),
    
    /// Amount provided is too small
    AmountTooSmall(u64, u64), // actual, minimum
    
    /// Invalid amount provided
    InvalidAmount,
    
    /// Unauthorized access to operation
    UnauthorizedAccess,
    
    /// Oracle price error
    OraclePriceError(String),
    
    /// Runes operation failed
    RunesOperationFailed(String),
    
    /// Invalid BTC address format
    InvalidAddress(String),
    
    /// Insufficient balance for operation
    InsufficientBalance(u64, u64), // requested, available
    
    /// Invalid state transition
    InvalidState(String),
    
    /// Math overflow/underflow
    MathOverflow,
    
    /// Parameter validation error
    ValidationError(String),
}

impl core::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProtocolError::InsufficientCollateral { required, actual } => {
                write!(f, "Insufficient collateral: required {}%, actual {}%", required, actual)
            }
            ProtocolError::CDPNotFound(id) => {
                write!(f, "CDP with ID {} not found", id)
            }
            ProtocolError::CDPAlreadyLiquidated(id) => {
                write!(f, "CDP with ID {} has already been liquidated", id)
            }
            ProtocolError::AmountTooSmall(actual, minimum) => {
                write!(f, "Amount {} is below minimum {}", actual, minimum)
            }
            ProtocolError::InvalidAmount => {
                write!(f, "Invalid amount provided")
            }
            ProtocolError::UnauthorizedAccess => {
                write!(f, "Unauthorized access to operation")
            }
            ProtocolError::OraclePriceError(msg) => {
                write!(f, "Oracle price error: {}", msg)
            }
            ProtocolError::RunesOperationFailed(msg) => {
                write!(f, "Runes operation failed: {}", msg)
            }
            ProtocolError::InvalidAddress(msg) => {
                write!(f, "Invalid BTC address: {}", msg)
            }
            ProtocolError::InsufficientBalance(requested, available) => {
                write!(f, "Insufficient balance: requested {}, available {}", requested, available)
            }
            ProtocolError::InvalidState(msg) => {
                write!(f, "Invalid state: {}", msg)
            }
            ProtocolError::MathOverflow => {
                write!(f, "Math operation overflow")
            }
            ProtocolError::ValidationError(msg) => {
                write!(f, "Validation error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ProtocolError {}

// Conversion to Candid for API responses
impl From<ProtocolError> for crate::types::ProtocolError {
    fn from(error: ProtocolError) -> Self {
        match error {
            ProtocolError::InsufficientCollateral { required, actual } => {
                crate::types::ProtocolError::InsufficientCollateral { required, actual }
            }
            ProtocolError::CDPNotFound(id) => {
                crate::types::ProtocolError::CDPNotFound(id)
            }
            ProtocolError::CDPAlreadyLiquidated(id) => {
                crate::types::ProtocolError::CDPAlreadyLiquidated(id)
            }
            ProtocolError::AmountTooSmall(actual, minimum) => {
                crate::types::ProtocolError::AmountTooSmall(actual, minimum)
            }
            ProtocolError::InvalidAmount => {
                crate::types::ProtocolError::InvalidAmount
            }
            ProtocolError::UnauthorizedAccess => {
                crate::types::ProtocolError::UnauthorizedAccess
            }
            ProtocolError::OraclePriceError(msg) => {
                crate::types::ProtocolError::OraclePriceError(msg)
            }
            ProtocolError::RunesOperationFailed(msg) => {
                crate::types::ProtocolError::RunesOperationFailed(msg)
            }
            ProtocolError::InvalidAddress(msg) => {
                crate::types::ProtocolError::InvalidAddress(msg)
            }
            _ => {
                crate::types::ProtocolError::ValidationError("Unknown error".to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = ProtocolError::CDPNotFound(123);
        assert_eq!(error.to_string(), "CDP with ID 123 not found");
    }

    #[test]
    fn test_error_conversions() {
        let error = ProtocolError::InvalidAddress("Invalid format".to_string());
        let candid_error: crate::types::ProtocolError = error.into();
        
        match candid_error {
            crate::types::ProtocolError::InvalidAddress(_) => assert!(true),
            _ => assert!(false, "Unexpected error type"),
        }
    }
}