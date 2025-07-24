//! CDP closure and redemption engine for returning BTC collateral

use crate::types::*;
use crate::price::*;
use std::cell::RefCell;

thread_local! {
    pub static TOTAL_CLOSURE_EVENTS: RefCell<u64> = RefCell::new(0);
    pub static TOTAL_CLOSURE_FEES: RefCell<u64> = RefCell::new(0);
}

/// Calculate the amount of Bollar needed to close a CDP
pub fn calculate_closure_amount(cdp: &CDP) -> u64 {
    cdp.minted_amount
}

/// Calculate redemption amount (BTC to return to user)
pub fn calculate_redemption_amount(
    cdp: &CDP,
    btc_price_cents: u64,
    closure_fee_rate: u32,
) -> u64 {
    let closure_fee = cdp.collateral_amount * closure_fee_rate as u64 / 10_000;
    cdp.collateral_amount.saturating_sub(closure_fee)
}

/// Validate CDP closure requirements
pub fn validate_closure(
    cdp: &CDP,
    caller: &Principal,
    repayment_amount: u64,
) -> Result<(), ProtocolError> {
    if cdp.owner != *caller {
        return Err(ProtocolError::UnauthorizedAccess);
    }
    
    if cdp.is_liquidated {
        return Err(ProtocolError::CDPAlreadyLiquidated(cdp.id));
    }
    
    if repayment_amount != cdp.minted_amount {
        return Err(ProtocolError::InvalidRepaymentAmount {
            expected: cdp.minted_amount,
            actual: repayment_amount,
        });
    }
    
    Ok(())
}

/// Execute CDP closure - safe version for testing
pub fn execute_closure_safe(
    mut cdp: CDP,
    repayment_amount: u64,
    btc_price_cents: u64,
    closure_fee_rate: u32,
) -> Result<ClosureResult, ProtocolError> {
    validate_closure(&cdp, &cdp.owner, repayment_amount)?;
    
    let redemption_amount = calculate_redemption_amount(&cdp, btc_price_cents, closure_fee_rate
    );
    
    let closure_result = ClosureResult {
        cdp_id: cdp.id,
        redemption_amount_satoshis: redemption_amount,
        closure_fee_satoshis: cdp.collateral_amount - redemption_amount,
        total_repaid_cents: repayment_amount,
    };
    
    // Mark CDP as closed (effectively liquidated to prevent further operations)
    cdp.is_liquidated = true;
    cdp.updated_at = ic_cdk::api::time();
    
    Ok(closure_result)
}

/// Structure for closure calculation results
#[derive(Debug, Clone)]
pub struct ClosureResult {
    pub cdp_id: u64,
    pub redemption_amount_satoshis: u64,
    pub closure_fee_satoshis: u64,
    pub total_repaid_cents: u64,
}

/// Structure for closure preview information
#[derive(Debug, Clone)]
pub struct ClosurePreview {
    pub cdp_id: u64,
    pub minted_amount_cents: u64,
    pub collateral_amount_satoshis: u64,
    pub redemption_amount_satoshis: u64,
    pub closure_fee_satoshis: u64,
    pub btc_price_cents: u64,
}

