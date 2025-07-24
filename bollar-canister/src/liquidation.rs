//! Liquidation engine for handling undercollateralized CDPs

use crate::types::*;
use crate::price::*;
use std::cell::RefCell;

thread_local! {
    pub static TOTAL_LIQUIDATION_EVENTS: RefCell<u64> = RefCell::new(0);
    pub static TOTAL_LIQUIDATION_PENALTIES: RefCell<u64> = RefCell::new(0);
}

/// Calculate liquidation penalty amount based on minted amount
pub fn calculate_liquidation_penalty(
    minted_amount: u64,
    penalty_rate: u32, // basis points
) -> u64 {
    minted_amount * penalty_rate as u64 / 10_000
}

/// Calculate liquidator reward amount
pub fn calculate_liquidator_reward(
    collateral_amount: u64,
    reward_rate: u32, // basis points
) -> u64 {
    collateral_amount * reward_rate as u64 / 10_000
}

/// Check if a CDP should be liquidated based on current collateral ratio
pub fn should_liquidate(
    cdp: &CDP,
    btc_price_cents: u64,
    liquidation_threshold: u32,
) -> bool {
    let current_ratio = cdp.calculate_collateral_ratio(btc_price_cents);
    current_ratio < liquidation_threshold
}

/// Calculate required repayment amount for liquidation
pub fn calculate_liquidation_amounts(
    cdp: &CDP,
    btc_price_cents: u64,
    penalty_rate: u32,
    reward_rate: u32,
) -> LiquidationAmounts {
    let penalty_amount = calculate_liquidation_penalty(cdp.minted_amount, penalty_rate);
    let total_repayment = cdp.minted_amount + penalty_amount;
    let liquidator_reward = calculate_liquidator_reward(cdp.collateral_amount, reward_rate);
    let remaining_collateral = cdp.collateral_amount.saturating_sub(liquidator_reward);
    
    LiquidationAmounts {
        total_repayment,
        penalty_amount,
        liquidator_reward_satoshis: liquidator_reward,
        remaining_collateral_satoshis: remaining_collateral,
    }
}

/// Execute liquidation - safe version for testing
pub fn execute_liquidation_safe(
    mut cdp: CDP,
    btc_price_cents: u64,
    penalty_rate: u32,
    reward_rate: u32,
) -> Result<LiquidationResult, ProtocolError> {
    if cdp.is_liquidated {
        return Err(ProtocolError::CDPAlreadyLiquidated(cdp.id));
    }
    
    if !should_liquidate(&cdp, btc_price_cents, 8500) { // 85% threshold
        let current_ratio = cdp.calculate_collateral_ratio(btc_price_cents);
        return Err(ProtocolError::CDPNotUndercollateralized {
            id: cdp.id,
            current_ratio,
            threshold: 8500,
        });
    }
    
    let amounts = calculate_liquidation_amounts(&cdp, btc_price_cents, penalty_rate, reward_rate);
    cdp.is_liquidated = true;
    cdp.updated_at = ic_cdk::api::time();
    
    Ok(LiquidationResult{
        cdp: cdp.clone(),
        amounts: amounts.clone(),
        liquidator_reward_satoshis: amounts.liquidator_reward_satoshis,
        protocol_fee_satoshis: amounts.remaining_collateral_satoshis,
    })
}

/// Find CDPs eligible for liquidation
pub fn find_liquidatable_cdps(
    btc_price_cents: u64,
    liquidation_threshold: u32,
) -> Vec<u64> {
    CDPS.with(|cdps| {
        cdps.borrow()
            .iter()
            .filter(|(_, cdp)| {
                !cdp.is_liquidated && should_liquidate(cdp, btc_price_cents, liquidation_threshold)
            })
            .map(|(id, _)| id)
            .collect()
    })
}

/// Structure for liquidation calculation results
#[derive(Debug, Clone)]
pub struct LiquidationAmounts {
    pub total_repayment: u64,           // Total Bollar to repay (principal + penalty)
    pub penalty_amount: u64,           // Protocol penalty (5% of minted amount)
    pub liquidator_reward_satoshis: u64, // BTC reward for liquidator (5% of collateral)
    pub remaining_collateral_satoshis: u64, // Remaining BTC goes to protocol
}

