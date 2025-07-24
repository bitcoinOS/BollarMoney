//! CDP Creation Module - Task 1.3 Implementation
//! Complete CDP creation with BTC transaction verification and indexing

use crate::types::*;
use crate::cdp::btc_verification::*;
use candid::Principal;
use ic_cdk::{api::time, print};
use std::collections::HashMap;

/// CDP Creation Request with BTC transaction details
#[derive(Debug, Clone, CandidType, Serialize, SerdeDeserialize)]
pub struct CreateCdpRequest {
    pub btc_tx_hash: String,
    pub btc_address: String,
    pub collateral_amount: u64,
    pub confirmations: u32,
    pub timestamp: u64,
}

/// CDP Creation Response
#[derive(Debug, Clone, CandidType, Serialize, SerdeDeserialize)]
pub struct CreateCdpResponse {
    pub cdp_id: u64,
    pub owner: Principal,
    pub collateral_amount: u64,
    pub max_mintable: u64,
    pub created_at: u64,
}

/// User CDP tracking system
#[derive(Debug, Clone, Default)]
pub struct UserCdpIndex {
    pub user_to_cdps: HashMap<Principal, Vec<u64>>,
    pub cdp_count: u64,
}

impl UserCdpIndex {
    pub fn new() -> Self {
        Self {
            user_to_cdps: HashMap::new(),
            cdp_count: 0,
        }
    }

    pub fn add_cdp(&mut self, owner: Principal, cdp_id: u64) {
        self.user_to_cdps
            .entry(owner)
            .or_insert_with(Vec::new)
            .push(cdp_id);
        self.cdp_count += 1;
    }