/// Calculate closure preview for a CDP
pub fn calculate_closure_preview(
    cdp: &CDP,
    btc_price_cents: u64,
    closure_fee_rate: u32,
) -> ClosurePreview {
    let redemption_amount = calculate_redemption_amount(cdp, btc_price_cents, closure_fee_rate);
    let closure_fee = cdp.collateral_amount - redemption_amount;
    
    ClosurePreview {
        cdp_id: cdp.id,
        minted_amount_cents: cdp.minted_amount,
        collateral_amount_satoshis: cdp.collateral_amount,
        redemption_amount_satoshis: redemption_amount,
        closure_fee_satoshis: closure_fee,
        btc_price_cents,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;
    use crate::types::CDP;
    
    fn test_principal() -> Principal {
        Principal::anonymous()
    }
    
    fn test_config() -> SystemConfig {
        SystemConfig::default()
    }
    
    fn test_cdp() -> CDP {
        CDP {
            id: 1,
            owner: test_principal(),
            collateral_amount: 1_000_000, // 0.01 BTC
            minted_amount: 500_000, // $500 Bollar
            created_at: 0,
            updated_at: 0,
            is_liquidated: false,
        }
    }
    
    #[test]
    fn test_calculate_closure_amount() {
        let cdp = test_cdp();
        let closure_amount = calculate_closure_amount(&cdp);
        assert_eq!(closure_amount, 500_000); // Must repay exact minted amount
    }
    
    #[test]
    fn test_calculate_redemption_amount() {
        let cdp = test_cdp();
        let btc_price = 65_000_000;
        let redemption = calculate_redemption_amount(&cdp, btc_price, 100); // 1% fee
        assert_eq!(redemption, 990_000); // 0.01 BTC - 1% = 0.0099 BTC
    }
    
    #[test]
    fn test_validate_closure_success() {
        let cdp = test_cdp();
        let caller = test_principal();
        assert!(validate_closure(&cdp, &caller, 500_000).is_ok());
    }
    
    #[test]
    fn test_validate_closure_wrong_owner() {
        let cdp = test_cdp();
        let wrong_owner = Principal::from_text("aaaaa-aa").unwrap();
        let result = validate_closure(&cdp, &wrong_owner, 500_000);
        assert!(matches!(result, Err(ProtocolError::UnauthorizedAccess)));
    }
    
    #[test]
    fn test_validate_closure_liquidated() {
        let mut cdp = test_cdp();
        cdp.is_liquidated = true;
        let caller = test_principal();
        let result = validate_closure(&cdp, &caller, 500_000);
        assert!(matches!(result, Err(ProtocolError::CDPAlreadyLiquidated(1))));
    }
    
    #[test]
    fn test_validate_closure_wrong_amount() {
        let cdp = test_cdp();
        let caller = test_principal();
        let result = validate_closure(&cdp, &caller, 400_000);
        assert!(matches!(result, Err(ProtocolError::InvalidRepaymentAmount { .. })));
    }
    
    #[test]
    fn test_execute_closure_safe_success() {
        let cdp = test_cdp();
        let btc_price = 65_000_000;
        let result = execute_closure_safe(cdp, 500_000, btc_price, 100);
        assert!(result.is_ok());
        let closure = result.unwrap();
        assert_eq!(closure.cdp_id, 1);
        assert_eq!(closure.redemption_amount_satoshis, 990_000);
        assert_eq!(closure.closure_fee_satoshis, 10_000);
        assert_eq!(closure.total_repaid_cents, 500_000);
    }
    
    #[test]
    fn test_calculate_closure_preview() {
        let cdp = test_cdp();
        let btc_price = 65_000_000;
        let preview = calculate_closure_preview(&cdp, btc_price, 100);
        
        assert_eq!(preview.cdp_id, 1);
        assert_eq!(preview.minted_amount_cents, 500_000);
        assert_eq!(preview.collateral_amount_satoshis, 1_000_000);
        assert_eq!(preview.redemption_amount_satoshis, 990_000);
        assert_eq!(preview.closure_fee_satoshis, 10_000);
        assert_eq!(preview.btc_price_cents, 65_000_000);
    }
    
    #[test]
    fn test_zero_closure_fee() {
        let cdp = test_cdp();
        let btc_price = 65_000_000;
        let redemption = calculate_redemption_amount(&cdp, btc_price, 0); // 0% fee
        assert_eq!(redemption, 1_000_000); // Full collateral returned
    }
    
    #[test]
    fn test_maximum_closure_fee() {
        let cdp = test_cdp();
        let btc_price = 65_000_000;
        let redemption = calculate_redemption_amount(&cdp, btc_price, 1000); // 10% fee
        assert_eq!(redemption, 900_000); // 90% of collateral returned
    }
    
    #[test]
    fn test_insufficient_collateral_for_fee() {
        let cdp = CDP {
            id: 2,
            owner: test_principal(),
            collateral_amount: 100, // Very small amount
            minted_amount: 50_000,
            created_at: 0,
            updated_at: 0,
            is_liquidated: false,
        };
        
        let btc_price = 65_000_000;
        let redemption = calculate_redemption_amount(&cdp, btc_price, 100); // 1% fee = 1 satoshi
        assert_eq!(redemption, 99); // Saturating subtraction prevents underflow
    }
}