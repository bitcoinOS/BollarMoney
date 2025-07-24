#[cfg(test)]
mod minting_integration_tests {
    use super::*;
    use std::collections::HashMap;

    // Test structures
    #[derive(Debug, Clone)]
    pub struct SystemConfig {
        pub max_collateral_ratio: u32,
        pub liquidation_threshold: u32,
        pub min_collateral_amount: u64,
        pub min_mint_amount: u64,
    }

    impl Default for SystemConfig {
        fn default() -> Self {
            Self {
                max_collateral_ratio: 9000, // 90%
                liquidation_threshold: 8500, // 85%
                min_collateral_amount: 100_000, // 0.001 BTC
                min_mint_amount: 1_000, // $0.01
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct BollarConfig {
        pub minting_fee_basis_points: u32,
        pub max_minting_amount: u64,
        pub min_collateral_ratio: u32,
    }

    impl Default for BollarConfig {
        fn default() -> Self {
            Self {
                minting_fee_basis_points: 10, // 0.1%
                max_minting_amount: 100_000_000_000,
                min_collateral_ratio: 10_000,
            }
        }
    }

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
    pub struct MintBollarRequest {
        pub cdp_id: u64,
        pub owner: [u8; 32],
        pub amount_to_mint: u64,
        pub btc_price_cents: u64,
    }

    #[derive(Debug, Clone)]
    pub struct MintBollarResponse {
        pub cdp_id: u64,
        pub previous_minted: u64,
        pub new_minted: u64,
        pub collateral_ratio: u32,
        pub minting_fee: u64,
    }

    #[derive(Debug, Clone)]
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

        pub fn calculate_collateral_ratio(
            &self,
            collateral_amount: u64,
            minted_amount: u64,
            btc_price_cents: u64,
        ) -> Result<u32, TestError> {
            if minted_amount == 0 {
                return Ok(10_000);
            }

            let collateral_value_cents = collateral_amount * btc_price_cents / 100_000_000;
            let ratio = (collateral_value_cents as u128 * 10_000) / (minted_amount as u128);
            
            if ratio > u32::MAX as u128 {
                return Err(TestError::ValidationError("Ratio calculation overflow".to_string()));
            }
            
            Ok(ratio as u32)
        }

        pub fn validate_minting_request(
            &self,
            cdp: &CDP,
            amount_to_mint: u64,
            btc_price_cents: u64,
            system_config: &SystemConfig,
        ) -> Result<(), TestError> {
            if cdp.is_liquidated {
                return Err(TestError::ValidationError("CDP is liquidated".to_string()));
            }

            if amount_to_mint == 0 {
                return Err(TestError::ValidationError("Mint amount must be positive".to_string()));
            }

            if amount_to_mint > self.config.max_minting_amount {
                return Err(TestError::ValidationError("Mint amount exceeds maximum".to_string()));
            }

            let new_total_minted = cdp.minted_amount
                .checked_add(amount_to_mint)
                .ok_or(TestError::ValidationError("Mint amount overflow".to_string()))?;

            let max_mintable = self.calculate_max_mintable(
                cdp.collateral_amount,
                btc_price_cents,
                system_config,
            )?;

            if new_total_minted > max_mintable {
                return Err(TestError::ValidationError(
                    format!(
                        "Insufficient collateral. Requested: {}, Max allowed: {}",
                        new_total_minted, max_mintable
                    )
                ));
            }

            let new_ratio = self.calculate_collateral_ratio(
                cdp.collateral_amount,
                new_total_minted,
                btc_price_cents,
            )?;

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

        pub fn mint_bollar(
            &mut self,
            request: MintBollarRequest,
            system_config: &SystemConfig,
        ) -> Result<MintBollarResponse, TestError> {
            let cdp = self.cdps.get(&request.cdp_id)
                .ok_or(TestError::ValidationError("CDP not found".to_string()))?;

            if cdp.owner != request.owner {
                return Err(TestError::ValidationError("Unauthorized minting".to_string()));
            }

            self.validate_minting_request(
                cdp,
                request.amount_to_mint,
                request.btc_price_cents,
                system_config,
            )?;

            let minting_fee = (request.amount_to_mint * self.config.minting_fee_basis_points as u64) / 10_000;
            let actual_mint_amount = request.amount_to_mint - minting_fee;

            let mut updated_cdp = cdp.clone();
            updated_cdp.minted_amount += actual_mint_amount;
            updated_cdp.updated_at = 1700000000;

            let new_collateral_ratio = self.calculate_collateral_ratio(
                updated_cdp.collateral_amount,
                updated_cdp.minted_amount,
                request.btc_price_cents,
            )?;

            self.cdps.insert(request.cdp_id, updated_cdp);

            Ok(MintBollarResponse {
                cdp_id: request.cdp_id,
                previous_minted: cdp.minted_amount,
                new_minted: cdp.minted_amount + actual_mint_amount,
                collateral_ratio: new_collateral_ratio,
                minting_fee,
            })
        }

        pub fn add_cdp(&mut self, cdp: CDP) {
            self.cdps.insert(cdp.id, cdp);
        }
    }

    #[derive(Debug, PartialEq)]
    pub enum TestError {
        ValidationError(String),
        AmountTooSmall(u64, u64),
        InvalidAmount,
    }

    #[test]
    fn test_complete_minting_workflow() {
        let mut engine = MintingEngine::new(BollarConfig::default());
        let config = SystemConfig::default();
        let btc_price = 50_000_000; // $50,000 per BTC

        // Create test CDP with 0.02 BTC collateral ($1,000 value)
        let cdp = CDP {
            id: 1,
            owner: [0u8; 32],
            collateral_amount: 2_000_000, // 0.02 BTC
            minted_amount: 0,
            created_at: 1700000000,
            updated_at: 1700000000,
            is_liquidated: false,
        };
        engine.add_cdp(cdp);

        // Mint $800 (80% of collateral value)
        let request = MintBollarRequest {
            cdp_id: 1,
            owner: [0u8; 32],
            amount_to_mint: 800_000,
            btc_price_cents: btc_price,
        };

        let result = engine.mint_bollar(request, &config);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.cdp_id, 1);
        assert_eq!(response.previous_minted, 0);
        assert_eq!(response.new_minted, 799_200); // $800 - 0.1% fee
        assert_eq!(response.minting_fee, 800); // 0.1% of $800
        assert_eq!(response.collateral_ratio, 12_512); // 125.12%
    }

    #[test]
    fn test_multiple_minting_operations() {
        let mut engine = MintingEngine::new(BollarConfig::default());
        let config = SystemConfig::default();
        let btc_price = 50_000_000; // $50,000 per BTC

        let cdp = CDP {
            id: 1,
            owner: [1u8; 32],
            collateral_amount: 3_000_000, // 0.03 BTC = $1,500
            minted_amount: 0,
            created_at: 1700000000,
            updated_at: 1700000000,
            is_liquidated: false,
        };
        engine.add_cdp(cdp);

        // First mint: $400
        let request1 = MintBollarRequest {
            cdp_id: 1,
            owner: [1u8; 32],
            amount_to_mint: 400_000,
            btc_price_cents: btc_price,
        };

        let response1 = engine.mint_bollar(request1, &config).unwrap();
        assert_eq!(response1.new_minted, 399_600);
        assert_eq!(response1.collateral_ratio, 37_537); // 375.37%

        // Second mint: $300
        let request2 = MintBollarRequest {
            cdp_id: 1,
            owner: [1u8; 32],
            amount_to_mint: 300_000,
            btc_price_cents: btc_price,
        };

        let response2 = engine.mint_bollar(request2, &config).unwrap();
        assert_eq!(response2.new_minted, 699_300); // 399,600 + 299,700
        assert_eq!(response2.collateral_ratio, 21_449); // 214.49%
    }

    #[test]
    fn test_ratio_boundary_conditions() {
        let mut engine = MintingEngine::new(BollarConfig::default());
        let config = SystemConfig::default();
        let btc_price = 100_000_000; // $100,000 per BTC

        // CDP with exactly 90% ratio limit
        let cdp = CDP {
            id: 1,
            owner: [2u8; 32],
            collateral_amount: 1_000_000, // 0.01 BTC = $1,000
            minted_amount: 0,
            created_at: 1700000000,
            updated_at: 1700000000,
            is_liquidated: false,
        };
        engine.add_cdp(cdp);

        // Try to mint exactly 90% ($900)
        let request = MintBollarRequest {
            cdp_id: 1,
            owner: [2u8; 32],
            amount_to_mint: 900_000,
            btc_price_cents: btc_price,
        };

        let result = engine.mint_bollar(request, &config);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.collateral_ratio, 10_000); // Exactly 100%
    }

