// Liquidation Engine Tests - Task 2.2

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
pub struct LiquidationConfig {
    pub liquidation_threshold: u32,
    pub liquidation_penalty: u32,
    pub liquidator_reward: u32,
    pub min_liquidation_amount: u64,
}

impl Default for LiquidationConfig {
    fn default() -> Self {
        Self {
            liquidation_threshold: 8500, // 85%
            liquidation_penalty: 500,    // 5%
            liquidator_reward: 300,      // 3%
            min_liquidation_amount: 1_000, // $0.01
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
pub struct LiquidationRequest {
    pub cdp_id: u64,
    pub liquidator: [u8; 32],
    pub btc_price_cents: u64,
    pub liquidation_amount: u64,
}

#[derive(Debug, Clone)]
pub struct LiquidationResponse {
    pub cdp_id: u64,
    pub liquidated_amount: u64,
    pub penalty_amount: u64,
    pub remaining_collateral: u64,
    pub remaining_debt: u64,
    pub liquidator_reward: u64,
}

#[derive(Debug, Clone)]
pub struct LiquidationCalculation {
    pub total_liquidation: u64,
    pub penalty_amount: u64,
    pub liquidator_reward: u64,
    pub remaining_collateral: u64,
}

#[derive(Debug, Clone)]
pub struct LiquidationEngine {
    cdps: std::collections::HashMap<u64, CDP>,
    config: LiquidationConfig,
}

impl LiquidationEngine {
    pub fn new(config: LiquidationConfig) -> Self {
        Self {
            cdps: std::collections::HashMap::new(),
            config,
        }
    }

    pub fn is_eligible_for_liquidation(
        &self,
        cdp: &CDP,
        btc_price_cents: u64,
        system_config: &SystemConfig,
    ) -> Result<bool, TestError> {
        if cdp.is_liquidated {
            return Ok(false);
        }

        if cdp.minted_amount == 0 {
            return Ok(false);
        }

        let collateral_value_cents = cdp.collateral_amount * btc_price_cents / 100_000_000;
        let current_ratio = (collateral_value_cents * 10_000) / cdp.minted_amount;

        Ok(current_ratio <= self.config.liquidation_threshold as u64)
    }

    pub fn calculate_liquidation_amounts(
        &self,
        cdp: &CDP,
        btc_price_cents: u64,
        system_config: &SystemConfig,
    ) -> Result<LiquidationCalculation, TestError> {
        let collateral_value_cents = cdp.collateral_amount * btc_price_cents / 100_000_000;
        let target_collateral_ratio = system_config.max_collateral_ratio as u64;
        
        // Calculate required liquidation to bring ratio back to target
        let required_collateral_value = (cdp.minted_amount * target_collateral_ratio) / 10_000;
        let shortfall = required_collateral_value.saturating_sub(collateral_value_cents);
        
        if shortfall == 0 {
            return Err(TestError::ValidationError("No liquidation needed".to_string()));
        }

        // Convert shortfall back to BTC amount
        let liquidation_btc_amount = (shortfall * 100_000_000) / btc_price_cents;
        
        // Ensure we don't liquidate more than available
        let actual_liquidation = liquidation_btc_amount.min(cdp.collateral_amount);
        
        // Calculate penalty and rewards
        let penalty_amount = (actual_liquidation * self.config.liquidation_penalty as u64) / 10_000;
        let liquidator_reward = (actual_liquidation * self.config.liquidator_reward as u64) / 10_000;
        
        Ok(LiquidationCalculation {
            total_liquidation: actual_liquidation,
            penalty_amount,
            liquidator_reward,
            remaining_collateral: cdp.collateral_amount - actual_liquidation,
        })
    }

    pub fn liquidate_cdp(
        &mut self,
        request: LiquidationRequest,
        system_config: &SystemConfig,
    ) -> Result<LiquidationResponse, TestError> {
        // Get CDP
        let cdp = self.cdps.get(&request.cdp_id)
            .ok_or(TestError::ValidationError("CDP not found".to_string()))?;

        // Check if already liquidated
        if cdp.is_liquidated {
            return Err(TestError::ValidationError("CDP already liquidated".to_string()));
        }

        // Check eligibility
        let is_eligible = self.is_eligible_for_liquidation(cdp, request.btc_price_cents, system_config)?;
        if !is_eligible {
            return Err(TestError::ValidationError("CDP not eligible for liquidation".to_string()));
        }

        // Calculate liquidation amounts
        let calculation = self.calculate_liquidation_amounts(cdp, request.btc_price_cents, system_config)?;

        // Validate liquidation amount
        if request.liquidation_amount < self.config.min_liquidation_amount {
            return Err(TestError::ValidationError("Liquidation amount too small".to_string()));
        }

        // Update CDP state
        let mut updated_cdp = cdp.clone();
        updated_cdp.collateral_amount = calculation.remaining_collateral;
        updated_cdp.is_liquidated = updated_cdp.collateral_amount == 0 || updated_cdp.minted_amount == 0;
        updated_cdp.updated_at = 1700000000; // Mock timestamp

        // Store updated CDP
        self.cdps.insert(request.cdp_id, updated_cdp);

        Ok(LiquidationResponse {
            cdp_id: request.cdp_id,
            liquidated_amount: calculation.total_liquidation,
            penalty_amount: calculation.penalty_amount,
            remaining_collateral: calculation.remaining_collateral,
            remaining_debt: cdp.minted_amount, // Simplified - would need debt calculation
            liquidator_reward: calculation.liquidator_reward,
        })
    }

    pub fn get_eligible_cdps(
        &self,
        btc_price_cents: u64,
        system_config: &SystemConfig,
    ) -> Vec<u64> {
        self.cdps
            .iter()
            .filter_map(|(id, cdp)| {
                match self.is_eligible_for_liquidation(cdp, btc_price_cents, system_config) {
                    Ok(true) => Some(*id),
                    _ => None,
                }
            })
            .collect()
    }

    pub fn add_cdp(&mut self, cdp: CDP) {
        self.cdps.insert(cdp.id, cdp);
    }

    pub fn get_cdp(&self, cdp_id: u64) -> Option<&CDP> {
        self.cdps.get(&cdp_id)
    }

    pub fn get_all_cdps(&self) -> Vec<&CDP> {
        self.cdps.values().collect()
    }
}

#[derive(Debug, PartialEq)]
pub enum TestError {
    ValidationError(String),
    AmountTooSmall(u64, u64),
    InvalidAmount,
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
fn test_liquidation_eligibility() {
    let engine = LiquidationEngine::new(LiquidationConfig::default());
    let config = SystemConfig::default();
    let btc_price = 50_000_000; // $50,000 per BTC

    // Test case 1: Healthy CDP (100% ratio)
    let healthy_cdp = create_test_cdp(1, [0u8; 32], 1_000_000, 500_000); // 0.01 BTC, $500 minted
    let eligible = engine.is_eligible_for_liquidation(
        &healthy_cdp, btc_price, &config).unwrap();
    assert!(!eligible);

    // Test case 2: Undercollateralized CDP (84% ratio)
    let risky_cdp = create_test_cdp(2, [0u8; 32], 1_000_000, 595_000); // 0.01 BTC, $595 minted
    let eligible = engine.is_eligible_for_liquidation(
        &risky_cdp, btc_price, &config).unwrap();
    assert!(eligible);

    // Test case 3: Already liquidated
    let mut liquidated_cdp = create_test_cdp(3, [0u8; 32], 1_000_000, 500_000);
    liquidated_cdp.is_liquidated = true;
    let eligible = engine.is_eligible_for_liquidation(
        &liquidated_cdp, btc_price, &config).unwrap();
    assert!(!eligible);
}

#[test]
fn test_liquidation_calculation() {
    let engine = LiquidationEngine::new(LiquidationConfig::default());
    let btc_price = 50_000_000; // $50,000 per BTC

    // CDP with 84% ratio (needs liquidation)
    let cdp = create_test_cdp(1, [0u8; 32], 1_000_000, 595_000); // 0.01 BTC, $595 minted

    let calculation = engine.calculate_liquidation_amounts(
        &cdp, btc_price, &SystemConfig::default()).unwrap();
    
    assert!(calculation.total_liquidation > 0);
    assert!(calculation.penalty_amount > 0);
    assert!(calculation.liquidator_reward > 0);
    assert!(calculation.remaining_collateral < cdp.collateral_amount);
}

#[test]
fn test_complete_liquidation_workflow() {
    let mut engine = LiquidationEngine::new(LiquidationConfig::default());
    let config = SystemConfig::default();
    let btc_price = 50_000_000; // $50,000 per BTC

    // Create undercollateralized CDP
    let cdp = create_test_cdp(1, [0u8; 32], 1_000_000, 595_000); // 84% ratio
    engine.add_cdp(cdp);

    let request = LiquidationRequest {
        cdp_id: 1,
        liquidator: [1u8; 32],
        btc_price_cents: btc_price,
        liquidation_amount: 100_000, // 0.001 BTC
    };

    let result = engine.liquidate_cdp(request, &config);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.cdp_id, 1);
    assert!(response.liquidated_amount > 0);
    assert!(response.penalty_amount > 0);
    assert!(response.liquidator_reward > 0);
}

#[test]
fn test_get_eligible_cdps() {
    let mut engine = LiquidationEngine::new(LiquidationConfig::default());
    let config = SystemConfig::default();
    let btc_price = 50_000_000; // $50,000 per BTC

    // Add multiple CDPs
    engine.add_cdp(create_test_cdp(1, [0u8; 32], 1_000_000, 500_000)); // 100% - safe
    engine.add_cdp(create_test_cdp(2, [0u8; 32], 1_000_000, 595_000)); // 84% - eligible
    engine.add_cdp(create_test_cdp(3, [0u8; 32], 1_000_000, 600_000)); // 83% - eligible

    let eligible = engine.get_eligible_cdps(btc_price, &config);
    assert_eq!(eligible.len(), 2);
    assert!(eligible.contains(&2));
    assert!(eligible.contains(&3));
}

#[test]
fn test_no_liquidation_needed() {
    let engine = LiquidationEngine::new(LiquidationConfig::default());
    let btc_price = 50_000_000; // $50,000 per BTC

    // Healthy CDP
    let cdp = create_test_cdp(1, [0u8; 32], 1_000_000, 400_000); // 125% ratio

    let result = engine.calculate_liquidation_amounts(
        &cdp, btc_price, &SystemConfig::default()).unwrap_err();
    assert!(matches!(result, TestError::ValidationError(msg) 
        if msg.contains("No liquidation needed")));
}

#[test]
fn test_edge_case_liquidations() {
    let mut engine = LiquidationEngine::new(LiquidationConfig::default());
    let config = SystemConfig::default();
    let btc_price = 50_000_000; // $50,000 per BTC

    // Test exact threshold
    let exact_cdp = create_test_cdp(1, [0u8; 32], 1_000_000, 588_235); // Exactly 85%
    engine.add_cdp(exact_cdp);

    let eligible = engine.is_eligible_for_liquidation(
        engine.get_cdp(1).unwrap(), 
        btc_price, 
        &config
    ).unwrap();
    assert!(eligible);

    // Test just above threshold
    let slightly_above = create_test_cdp(2, [0u8; 32], 1_000_000, 580_000); // ~86%
    engine.add_cdp(slightly_above);

    let eligible = engine.is_eligible_for_liquidation(
        engine.get_cdp(2).unwrap(), 
        btc_price, &config
    ).unwrap();
    assert!(!eligible);
}

#[test]
fn test_partial_liquidation() {
    let mut engine = LiquidationEngine::new(LiquidationConfig::default());
    let config = SystemConfig::default();
    let btc_price = 50_000_000; // $50,000 per BTC

    // Create severely undercollateralized CDP (70% ratio)
    let cdp = create_test_cdp(1, [0u8; 32], 1_000_000, 714_285); // 0.01 BTC, $714 minted
    engine.add_cdp(cdp);

    let request = LiquidationRequest {
        cdp_id: 1,
        liquidator: [1u8; 32],
        btc_price_cents: btc_price,
        liquidation_amount: 200_000, // 0.002 BTC
    };

    let result = engine.liquidate_cdp(request, &config);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.liquidated_amount > 0);
    assert!(response.remaining_collateral > 0); // Partial liquidation
    assert!(!engine.get_cdp(1).unwrap().is_liquidated); // Still not fully liquidated
}

#[test]
fn test_full_liquidation() {
    let mut engine = LiquidationEngine::new(LiquidationConfig::default());
    let config = SystemConfig::default();
    let btc_price = 50_000_000; // $50,000 per BTC

    // Create CDP that will be fully liquidated
    let cdp = create_test_cdp(1, [0u8; 32], 100_000, 100_000); // 0.001 BTC, $100 minted
    engine.add_cdp(cdp);

    let request = LiquidationRequest {
        cdp_id: 1,
        liquidator: [1u8; 32],
        btc_price_cents: btc_price,
        liquidation_amount: 100_000, // 0.001 BTC (full amount)
    };

    let result = engine.liquidate_cdp(request, &config);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.remaining_collateral, 0);
    assert!(engine.get_cdp(1).unwrap().is_liquidated); // Fully liquidated
}

#[test]
fn test_liquidation_error_cases() {
    let mut engine = LiquidationEngine::new(LiquidationConfig::default());
    let config = SystemConfig::default();
    let btc_price = 50_000_000; // $50,000 per BTC

    // Test non-existent CDP
    let request = LiquidationRequest {
        cdp_id: 999,
        liquidator: [1u8; 32],
        btc_price_cents: btc_price,
        liquidation_amount: 100_000,
    };

    let result = engine.liquidate_cdp(request, &config);
    assert!(matches!(result, Err(TestError::ValidationError(msg)) 
        if msg.contains("CDP not found")));

    // Test already liquidated CDP
    let mut cdp = create_test_cdp(1, [0u8; 32], 1_000_000, 595_000);
    cdp.is_liquidated = true;
    engine.add_cdp(cdp);

    let request = LiquidationRequest {
        cdp_id: 1,
        liquidator: [1u8; 32],
        btc_price_cents: btc_price,
        liquidation_amount: 100_000,
    };

    let result = engine.liquidate_cdp(request, &config);
    assert!(matches!(result, Err(TestError::ValidationError(msg)) 
        if msg.contains("already liquidated")));

    // Test not eligible CDP
    let safe_cdp = create_test_cdp(2, [0u8; 32], 1_000_000, 400_000); // 125% ratio
    engine.add_cdp(safe_cdp);

    let request = LiquidationRequest {
        cdp_id: 2,
        liquidator: [1u8; 32],
        btc_price_cents: btc_price,
        liquidation_amount: 100_000,
    };

    let result = engine.liquidate_cdp(request, &config);
    assert!(matches!(result, Err(TestError::ValidationError(msg)) 
        if msg.contains("not eligible")));
}

#[test]
fn test_price_volatility_impact_on_liquidation() {
    let mut engine = LiquidationEngine::new(LiquidationConfig::default());
    let config = SystemConfig::default();

    // Create CDP with fixed minted amount
    let cdp = create_test_cdp(1, [0u8; 32], 1_000_000, 500_000); // 0.01 BTC, $500 minted
    engine.add_cdp(cdp);

    let test_cases = vec![
        (40_000_000, true),  // $40k BTC = 80% ratio - eligible
        (50_000_000, false), // $50k BTC = 100% ratio - safe
        (60_000_000, false), // $60k BTC = 120% ratio - safe
    ];

    for (btc_price, should_be_eligible) in test_cases {
        let eligible = engine.is_eligible_for_liquidation(
            engine.get_cdp(1).unwrap(), 
            btc_price, 
            &config
        ).unwrap();
        
        assert_eq!(eligible, should_be_eligible, "Failed for price: {}", btc_price);
    }
}

#[cfg(test)]
fn run_all_liquidation_tests() {
    test_liquidation_eligibility();
    test_liquidation_calculation();
    test_complete_liquidation_workflow();
    test_get_eligible_cdps();
    test_no_liquidation_needed();
    test_edge_case_liquidations();
    test_partial_liquidation();
    test_full_liquidation();
    test_liquidation_error_cases();
    test_price_volatility_impact_on_liquidation();
    
    println!("🎉 All liquidation engine tests passed!");
}

fn main() {
    run_all_liquidation_tests();
}