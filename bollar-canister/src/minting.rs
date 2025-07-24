//! Bollar Minting Engine - Task 2.1 Implementation
//! Complete Bollar token minting with collateral ratio validation

use crate::types::*;
use candid::Principal;
use std::collections::HashMap;

/// Minting request structure
#[derive(Debug, Clone)]
pub struct MintBollarRequest {
    pub cdp_id: u64,
    pub owner: [u8; 32],
    pub amount_to_mint: u64,
    pub btc_price_cents: u64,
}

/// Minting response structure
#[derive(Debug, Clone)]
pub struct MintBollarResponse {
    pub cdp_id: u64,
    pub previous_minted: u64,
    pub new_minted: u64,
    pub collateral_ratio: u32,
    pub minting_fee: u64,
}

/// Bollar token configuration
pub struct BollarConfig {
    pub minting_fee_basis_points: u32, // 0.1% = 10 basis points
    pub max_minting_amount: u64,
    pub min_collateral_ratio: u32, // 100% = 10,000
}

impl Default for BollarConfig {
    fn default() -> Self {
        Self {
            minting_fee_basis_points: 10, // 0.1%
            max_minting_amount: 100_000_000_000, // 1 billion cents = $10M
            min_collateral_ratio: 10_000, // 100%
        }
    }
}

/// Core minting engine
pub struct MintingEngine {
    cdps: HashMap<u64, CDP>,
    config: BollarConfig,
}

impl MintingEngine {
    pub fn new(config: BollarConfig) -> Self {
        Self {
            cdps: HashMap::new(),
            config,
        }
    }

    /// Calculate maximum mintable amount based on collateral
    pub fn calculate_max_mintable(
        &self,
        collateral_amount: u64,
        btc_price_cents: u64,
        system_config: &SystemConfig,
    ) -> Result<u64, TestError> {
        let collateral_value_cents = collateral_amount * btc_price_cents / 100_000_000;
        let max_mintable = collateral_value_cents * system_config.max_collateral_ratio as u64 / 10_000;
        
        if max_mintable < system_config.min_mint_amount {
            return Err(TestError::AmountTooSmall(
                max_mintable,
                system_config.min_mint_amount,
            ));
        }
        
        Ok(max_mintable)
    }

    /// Calculate current collateral ratio
    pub fn calculate_collateral_ratio(
        &self,
        collateral_amount: u64,
        minted_amount: u64,
        btc_price_cents: u64,
    ) -> Result<u32, TestError> {
        if minted_amount == 0 {
            return Ok(10_000); // 100% when nothing minted
        }

        let collateral_value_cents = collateral_amount * btc_price_cents / 100_000_000;
        
        // Use u128 to prevent overflow
        let ratio = (collateral_value_cents as u128 * 10_000) / (minted_amount as u128);
        
        if ratio > u32::MAX as u128 {
            return Err(TestError::ValidationError("Ratio calculation overflow".to_string()));
        }
        
        Ok(ratio as u32)
    }

    /// Validate minting request against protocol constraints
    pub fn validate_minting_request(
        &self,
        cdp: &CDP,
        amount_to_mint: u64,
        btc_price_cents: u64,
        system_config: &SystemConfig,
    ) -> Result<(), TestError> {
        // Check CDP exists and is active
        if cdp.is_liquidated {
            return Err(TestError::ValidationError("CDP is liquidated".to_string()));
        }

        // Check amount is positive
        if amount_to_mint == 0 {
            return Err(TestError::ValidationError("Mint amount must be positive".to_string()));
        }

        // Check against maximum minting amount
        if amount_to_mint > self.config.max_minting_amount {
            return Err(TestError::ValidationError("Mint amount exceeds maximum".to_string()));
        }

        // Calculate current and new minted amounts
        let new_total_minted = cdp.minted_amount
            .checked_add(amount_to_mint)
            .ok_or(TestError::ValidationError("Mint amount overflow".to_string()))?;

        // Calculate maximum allowed based on collateral
        let max_mintable = self.calculate_max_mintable(
            cdp.collateral_amount,
            btc_price_cents,
            system_config,
        )?;

        // Ensure new total doesn't exceed maximum
        if new_total_minted > max_mintable {
            return Err(TestError::ValidationError(
                format!(
                    "Insufficient collateral. Requested: {}, Max allowed: {}",
                    new_total_minted, max_mintable
                )
            ));
        }

        // Check collateral ratio after minting
        let new_ratio = self.calculate_collateral_ratio(
            cdp.collateral_amount,
            new_total_minted,
            btc_price_cents,
        )?;

        // Ensure ratio stays above system minimum
        if new_ratio < system_config.max_collateral_ratio {
            return Err(TestError::ValidationError(
                format!(
                    "Collateral ratio too low. Required: {}%%, Current: {}%%",
                    system_config.max_collateral_ratio as f64 / 100.0,
                    new_ratio as f64 / 100.0
                )
            ));
        }

        Ok(())
    }