    #[test]
    fn test_error_scenarios() {
        let mut engine = MintingEngine::new(BollarConfig::default());
        let config = SystemConfig::default();
        let btc_price = 50_000_000; // $50,000 per BTC

        // Test non-existent CDP
        let request = MintBollarRequest {
            cdp_id: 999,
            owner: [3u8; 32],
            amount_to_mint: 100_000,
            btc_price_cents: btc_price,
        };

        let result = engine.mint_bollar(request, &config);
        assert!(matches!(result, Err(TestError::ValidationError(msg)) 
            if msg.contains("CDP not found")));

        // Test wrong owner
        let cdp = CDP {
            id: 1,
            owner: [4u8; 32],
            collateral_amount: 1_000_000,
            minted_amount: 0,
            created_at: 1700000000,
            updated_at: 1700000000,
            is_liquidated: false,
        };
        engine.add_cdp(cdp);

        let wrong_owner_request = MintBollarRequest {
            cdp_id: 1,
            owner: [5u8; 32], // Wrong owner
            amount_to_mint: 100_000,
            btc_price_cents: btc_price,
        };

        let result = engine.mint_bollar(wrong_owner_request, &config);
        assert!(matches!(result, Err(TestError::ValidationError(msg)) 
            if msg.contains("Unauthorized")));

        // Test liquidated CDP
        let mut liquidated_cdp = CDP {
            id: 2,
            owner: [6u8; 32],
            collateral_amount: 1_000_000,
            minted_amount: 0,
            created_at: 1700000000,
            updated_at: 1700000000,
            is_liquidated: true,
        };
        engine.add_cdp(liquidated_cdp);

        let liquidated_request = MintBollarRequest {
            cdp_id: 2,
            owner: [6u8; 32],
            amount_to_mint: 100_000,
            btc_price_cents: btc_price,
        };

        let result = engine.mint_bollar(liquidated_request, &config);
        assert!(matches!(result, Err(TestError::ValidationError(msg)) 
            if msg.contains("liquidated")));
    }

