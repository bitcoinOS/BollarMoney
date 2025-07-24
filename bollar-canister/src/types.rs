// Core types and data structures for Bollar Money protocol
use candid::{CandidType, Deserialize};
use serde::{Deserialize as SerdeDeserialize, Serialize};

// 使用虚拟Principal，因为测试环境没有真实的
#[cfg(test)]
mod test_utils {
    pub fn mock_principal() -> candid::Principal {
        candid::Principal::anonymous()
    }
}

#[cfg(not(test))]
use ic_cdk::api::caller;

/// System configuration parameters
#[derive(CandidType, Serialize, SerdeDeserialize, Clone, Debug)]
pub struct SystemConfig {
    pub max_collateral_ratio: u32,      // Basis points (9000 = 90%)
    pub liquidation_threshold: u32,     // Basis points (8500 = 85%)
    pub liquidation_penalty: u32,       // Basis points (500 = 5%)
    pub min_collateral_amount: u64,     // Minimum BTC amount in satoshis
    pub min_mint_amount: u64,           // Minimum Bollar amount in cents
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            max_collateral_ratio: 9000,     // 90%
            liquidation_threshold: 8500,    // 85%
            liquidation_penalty: 500,       // 5%
            min_collateral_amount: 100_000, // 0.001 BTC in satoshis
            min_mint_amount: 1_000,         // $0.01 in cents
        }
    }
}

/// Collateralized Debt Position (CDP) structure
#[derive(CandidType, Serialize, SerdeDeserialize, Clone, Debug)]
pub struct CDP {
    pub id: u64,
    pub owner: Principal,
    pub collateral_amount: u64,     // BTC amount in satoshis
    pub minted_amount: u64,         // Bollar amount in cents
    pub created_at: u64,            // Timestamp
    pub updated_at: u64,            // Last update timestamp
    pub is_liquidated: bool,
}

impl CDP {
    /// Calculate current collateral ratio given BTC price
    pub fn calculate_collateral_ratio(&self, btc_price_cents: u64) -> u32 {
        if self.minted_amount == 0 {
            return u32::MAX; // Infinite collateralization for zero debt
        }
        
        let collateral_value_cents = self.collateral_amount * btc_price_cents / 100_000_000; // Convert satoshis to USD cents
        (collateral_value_cents * 10_000 / self.minted_amount) as u32 // Basis points
    }
    
    /// Check if CDP is eligible for liquidation
    pub fn should_liquidate(&self, btc_price_cents: u64, threshold: u32) -> bool {
        if self.minted_amount == 0 {
            return false;
        }
        self.calculate_collateral_ratio(btc_price_cents) < threshold
    }
    
    /// Calculate maximum mintable amount for this CDP
    pub fn max_mintable_amount(&self, btc_price_cents: u64, max_ratio: u32) -> u64 {
        let collateral_value_cents = self.collateral_amount * btc_price_cents / 100_000_000;
        let max_debt = collateral_value_cents * max_ratio as u64 / 10_000;
        max_debt.saturating_sub(self.minted_amount)
    }
}

/// Oracle price data structure
#[derive(CandidType, Serialize, SerdeDeserialize, Clone, Debug)]
pub struct PriceData {
    pub price_cents: u64,      // BTC price in USD cents
    pub timestamp: u64,        // Unix timestamp
    pub source: String,        // Data source identifier
    pub confidence: u8,        // Confidence level (0-100)
}

/// Deposit request structure
#[derive(CandidType, Serialize, SerdeDeserialize, Debug)]
pub struct DepositRequest {
    pub btc_address: String,
    pub amount_satoshis: u64,
}

/// Mint request structure
#[derive(CandidType, Serialize, SerdeDeserialize, Debug)]
pub struct MintRequest {
    pub cdp_id: u64,
    pub amount_cents: u64,
}

/// Liquidation info for display
#[derive(CandidType, Serialize, SerdeDeserialize, Debug)]
pub struct LiquidationInfo {
    pub cdp_id: u64,
    pub owner: Principal,
    pub collateral_amount: u64,
    pub minted_amount: u64,
    pub current_ratio: u32,
    pub liquidation_reward: u64,
}

/// System health summary
#[derive(CandidType, Serialize, SerdeDeserialize, Debug)]
pub struct SystemHealth {
    pub total_collateral_satoshis: u64,
    pub total_minted_cents: u64,
    pub average_collateral_ratio: u32,
    pub active_cdps_count: u64,
    pub btc_price_cents: u64,
    pub system_utilization_ratio: u32, // Basis points
}

impl SystemHealth {
    pub fn utilization_ratio(&self) -> u32 {
        if self.total_collateral_satoshis == 0 {
            return 0;
        }
        let total_collateral_cents = self.total_collateral_satoshis * self.btc_price_cents / 100_000_000;
        (self.total_minted_cents * 10_000 / total_collateral_cents) as u32
    }
}

/// Error types for the protocol
#[derive(CandidType, Serialize, SerdeDeserialize, Debug)]
pub enum ProtocolError {
    InsufficientCollateral {
        required: u32,
        actual: u32,
    },
    CDPNotFound(u64),
    CDPAlreadyLiquidated(u64),
    AmountTooSmall(u64, u64), // actual, minimum
    InvalidAmount,
    UnauthorizedAccess,
    OraclePriceError(String),
    RunesOperationFailed(String),
}

/// Response type for API calls
#[derive(CandidType, Serialize, SerdeDeserialize, Debug)]
pub enum ApiResponse<T> {
    Success(T),
    Error(ProtocolError),
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        ApiResponse::Success(data)
    }
    
    pub fn error(error: ProtocolError) -> Self {
        ApiResponse::Error(error)
    }
}

/// Configuration for price validation
#[derive(CandidType, Serialize, SerdeDeserialize, Debug)]
pub struct PriceValidationConfig {
    pub max_price_change_percent: u32, // Max allowed price change from previous
    pub max_age_seconds: u64,          // Max age of price data
    pub min_confidence_level: u8,      // Minimum confidence required
}

impl Default for PriceValidationConfig {
    fn default() -> Self {
        Self {
            max_price_change_percent: 10, // 10% max change
            max_age_seconds: 300,         // 5 minutes max age
            min_confidence_level: 95,     // 95% minimum confidence
        }
    }
}

/// Mint statistics for analytics
#[derive(CandidType, Serialize, SerdeDeserialize, Debug)]
pub struct MintStats {
    pub total_bollar_minted: u64,
    pub total_btc_collateral: u64,
    pub average_collateral_ratio: u32,
    pub unique_users_count: u64,
    pub last_update_timestamp: u64,
}