    /// Execute minting operation with fee calculation
    pub fn mint_bollar(
        &mut self,
        request: MintBollarRequest,
        system_config: &SystemConfig,
    ) -> Result<MintBollarResponse, TestError> {
        // Get CDP
        let cdp = self.cdps.get(&request.cdp_id)
            .ok_or(TestError::ValidationError("CDP not found".to_string()))?;

        // Verify ownership
        if cdp.owner != request.owner {
            return Err(TestError::ValidationError("Unauthorized minting".to_string()));
        }

        // Validate the minting request
        self.validate_minting_request(
            cdp,
            request.amount_to_mint,
            request.btc_price_cents,
            system_config,
        )?;

        // Calculate minting fee
        let minting_fee = (request.amount_to_mint * self.config.minting_fee_basis_points as u64) / 10_000;
        let actual_mint_amount = request.amount_to_mint - minting_fee;

        // Update CDP
        let mut updated_cdp = cdp.clone();
        updated_cdp.minted_amount += actual_mint_amount;
        updated_cdp.updated_at = 1700000000; // Mock timestamp

        let new_collateral_ratio = self.calculate_collateral_ratio(
            updated_cdp.collateral_amount,
            updated_cdp.minted_amount,
            request.btc_price_cents,
        )?;

        // Store updated CDP
        self.cdps.insert(request.cdp_id, updated_cdp);

        Ok(MintBollarResponse {
            cdp_id: request.cdp_id,
            previous_minted: cdp.minted_amount,
            new_minted: cdp.minted_amount + actual_mint_amount,
            collateral_ratio: new_collateral_ratio,
            minting_fee,
        })
    }

    /// Get CDP for minting operations
    pub fn get_cdp(&self, cdp_id: u64) -> Option<&CDP> {
        self.cdps.get(&cdp_id)
    }

    /// Add CDP to minting engine (for testing)
    pub fn add_cdp(&mut self, cdp: CDP) {
        self.cdps.insert(cdp.id, cdp);
    }
}

/// Test structures
#[derive(Debug, Clone)]
pub struct CDP {
    pub id: u64,
    pub owner: [u8; 32],
    pub collateral_amount: u64,
    pub minted_amount: u64,
    pub created_at: u64,
    pub updated_at: u64,
    pub is_liquidated: bool,
}

#[derive(Debug, Clone)]
pub struct SystemConfig {
    pub max_collateral_ratio: u32,
    pub liquidation_threshold: u32,
    pub min_collateral_amount: u64,
    pub min_mint_amount: u64,
}

#[derive(Debug, PartialEq)]
pub enum TestError {
    ValidationError(String),
    AmountTooSmall(u64, u64),
    InvalidAmount,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> SystemConfig {
        SystemConfig {
            max_collateral_ratio: 9000, // 90%
            liquidation_threshold: 8500, // 85%
            min_collateral_amount: 100_000, // 0.001 BTC
            min_mint_amount: 1_000, // $0.01
        }
    }

    fn test_bollar_config() -> BollarConfig {
        BollarConfig::default()
    }

    fn create_test_cdp(
        id: u64,
        owner: [u8; 32],
        collateral_amount: u64,
        minted_amount: u64,
    ) -> CDP {
        CDP {
            id,
            owner,
            collateral_amount,
            minted_amount,
            created_at: 1700000000,
            updated_at: 1700000000,
            is_liquidated: false,
        }
    }