    #[test]
    fn test_fee_edge_cases() {
        let mut engine = MintingEngine::new(BollarConfig::default());
        let config = SystemConfig::default();
        let btc_price = 50_000_000; // $50,000 per BTC

        let cdp = CDP {
            id: 1,
            owner: [7u8; 32],
            collateral_amount: 1_000_000,
            minted_amount: 0,
            created_at: 1700000000,
            updated_at: 1700000000,
            is_liquidated: false,
        };
        engine.add_cdp(cdp);

        // Test minimum fee calculation
        let request = MintBollarRequest {
            cdp_id: 1,
            owner: [7u8; 32],
            amount_to_mint: 1_000, // Minimum mint amount
            btc_price_cents: btc_price,
        };

        let response = engine.mint_bollar(request, &config).unwrap();
        assert_eq!(response.minting_fee, 1); // 0.1% of $1
        assert_eq!(response.new_minted, 999);
    }

    #[test]
    fn test_price_volatility_impact() {
        let mut engine = MintingEngine::new(BollarConfig::default());
        let config = SystemConfig::default();

        let cdp = CDP {
            id: 1,
            owner: [8u8; 32],
            collateral_amount: 2_000_000, // 0.02 BTC
            minted_amount: 0,
            created_at: 1700000000,
            updated_at: 1700000000,
            is_liquidated: false,
        };
        engine.add_cdp(cdp);

        // Test with different BTC prices
        let test_cases = vec![
            (40_000_000, 720_000), // $40k BTC = $800 max, 90% = $720
            (50_000_000, 900_000), // $50k BTC = $1,000 max, 90% = $900
            (60_000_000, 1_080_000), // $60k BTC = $1,200 max, 90% = $1,080
        ];

        for (btc_price, expected_max) in test_cases {
            let max_mintable = engine.calculate_max_mintable(
                2_000_000,
                btc_price,
                &config,
            ).unwrap();
            assert_eq!(max_mintable, expected_max);
        }
    }

    #[test]
    fn test_atomic_state_updates() {
        let mut engine = MintingEngine::new(BollarConfig::default());
        let config = SystemConfig::default();
        let btc_price = 50_000_000;

        let cdp = CDP {
            id: 1,
            owner: [9u8; 32],
            collateral_amount: 1_000_000,
            minted_amount: 0,
            created_at: 1700000000,
            updated_at: 1700000000,
            is_liquidated: false,
        };
        engine.add_cdp(cdp);

        let request = MintBollarRequest {
            cdp_id: 1,
            owner: [9u8; 32],
            amount_to_mint: 500_000,
            btc_price_cents: btc_price,
        };

        let response = engine.mint_bollar(request, &config).unwrap();

        // Verify state was updated atomically
        let updated_cdp = engine.cdps.get(&1).unwrap();
        assert_eq!(updated_cdp.minted_amount, 499_500); // 500,000 - 500 fee
        assert_eq!(updated_cdp.updated_at, 1700000000);
    }