/// Structure for liquidation execution result
#[derive(Debug, Clone)]
pub struct LiquidationResult {
    pub cdp: CDP,
    pub amounts: LiquidationAmounts,
    pub liquidator_reward_satoshis: u64,
    pub protocol_fee_satoshis: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;
    use crate::types::CDP;
    
    fn test_principal() -> Principal {
        Principal::anonymous()
    }
    
    fn test_cdp() -> CDP {
        CDP {
            id: 1,
            owner: test_principal(),
            collateral_amount: 1_000_000, // 0.01 BTC
            minted_amount: 600_000, // $600 Bollar minted
            created_at: ic_cdk::api::time(),
            updated_at: ic_cdk::api::time(),
            is_liquidated: false,
        }
    }
    
    fn liquidatable_cdp() -> CDP {
        CDP {
            id: 2,
            owner: test_principal(),
            collateral_amount: 1_000_000, // 0.01 BTC at $65k = $650
            minted_amount: 750_000, // $750 minted (84.6% ratio)
            created_at: ic_cdk::api::time(),
            updated_at: ic_cdk::api::time(),
            is_liquidated: false,
        }
    }
    
    #[test]
    fn test_calculate_liquidation_penalty() {
        let minted_amount = 1_000_000; // $1,000 Bollar
        let penalty_rate = 500; // 5%
        let penalty = calculate_liquidation_penalty(minted_amount, penalty_rate);
        assert_eq!(penalty, 50_000); // $50 penalty
    }
    
    #[test]
    fn test_calculate_liquidator_reward() {
        let collateral_amount = 1_000_000; // 0.01 BTC
        let reward_rate = 500; // 5%
        let reward = calculate_liquidator_reward(collateral_amount, reward_rate);
        assert_eq!(reward, 50_000); // 0.0005 BTC reward
    }
    
    #[test]
    fn test_should_liquidate_success() {
        let liquidatable = liquidatable_cdp();
        let btc_price = 65_000_000; // $65,000 per BTC
        assert!(should_liquidate(&liquidatable, btc_price, 8500));
    }
    
    #[test]
    fn test_should_liquidate_failure() {
        let safe_cdp = test_cdp(); // Ratio would be ~108% at 0.01 BTC for $600
        let btc_price = 65_000_000;
        assert!(!should_liquidate(&safe_cdp, btc_price, 8500));
    }
    
    #[test]
    fn test_calculate_liquidation_amounts() {
        let cdp = liquidatable_cdp();
        let btc_price = 65_000_000;
        let amounts = calculate_liquidation_amounts(&cdp, btc_price, 500, 500);
        
        assert_eq!(amounts.penalty_amount, 37_500); // 5% of 750,000
        assert_eq!(amounts.total_repayment, 787_500); // 750,000 + 37,500
        assert_eq!(amounts.liquidator_reward_satoshis, 50_000); // 5% of 1,000,000
        assert_eq!(amounts.remaining_collateral_satoshis, 950_000);
    }
    
    #[test]
    fn test_execute_liquidation_safe_success() {
        let cdp = liquidatable_cdp();
        let btc_price = 65_000_000;
        let result = execute_liquidation_safe(cdp, btc_price, 500, 500);
        assert!(result.is_ok());
        let liquidation = result.unwrap();
        assert!(liquidation.cdp.is_liquidated);
        assert_eq!(liquidation.liquidator_reward_satoshis, 50_000);
    }
    
    #[test]
    fn test_execute_liquidation_safe_not_undercollateralized() {
        let cdp = test_cdp();
        let btc_price = 65_000_000;
        let result = execute_liquidation_safe(cdp.clone(), btc_price, 500, 500);
        assert!(matches!(result, Err(ProtocolError::CDPNotUndercollateralized { .. })));
    }
    
    #[test]
    fn test_execute_liquidation_safe_already_liquidated() {
        let mut cdp = liquidatable_cdp();
        cdp.is_liquidated = true;
        let btc_price = 65_000_000;
        let result = execute_liquidation_safe(cdp, btc_price, 500, 500);
        assert!(matches!(result, Err(ProtocolError::CDPAlreadyLiquidated(2))));
    }
}