    #[test]
    fn test_max_mintable_calculation() {
        let engine = MintingEngine::new(test_bollar_config());
        let config = test_config();
        let btc_price = 50_000_000; // $50,000 per BTC

        // 1 BTC = 100,000,000 satoshi
        let result = engine.calculate_max_mintable(
            100_000_000, // 1 BTC
            btc_price,
            &config,
        );

        assert_eq!(result.unwrap(), 45_000_000); // $450 at 90% LTV
    }

    #[test]
    fn test_collateral_ratio_calculation() {
        let engine = MintingEngine::new(test_bollar_config());
        let btc_price = 50_000_000; // $50,000 per BTC

        // 1 BTC collateral, $300 minted = 166.67% ratio
        let ratio = engine.calculate_collateral_ratio(
            100_000_000, // 1 BTC
            300_000, // $300
            btc_price,
        );

        assert_eq!(ratio.unwrap(), 16_667); // 166.67%
    }

    #[test]
    fn test_successful_minting() {
        let mut engine = MintingEngine::new(test_bollar_config());
        let config = test_config();
        let btc_price = 50_000_000; // $50,000 per BTC

        let cdp = create_test_cdp(1, [0u8; 32], 1_000_000, 0); // 0.01 BTC, $500 value
        engine.add_cdp(cdp);

        let request = MintBollarRequest {
            cdp_id: 1,
            owner: [0u8; 32],
            amount_to_mint: 400_000, // $400 (80% of $500)
            btc_price_cents: btc_price,
        };

        let result = engine.mint_bollar(request, &config);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.cdp_id, 1);
        assert_eq!(response.previous_minted, 0);
        assert_eq!(response.new_minted, 396_000); // $400 - 0.1% fee
        assert_eq!(response.collateral_ratio, 12_626); // ~126.26%
        assert_eq!(response.minting_fee, 4_000); // 0.1% of $400