    #[test]
    fn test_concurrent_minting_simulation() {
        let mut engine = MintingEngine::new(BollarConfig::default());
        let config = SystemConfig::default();
        let btc_price = 50_000_000;

        // Create multiple CDPs
        for i in 0..5 {
            let cdp = CDP {
                id: i + 1,
                owner: [i as u8; 32],
                collateral_amount: 1_000_000,
                minted_amount: 0,
                created_at: 1700000000,
                updated_at: 1700000000,
                is_liquidated: false,
            };
            engine.add_cdp(cdp);
        }

        // Simulate concurrent minting requests
        let mint_amounts = vec![100_000, 200_000, 300_000, 400_000, 450_000];
        let mut results = Vec::new();

        for (i, amount) in mint_amounts.iter().enumerate() {
            let request = MintBollarRequest {
                cdp_id: (i + 1) as u64,
                owner: [i as u8; 32],
                amount_to_mint: *amount,
                btc_price_cents: btc_price,
            };

            let result = engine.mint_bollar(request, &config);
            results.push(result);
        }

        // Verify all operations succeeded
        for result in results {
            assert!(result.is_ok());
        }

        // Verify final states
        for i in 0..5 {
            let cdp = engine.cdps.get(&((i + 1) as u64)).unwrap();
            assert!(cdp.minted_amount > 0);
            assert!(!cdp.is_liquidated);
        }
    }

    #[test]
    fn test_collateral_ratio_precision() {
        let engine = MintingEngine::new(BollarConfig::default());
        let btc_price = 100_000_000; // $100,000 per BTC

        // Test precise ratio calculations
        let test_cases = vec![
            (1_000_000, 900_000, 11_111), // 111.11%
            (1_000_000, 950_000, 10_526), // 105.26%
            (1_000_000, 1_000_000, 10_000), // 100.00%
            (1_000_000, 1_100_000, 9_090),  // 90.90%
        ];

        for (collateral, minted, expected_ratio) in test_cases {
            let ratio = engine.calculate_collateral_ratio(
                collateral,
                minted,
                btc_price,
            ).unwrap();
            assert_eq!(ratio, expected_ratio);
        }
    }

    #[test]
    fn test_minting_limits_enforcement() {
        let mut engine = MintingEngine::new(BollarConfig::default());
        let config = SystemConfig::default();
        let btc_price = 50_000_000;

        let cdp = CDP {
            id: 1,
            owner: [10u8; 32],
            collateral_amount: 1_000_000, // 0.01 BTC = $500
            minted_amount: 0,
            created_at: 1700000000,
            updated_at: 1700000000,
            is_liquidated: false,
        };
        engine.add_cdp(cdp);

        // Try to mint exactly at 90% limit
        let request = MintBollarRequest {
            cdp_id: 1,
            owner: [10u8; 32],
            amount_to_mint: 450_000, // Exactly 90% of $500
            btc_price_cents: btc_price,
        };

        let result = engine.mint_bollar(request, &config);
        assert!(result.is_ok());

        // Try to mint slightly over 90%
        let request_over = MintBollarRequest {
            cdp_id: 1,
            owner: [10u8; 32],
            amount_to_mint: 451_000, // Slightly over 90%
            btc_price_cents: btc_price,
        };

        let result = engine.mint_bollar(request_over, &config);
        assert!(result.is_err());
    }
}

#[cfg(test)]
pub fn run_all_minting_tests() {
    use minting_integration_tests::*;
    
    println!("🧪 Running Minting Engine Integration Tests...");
    
    test_complete_minting_workflow();
    println!("✅ Complete minting workflow test passed");
    
    test_multiple_minting_operations();
    println!("✅ Multiple minting operations test passed");
    
    test_ratio_boundary_conditions();
    println!("✅ Ratio boundary conditions test passed");
    
    test_error_scenarios();
    println!("✅ Error scenarios test passed");
    
    test_fee_edge_cases();
    println!("✅ Fee edge cases test passed");
    
    test_price_volatility_impact();
    println!("✅ Price volatility impact test passed");
    
    test_atomic_state_updates();
    println!("✅ Atomic state updates test passed");
    
    test_concurrent_minting_simulation();
    println!("✅ Concurrent minting simulation test passed");
    
    test_collateral_ratio_precision();
    println!("✅ Collateral ratio precision test passed");
    
    test_minting_limits_enforcement();
    println!("✅ Minting limits enforcement test passed");
    
    println!("🎉 All minting engine tests completed successfully!");
}

#[cfg(test)]
fn main() {
    run_all_minting_tests();
}

#[cfg(not(test))]
fn main() {
    println!("Minting Engine Tests - Run with `cargo test`");
}