    pub fn get_user_cdps(&self, owner: Principal) -> Vec<u64> {
        self.user_to_cdps
            .get(&owner)
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_total_cdps(&self) -> u64 {
        self.cdp_count
    }
}

/// Complete CDP creation with BTC verification
pub fn create_cdp(
    owner: Principal,
    request: CreateCdpRequest,
    btc_price_cents: u64,
    config: &SystemConfig,
    index: &mut UserCdpIndex,
) -> Result<CreateCdpResponse, ProtocolError> {
    
    // Step 1: Validate BTC transaction
    let btc_tx = verify_btc_transaction(
        &request.btc_tx_hash,
        request.collateral_amount,
        &request.btc_address,
        request.confirmations,
        request.timestamp,
    )?;

    // Step 2: Validate collateral amount
    validate_collateral_amount(request.collateral_amount, config)?;

    // Step 3: Calculate minting limits
    let collateral_value_cents = request.collateral_amount * btc_price_cents / 100_000_000;
    let max_mintable = collateral_value_cents * config.max_collateral_ratio as u64 / 10_000;

    // Step 4: Ensure minimum mint amount
    if max_mintable < config.min_mint_amount {
        return Err(ProtocolError::AmountTooSmall(
            max_mintable,
            config.min_mint_amount,
        ));
    }

    // Step 5: Create CDP
    let cdp_id = index.get_total_cdps() + 1;
    let created_at = time();
    
    let cdp = CDP {
        id: cdp_id,
        owner,
        collateral_amount: request.collateral_amount,
        minted_amount: 0,
        created_at,
        updated_at: created_at,
        is_liquidated: false,
    };

    // Step 6: Update user index
    index.add_cdp(owner, cdp_id);

    print(format!(
        "CDP created: ID={}, Owner={}, Collateral={} satoshi, Max Mintable={} cents",
        cdp_id, owner, request.collateral_amount, max_mintable
    ));

    Ok(CreateCdpResponse {
        cdp_id,
        owner,
        collateral_amount: request.collateral_amount,
        max_mintable,
        created_at,
    })
}

/// Validate collateral amount within protocol limits
fn validate_collateral_amount(
    amount: u64,
    config: &SystemConfig,
) -> Result<(), ProtocolError> {
    if amount < config.min_collateral_amount {
        return Err(ProtocolError::AmountTooSmall(
            amount,
            config.min_collateral_amount,
        ));
    }

    const MAX_COLLATERAL_AMOUNT: u64 = 100_000_000_000; // 1000 BTC in satoshis
    if amount > MAX_COLLATERAL_AMOUNT {
        return Err(ProtocolError::InvalidAmount);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    fn test_principal() -> Principal {
        Principal::anonymous()
    }

    fn test_config() -> SystemConfig {
        SystemConfig {
            max_collateral_ratio: 9000, // 90%
            liquidation_threshold: 8500, // 85%
            min_collateral_amount: 100_000, // 0.001 BTC
            min_mint_amount: 1_000, // $0.01
        }
    }

    #[test]
    fn test_successful_cdp_creation() {
        let owner = test_principal();
        let mut index = UserCdpIndex::new();
        let config = test_config();
        let btc_price = 50_000_000; // $50,000 per BTC

        let request = CreateCdpRequest {
            btc_tx_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            btc_address: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
            collateral_amount: 1_000_000, // 0.01 BTC
            confirmations: 10,
            timestamp: 1700000000,
        };

        let result = create_cdp(owner, request, btc_price, &config, &mut index);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.cdp_id, 1);
        assert_eq!(response.collateral_amount, 1_000_000);
        assert!(response.max_mintable > 0);
    }

    #[test]
    fn test_insufficient_collateral() {
        let owner = test_principal();
        let mut index = UserCdpIndex::new();
        let config = test_config();
        let btc_price = 50_000_000;

        let request = CreateCdpRequest {
            btc_tx_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            btc_address: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
            collateral_amount: 50_000, // Below minimum
            confirmations: 10,
            timestamp: 1700000000,
        };

        let result = create_cdp(owner, request, btc_price, &config, &mut index);
        assert!(matches!(result, Err(ProtocolError::AmountTooSmall(_, _))));
    }

    #[test]
    fn test_invalid_btc_transaction() {
        let owner = test_principal();
        let mut index = UserCdpIndex::new();
        let config = test_config();
        let btc_price = 50_000_000;

        let request = CreateCdpRequest {
            btc_tx_hash: "invalid_hash".to_string(), // Invalid format
            btc_address: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
            collateral_amount: 1_000_000,
            confirmations: 10,
            timestamp: 1700000000,
        };

        let result = create_cdp(owner, request, btc_price, &config, &mut index);
        assert!(matches!(result, Err(ProtocolError::ValidationError(_))));
    }

    #[test]
    fn test_user_indexing() {
        let owner = test_principal();
        let mut index = UserCdpIndex::new();
        let config = test_config();
        let btc_price = 50_000_000;

        let request = CreateCdpRequest {
            btc_tx_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            btc_address: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
            collateral_amount: 1_000_000,
            confirmations: 10,
            timestamp: 1700000000,
        };

        let _ = create_cdp(owner, request.clone(), btc_price, &config, &mut index);
        
        let user_cdps = index.get_user_cdps(owner);
        assert_eq!(user_cdps.len(), 1);
        assert_eq!(user_cdps[0], 1);
    }

    #[test]
    fn test_min_mint_amount_check() {
        let owner = test_principal();
        let mut index = UserCdpIndex::new();
        let config = SystemConfig {
            max_collateral_ratio: 9000,
            liquidation_threshold: 8500,
            min_collateral_amount: 100_000,
            min_mint_amount: 10_000, // High minimum for test
        };
        let btc_price = 500_000; // Low price to trigger min mint check

        let request = CreateCdpRequest {
            btc_tx_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            btc_address: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
            collateral_amount: 100_000, // Minimum collateral
            confirmations: 10,
            timestamp: 1700000000,
        };

        let result = create_cdp(owner, request, btc_price, &config, &mut index);
        assert!(matches!(result, Err(ProtocolError::AmountTooSmall(_, _))));
    }
}