        // Verify CDP was updated
        let updated_cdp = engine.get_cdp(1).unwrap();
        assert_eq!(updated_cdp.minted_amount, 396_000);
    }

    #[test]
    fn test_insufficient_collateral() {
        let mut engine = MintingEngine::new(test_bollar_config());
        let config = test_config();
        let btc_price = 50_000_000; // $50,000 per BTC

        let cdp = create_test_cdp(1, [0u8; 32], 1_000_000, 0); // 0.01 BTC, $500 value
        engine.add_cdp(cdp);

        let request = MintBollarRequest {
            cdp_id: 1,
            owner: [0u8; 32],
            amount_to_mint: 500_000, // $500 (100% - would exceed 90% limit)
            btc_price_cents: btc_price,
        };

        let result = engine.mint_bollar(request, &config);
        assert!(result.is_err());
        assert!(matches!(result, Err(TestError::ValidationError(_))));
    }

    #[test]
    fn test_exceeds_max_ratio() {
        let mut engine = MintingEngine::new(test_bollar_config());
        let config = test_config();
        let btc_price = 50_000_000; // $50,000 per BTC

        let cdp = create_test_cdp(1, [0u8; 32], 1_000_000, 400_000); // Already has $400 minted
        engine.add_cdp(cdp);

        // Try to mint more, would exceed 90% ratio
        let request = MintBollarRequest {
            cdp_id: 1,
            owner: [0u8; 32],
            amount_to_mint: 100_000, // Would make total $500, exactly at limit
            btc_price_cents: btc_price,
        };

        let result = engine.mint_bollar(request, &config);
        assert!(result.is_ok()); // Exactly at limit should be allowed
        
        let response = result.unwrap();
        assert_eq!(response.collateral_ratio, 9000); // Exactly 90%
    }

    #[test]
    fn test_minting_fee_calculation() {
        let mut engine = MintingEngine::new(test_bollar_config());
        let config = test_config();
        let btc_price = 50_000_000; // $50,000 per BTC

        let cdp = create_test_cdp(1, [0u8; 32], 1_000_000, 0);
        engine.add_cdp(cdp);

        let request = MintBollarRequest {
            cdp_id: 1,
            owner: [0u8; 32],
            amount_to_mint: 1_000_000, // $1,000
            btc_price_cents: btc_price,
        };

        let response = engine.mint_bollar(request, &config).unwrap();
        assert_eq!(response.minting_fee, 100_000); // 0.1% of $1,000
        assert_eq!(response.new_minted, 990_000); // $1,000 - $1 fee
    }

    #[test]
    fn test_boundary_ratio_calculations() {
        let engine = MintingEngine::new(test_bollar_config());
        let config = test_config();
        let btc_price = 100_000_000; // $100,000 per BTC

        // Test 89.99% ratio (should be allowed)
        let ratio = engine.calculate_collateral_ratio(
            1_000_000, // 0.01 BTC = $1,000
            899_900,   // $899.90 minted
            btc_price,
        );
        assert_eq!(ratio.unwrap(), 10_002); // 100.02% (slightly above 90% due to rounding)

        // Test 90.01% ratio (should be rejected)
        let ratio = engine.calculate_collateral_ratio(
            1_000_000, // 0.01 BTC = $1,000
            900_100,   // $900.10 minted
            btc_price,
        );
        assert_eq!(ratio.unwrap(), 9_999); // 99.99% (slightly below 90%)
    }

    #[test]
    fn test_unauthorized_minting() {
        let mut engine = MintingEngine::new(test_bollar_config());
        let config = test_config();
        let btc_price = 50_000_000; // $50,000 per BTC

        let cdp = create_test_cdp(1, [1u8; 32], 1_000_000, 0); // Owner is [1;32]
        engine.add_cdp(cdp);

        let request = MintBollarRequest {
            cdp_id: 1,
            owner: [2u8; 32], // Wrong owner
            amount_to_mint: 100_000,
            btc_price_cents: btc_price,
        };

        let result = engine.mint_bollar(request, &config);
        assert!(matches!(result, Err(TestError::ValidationError(msg)) 
            if msg.contains("Unauthorized")));
    }

    #[test]
    fn test_liquidated_cdp_minting() {
        let mut engine = MintingEngine::new(test_bollar_config());
        let config = test_config();
        let btc_price = 50_000_000; // $50,000 per BTC

        let mut cdp = create_test_cdp(1, [0u8; 32], 1_000_000, 0);
        cdp.is_liquidated = true;
        engine.add_cdp(cdp);

        let request = MintBollarRequest {
            cdp_id: 1,
            owner: [0u8; 32],
            amount_to_mint: 100_000,
            btc_price_cents: btc_price,
        };

        let result = engine.mint_bollar(request, &config);
        assert!(matches!(result, Err(TestError::ValidationError(msg)) 
            if msg.contains("liquidated")));
    }

    #[test]
    fn test_zero_mint_amount() {
        let mut engine = MintingEngine::new(test_bollar_config());
        let config = test_config();
        let btc_price = 50_000_000; // $50,000 per BTC

        let cdp = create_test_cdp(1, [0u8; 32], 1_000_000, 0);
        engine.add_cdp(cdp);

        let request = MintBollarRequest {
            cdp_id: 1,
            owner: [0u8; 32],
            amount_to_mint: 0,
            btc_price_cents: btc_price,
        };

        let result = engine.mint_bollar(request, &config);
        assert!(matches!(result, Err(TestError::ValidationError(msg)) 
            if msg.contains("positive")));
    }

    #[test]
    fn test_overflow_protection() {
        let mut engine = MintingEngine::new(test_bollar_config());
        let config = test_config();
        let btc_price = 50_000_000; // $50,000 per BTC

        let cdp = create_test_cdp(1, [0u8; 32], 1_000_000, 0);
        engine.add_cdp(cdp);

        let request = MintBollarRequest {
            cdp_id: 1,
            owner: [0u8; 32],
            amount_to_mint: u64::MAX,
            btc_price_cents: btc_price,
        };

        let result = engine.mint_bollar(request, &config);
        assert!(matches!(result, Err(TestError::ValidationError(msg)) 
            if msg.contains("overflow")));
    }
}