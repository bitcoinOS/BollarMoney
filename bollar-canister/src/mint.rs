//! Bollar minting engine for creating stablecoins against CDP collateral

use crate::types::*;
use crate::price::*;
use crate::cdp::*;
use ic_cdk::{caller, print};
use std::cell::RefCell;

thread_local! {
    pub static TOTAL_MINT_EVENTS: RefCell<u64> = RefCell::new(0);
    pub static TOTAL_PROTOCOL_REVENUE: RefCell<u64> = RefCell::new(0);
}

/// Calculate maximum mintable Bollar amount for a given CDP
pub fn calculate_max_mintable(
    cdp: &CDP,
    btc_price_cents: u64,
    max_collateral_ratio: u32,
) -> u64 {
    cdp.max_mintable_amount(btc_price_cents, max_collateral_ratio)
}

/// Validate mint amount against collateral ratio requirements
pub fn validate_mint_amount(
    cdp: &CDP,
    requested_amount: u64,
    btc_price_cents: u64,
    max_collateral_ratio: u32,
    min_mint_amount: u64,
) -> Result<(), ProtocolError> {
    if requested_amount == 0 {
        return Err(ProtocolError::InvalidAmount);
    }
    
    if requested_amount < min_mint_amount {
        return Err(ProtocolError::AmountTooSmall(requested_amount, min_mint_amount));
    }
    
    let max_mintable = calculate_max_mintable(cdp, btc_price_cents, max_collateral_ratio);
    if requested_amount > max_mintable {
        let current_ratio = cdp.calculate_collateral_ratio(btc_price_cents);
        return Err(ProtocolError::InsufficientCollateral {
            required: max_collateral_ratio,
            actual: current_ratio,
        });
    }
    
    Ok(())
}

/// Execute mint operation - safe version without state mutation
pub fn execute_mint_safe(
    mut cdp: CDP,
    amount_cents: u64,
    btc_price_cents: u64,
    config: &SystemConfig,
) -> Result<CDP, ProtocolError> {
    validate_mint_amount(
        &cdp,
        amount_cents,
        btc_price_cents,
        config.max_collateral_ratio,
        config.min_mint_amount,
    )?;
    
    // Calculate new collateral ratio for verification
    let new_minted_amount = cdp.minted_amount + amount_cents;
    let collateral_value_cents = cdp.collateral_amount * btc_price_cents / 100_000_000;
    let new_ratio = (collateral_value_cents * 10_000 / new_minted_amount) as u32;
    
    // Ensure ratio stays above liquidation threshold
    if new_ratio < config.liquidation_threshold {
        return Err(ProtocolError::InsufficientCollateral {
            required: config.max_collateral_ratio,
            actual: new_ratio,
        });
    }
    
    cdp.minted_amount = new_minted_amount;
    cdp.updated_at = ic_cdk::api::time();
    Ok(cdp)
}

/// Mint Bollar against existing CDP collateral
#[update]
fn mint_bollar(cdp_id: u64, amount_cents: u64) -> ApiResponse<u64> {
    let caller = caller();
    let config = SYSTEM_CONFIG.with(|c| c.borrow().clone());
    let btc_price = PRICE_ORACLE.with(|oracle| {
        oracle.borrow_mut().get_btc_price()
            .unwrap_or_else(|_| 65_000_000)
    });
    
    CDPS.with(|cdps| {
        let mut cdps_ref = cdps.borrow_mut();
        let mut cdp = match cdps_ref.get(&cdp_id) {
            Some(cdp) => cdp,
            None => return ApiResponse::error(ProtocolError::CDPNotFound(cdp_id)),
        };
        
        // Verify ownership
        if cdp.owner != caller {
            return ApiResponse::error(ProtocolError::UnauthorizedAccess);
        }
        
        // Check if liquidated
        if cdp.is_liquidated {
            return ApiResponse::error(ProtocolError::CDPAlreadyLiquidated(cdp_id));
        }
        
        match execute_mint_safe(cdp.clone(), amount_cents, btc_price, &config) {
            Ok(updated_cdp) => {
                cdps_ref.insert(cdp_id, updated_cdp.clone());
                print(format!("Minted {} Bollar cents against CDP {}", amount_cents, cdp_id));
                ApiResponse::success(updated_cdp.minted_amount)
            }
            Err(e) => ApiResponse::error(e),
        }
    })
}

/// Get mint preview for a CDP without actual minting
#[query]  
fn get_mint_preview(cdp_id: u64, amount_cents: u64) -> ApiResponse<MintPreview> {
    let config = SYSTEM_CONFIG.with(|c| c.borrow().clone());
    let btc_price = PRICE_ORACLE.with(|oracle| {
        oracle.borrow_mut().get_btc_price()
            .unwrap_or_else(|_| 65_000_000)
    });
    
    CDPS.with(|cdps| {
        let cdps_ref = cdps.borrow();
        let cdp = match cdps_ref.get(&cdp_id) {
            Some(cdp) => cdp,
            None => return ApiResponse::error(ProtocolError::CDPNotFound(cdp_id)),
        };
        
        match calculate_mint_preview(cdp, amount_cents, btc_price, &config) {
            Ok(preview) => ApiResponse::success(preview),
            Err(e) => ApiResponse::error(e),
        }
    })
}

/// Calculate mint preview with detailed information
pub fn calculate_mint_preview(
    cdp: &CDP,
    amount_cents: u64,
    btc_price_cents: u64,
    config: &SystemConfig,
) -> Result<MintPreview, ProtocolError> {
    validate_mint_amount(
        cdp,
        amount_cents,
        btc_price_cents,
        config.max_collateral_ratio,
        config.min_mint_amount,
    )?;
    
    let max_mintable = calculate_max_mintable(cdp, btc_price_cents, config.max_collateral_ratio);
    let new_minted_amount = cdp.minted_amount + amount_cents;
    let collateral_value_cents = cdp.collateral_amount * btc_price_cents / 100_000_000;
    let new_ratio = (collateral_value_cents * 10_000 / new_minted_amount) as u32;
    
    Ok(MintPreview {
        requested_amount: amount_cents,
        max_mintable,
        new_total_minted: new_minted_amount,
        new_collateral_ratio: new_ratio,
        collateral_value_cents,
        liquidation_threshold: config.liquidation_threshold,
    })
}

/// Preview structure for mint operations
#[derive(CandidType, Serialize, SerdeDeserialize, Debug)]
pub struct MintPreview {
    pub requested_amount: u64,
    pub max_mintable: u64,
    pub new_total_minted: u64,
    pub new_collateral_ratio: u32,
    pub collateral_value_cents: u64,
    pub liquidation_threshold: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;
    
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
            minted_amount: 0,
            created_at: ic_cdk::api::time(),
            updated_at: ic_cdk::api::time(),
            is_liquidated: false,
        }
    }
    
    #[test]
    fn test_calculate_max_mintable() {
        let cdp = test_cdp();
        let btc_price = 65_000_000; // $65,000
        let max_ratio = 9000; // 90%
        let max_mintable = calculate_max_mintable(&cdp, btc_price, max_ratio
        );
        assert_eq!(max_mintable, 585_000); // $585 at 90% LTV
    }
    
    #[test]
    fn test_validate_mint_amount_success() {
        let cdp = test_cdp();
        let btc_price = 65_000_000;
        let config = test_config();
        let result = validate_mint_amount(
            &cdp,
            500_000, // $500 mint
            btc_price,
            config.max_collateral_ratio,
            config.min_mint_amount,
        );
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_validate_mint_amount_insufficient_collateral() {
        let mut cdp = test_cdp();
        cdp.minted_amount = 580_000; // Already have $580 minted
        let btc_price = 65_000_000;
        let config = test_config();
        let result = validate_mint_amount(
            &cdp,
            100_000, // Try to mint $100 more
            btc_price,
            config.max_collateral_ratio,
            config.min_mint_amount,
        );
        assert!(matches!(result, Err(ProtocolError::InsufficientCollateral { .. })));
    }
    
    #[test]
    fn test_validate_mint_amount_too_small() {
        let cdp = test_cdp();
        let btc_price = 65_000_000;
        let mut config = test_config();
        config.min_mint_amount = 5_000; // $50 minimum
        let result = validate_mint_amount(
            &cdp,
            100, // Too small amount
            btc_price,
            config.max_collateral_ratio,
            config.min_mint_amount,
        );
        assert!(matches!(result, Err(ProtocolError::AmountTooSmall(_, _))));
    }
    
    #[test]
    fn test_execute_mint_safe_success() {
        let cdp = test_cdp();
        let btc_price = 65_000_000;
        let config = test_config();
        let result = execute_mint_safe(cdp, 500_000, btc_price, &config);
        assert!(result.is_ok());
        let updated_cdp = result.unwrap();
        assert_eq!(updated_cdp.minted_amount, 500_000);
    }
    
    #[test]
    fn test_execute_mint_safe_liquidation_risk() {
        let mut cdp = test_cdp();
        cdp.minted_amount = 580_000; // Almost at max LTV
        let btc_price = 65_000_000;
        let config = test_config();
        let result = execute_mint_safe(cdp, 100_000, btc_price, &config);
        assert!(matches!(result, Err(ProtocolError::InsufficientCollateral { .. })));
    }
    
    #[test]
    fn test_calculate_mint_preview() {
        let cdp = test_cdp();
        let btc_price = 65_000_000;
        let config = test_config();
        let preview = calculate_mint_preview(&cdp, 500_000, btc_price, &config).unwrap();
        
        assert_eq!(preview.requested_amount, 500_000);
        assert_eq!(preview.max_mintable, 585_000);
        assert_eq!(preview.new_total_minted, 500_000);
        assert_eq!(preview.new_collateral_ratio, 13000); // 130%
        assert_eq!(preview.collateral_value_cents, 650_000);
        assert_eq!(preview.liquidation_threshold, 8500);
    }
    
    #[test]
    fn test_zero_amount_validation() {
        let cdp = test_cdp();
        let btc_price = 65_000_000;
        let config = test_config();
        let result = validate_mint_amount(
            &cdp,
            0,
            btc_price,
            config.max_collateral_ratio,
            config.min_mint_amount,
        );
        assert!(matches!(result, Err(ProtocolError::InvalidAmount)));
    }
    
    #[test]
    fn test_exact_max_mintable() {
        let cdp = test_cdp();
        let btc_price = 65_000_000;
        let config = test_config();
        let max_mintable = calculate_max_mintable(&cdp, btc_price, config.max_collateral_ratio
        );
        
        let result = validate_mint_amount(
            &cdp,
            max_mintable,
            btc_price,
            config.max_collateral_ratio,
            config.min_mint_amount,
        );
        assert!(result.is_ok());
    }
}