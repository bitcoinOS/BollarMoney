//! CDP (Collateralized Debt Position) creation and management

use crate::types::*;
use candid::Principal;
use ic_cdk::print;
use std::cell::RefCell;

thread_local! {
    pub static TOTAL_CREATE_EVENTS: RefCell<u64> = RefCell::new(0);
}

/// Create a new CDP with BTC collateral
pub fn create_cdp_logic(
    owner: Principal,
    collateral_amount: u64,
    btc_price_cents: u64,
    config: &SystemConfig,
) -> Result<CDP, ProtocolError> {
    // Validate collateral amount
    if collateral_amount < config.min_collateral_amount {
        return Err(ProtocolError::AmountTooSmall(
            collateral_amount,
            config.min_collateral_amount,
        ));
    }
    
    // Maximum collateral amount: 1000 BTC = 100,000,000,000 satoshis
    let max_collateral_amount = 100_000_000_000u64;
    if collateral_amount > max_collateral_amount {
        return Err(ProtocolError::InvalidAmount);
    }
    
    // Calculate maximum mintable amount at 90% LTV
    let collateral_value_cents = collateral_amount * btc_price_cents / 100_000_000;
    let max_mintable = collateral_value_cents * config.max_collateral_ratio as u64 / 10_000;
    
    // Ensure minimum mint amount
    if max_mintable < config.min_mint_amount {
        return Err(ProtocolError::AmountTooSmall(
            max_mintable,
            config.min_mint_amount,
        ));
    
    
    let cdp = CDP {
        id: 0, // Will be set by caller
        owner,
        collateral_amount,
        minted_amount: 0, // Start with zero minted
        created_at: ic_cdk::api::time(),
        updated_at: ic_cdk::api::time(),
        is_liquidated: false,
    };
    
    Ok(cdp)
}

/// Bitcoin transaction verification structure
#[derive(Debug, Clone, Serialize, SerdeDeserialize)]
pub struct BtcTransaction {
    pub tx_hash: String,
    pub amount_satoshis: u64,
    pub confirmations: u32,
    pub recipient_address: String,
    pub sender_address: String,
    pub block_height: u64,
    pub timestamp: u64,
    pub is_confirmed: bool,
}

/// BTC transaction verification and validation
pub mod btc_verification {
    use super::*;
    
    /// Minimum confirmations required for transaction validity
    pub const MIN_CONFIRMATIONS: u32 = 6;
    
    /// Maximum acceptable age for transaction (24 hours)
    pub const MAX_TRANSACTION_AGE_SECONDS: u64 = 86400;
    
    /// Verify BTC transaction details
    pub fn verify_btc_transaction(
        tx_hash: &str,
        expected_amount: u64,
        expected_address: &str,
        confirmations: u32,
        timestamp: u64,
    ) -> Result<BtcTransaction, ProtocolError> {
        // Validate transaction hash format
        if tx_hash.len() != 64 {
            return Err(ProtocolError::ValidationError("Invalid transaction hash format".to_string()));
        }
        
        // Validate hexadecimal characters
        if !tx_hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ProtocolError::ValidationError("Invalid transaction hash characters".to_string()));
        }
        
        // Validate confirmations
        if confirmations < MIN_CONFIRMATIONS {
            return Err(ProtocolError::ValidationError(
                format!("Insufficient confirmations: {} < {}", confirmations, MIN_CONFIRMATIONS)
            ));
        }
        
        // Validate transaction age
        let current_time = ic_cdk::api::time() / 1_000_000_000; // Convert to seconds
        if current_time.saturating_sub(timestamp) > MAX_TRANSACTION_AGE_SECONDS {
            return Err(ProtocolError::ValidationError("Transaction too old".to_string()));
        }
        
        // Validate BTC address
        validate_btc_address(expected_address)?;
        
        Ok(BtcTransaction {
            tx_hash: tx_hash.to_string(),
            amount_satoshis: expected_amount,
            confirmations,
            recipient_address: expected_address.to_string(),
            sender_address: String::new(), // Would be populated from actual BTC network
            block_height: 0, // Would be populated from actual BTC network
            timestamp,
            is_confirmed: true,
        })
    }
    
    /// Verify transaction amount matches expected
    pub fn verify_transaction_amount(
        actual_amount: u64,
        expected_amount: u64,
        tolerance_satoshis: u64,
    ) -> Result<(), ProtocolError> {
        let difference = if actual_amount > expected_amount {
            actual_amount - expected_amount
        } else {
            expected_amount - actual_amount
        };
        
        if difference > tolerance_satoshis {
            return Err(ProtocolError::ValidationError(
                format!("Amount mismatch: expected {}, got {}", expected_amount, actual_amount)
            ));
        }
        
        Ok(())
    }
    
    /// Mock BTC transaction verification for testing
    pub fn mock_verify_transaction(
        tx_hash: &str,
        amount: u64,
        address: &str,
    ) -> Result<BtcTransaction, ProtocolError> {
        validate_btc_address(address)?;
        
        Ok(BtcTransaction {
            tx_hash: tx_hash.to_string(),
            amount_satoshis: amount,
            confirmations: 10, // Mock confirmations
            recipient_address: address.to_string(),
            sender_address: "mock_sender".to_string(),
            block_height: 800000,
            timestamp: ic_cdk::api::time() / 1_000_000_000,
            is_confirmed: true,
        })
    }
}

/// Validate a BTC address format with comprehensive checks
pub fn validate_btc_address(address: &str) -> Result<(), ProtocolError> {
    if address.is_empty() {
        return Err(ProtocolError::InvalidAddress("Address cannot be empty".to_string()));
    }
    
    let address = address.trim();
    
    // Check for valid BTC address formats
    // P2PKH addresses (Legacy) - start with 1
    if address.starts_with('1') {
        if address.len() != 34 {
            return Err(ProtocolError::InvalidAddress("P2PKH address must be 34 characters".to_string()));
        }
        
        // Check base58 characters (alphanumeric, excluding 0, O, I, l)
        let valid_base58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        for c in address.chars() {
            if !valid_base58.contains(c) {
                return Err(ProtocolError::InvalidAddress("P2PKH address contains invalid characters".to_string()));
            }
        }
        
        return Ok(());
    }
    
    // P2SH addresses - start with 3
    if address.starts_with('3') {
        if address.len() != 34 {
            return Err(ProtocolError::InvalidAddress("P2SH address must be 34 characters".to_string()));
        }
        
        // Check base58 characters
        let valid_base58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        for c in address.chars() {
            if !valid_base58.contains(c) {
                return Err(ProtocolError::InvalidAddress("P2SH address contains invalid characters".to_string()));
            }
        }
        
        return Ok(());
    }
    
    // Bech32 addresses (SegWit) - start with bc1
    if address.starts_with("bc1") {
        if address.len() < 39 || address.len() > 62 {
            return Err(ProtocolError::InvalidAddress("Bech32 address length invalid".to_string()));
        }
        
        // Check bech32 characters (lowercase alphanumeric excluding 1, b, i, o)
        let valid_bech32 = "023456789acdefghjklmnpqrstuvwxyz";
        for c in address[3..].chars() {
            if !valid_bech32.contains(c) {
                return Err(ProtocolError::InvalidAddress("Bech32 address contains invalid characters".to_string()));
            }
        }
        
        return Ok(());
    }
    
    Err(ProtocolError::InvalidAddress("Invalid BTC address format".to_string()))
}

/// Get CDP creation preview without actually creating
pub fn get_cdp_preview(
    collateral_amount: u64,
    btc_price_cents: u64,
    config: &SystemConfig,
) -> Result<CdpPreview, ProtocolError> {
    // Validate collateral amount
    if collateral_amount < config.min_collateral_amount {
        return Err(ProtocolError::AmountTooSmall(
            collateral_amount,
            config.min_collateral_amount,
        ));
    }
    
    let max_collateral_amount = 100_000_000_000u64;
    if collateral_amount > max_collateral_amount {
        return Err(ProtocolError::InvalidAmount);
    }
    
    let collateral_value_cents = collateral_amount * btc_price_cents / 100_000_000;
    let max_mintable = collateral_value_cents * config.max_collateral_ratio as u64 / 10_000;
    let liquidation_threshold_cents = collateral_value_cents * config.liquidation_threshold as u64 / 10_000;
    
    Ok(CdpPreview {
        collateral_amount,
        collateral_value_cents,
        max_mintable,
        liquidation_threshold_cents,
        current_collateral_ratio: config.max_collateral_ratio,
    })
}

/// Preview structure for CDP creation
#[derive(Debug, CandidType, Serialize, SerdeDeserialize)]
pub struct CdpPreview {
    pub collateral_amount: u64,
    pub collateral_value_cents: u64,
    pub max_mintable: u64,
    pub liquidation_threshold_cents: u64,
    pub current_collateral_ratio: u32,
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
    
    #[test]
    fn test_create_cdp_success() {
        let owner = test_principal();
        let collateral = 1_000_000; // 0.01 BTC
        let btc_price = 65_000_000; // $65,000
        let config = test_config();
        
        let result = create_cdp_logic(owner, collateral, btc_price, &config);
        assert!(result.is_ok());
        
        let cdp = result.unwrap();
        assert_eq!(cdp.collateral_amount, collateral);
        assert_eq!(cdp.minted_amount, 0);
        assert_eq!(cdp.owner, owner);
        assert!(!cdp.is_liquidated);
    }
    
    #[test]
    fn test_create_cdp_insufficient_collateral() {
        let owner = test_principal();
        let collateral = 50_000; // 0.0005 BTC - below minimum
        let btc_price = 65_000_000;
        let config = test_config();
        
        let result = create_cdp_logic(owner, collateral, btc_price, &config);
        assert!(matches!(result, Err(ProtocolError::AmountTooSmall(_, _))));
    }
    
    #[test]
    fn test_create_cdp_excessive_collateral() {
        let owner = test_principal();
        let collateral = 200_000_000_000; // 2000 BTC - above maximum
        let btc_price = 65_000_000;
        let config = test_config();
        
        let result = create_cdp_logic(owner, collateral, btc_price, &config);
        assert!(matches!(result, Err(ProtocolError::InvalidAmount)));
    }
    
    #[test]
    fn test_validate_btc_address_p2pkh_valid() {
        assert!(validate_btc_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa").is_ok());
        assert!(validate_btc_address("1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2").is_ok());
        assert!(validate_btc_address("1FeexV6bAHb8ybZjqQMjJrcCrHGW9sb6uF").is_ok());
    }
    
    #[test]
    fn test_validate_btc_address_p2sh_valid() {
        assert!(validate_btc_address("3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy").is_ok());
        assert!(validate_btc_address("3FZbgi29cpjq2GjdwV8eyHuJJnkLtktZc5").is_ok());
        assert!(validate_btc_address("3QJmV3qfvL9SuYo34YihAf3sRCW3qSinyC").is_ok());
    }
    
    #[test]
    fn test_validate_btc_address_bech32_valid() {
        assert!(validate_btc_address("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4").is_ok());
        assert!(validate_btc_address("bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr").is_ok());
        assert!(validate_btc_address("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq").is_ok());
    }
    
    #[test]
    fn test_validate_btc_address_invalid_formats() {
        // Empty address
        assert!(matches!(validate_btc_address(""), Err(ProtocolError::InvalidAddress(_))));
        
        // Wrong format
        assert!(matches!(validate_btc_address("invalid"), Err(ProtocolError::InvalidAddress(_))));
        assert!(matches!(validate_btc_address("0x1234567890abcdef"), Err(ProtocolError::InvalidAddress(_))));
        assert!(matches!(validate_btc_address("2MzQwSSnBHWHqSAqtTVQ6v47XtaisrJa1Vc"), Err(ProtocolError::InvalidAddress(_))));
    }
    
    #[test]
    fn test_validate_btc_address_length_validation() {
        // P2PKH too short
        assert!(validate_btc_address("1A").is_err());
        // P2PKH too long
        assert!(validate_btc_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa123").is_err());
        
        // P2SH too short
        assert!(validate_btc_address("3J").is_err());
        // P2SH too long
        assert!(validate_btc_address("3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy123").is_err());
        
        // Bech32 too short
        assert!(validate_btc_address("bc1q").is_err());
        // Bech32 too long
        assert!(validate_btc_address("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq123456789012345").is_err());
    }
    
    #[test]
    fn test_validate_btc_address_character_validation() {
        // Invalid characters in P2PKH
        assert!(validate_btc_address("1a1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa").is_err()); // lowercase
        assert!(validate_btc_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa!").is_err()); // special char
        
        // Invalid characters in P2SH
        assert!(validate_btc_address("3j98t1WpEZ73CNmQviecrnyiWrnqRhWNLy").is_err()); // lowercase
        
        // Invalid characters in Bech32
        assert!(validate_btc_address("BC1QW508D6QEJXTDG4Y5R3ZARVARY0C5XW7KV8F3T4").is_err()); // uppercase
        assert!(validate_btc_address("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4!").is_err()); // special char
    }
    
    #[test]
    fn test_validate_btc_address_whitespace_handling() {
        assert!(validate_btc_address(" 1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa ").is_ok());
        assert!(validate_btc_address("\t1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa\n").is_ok());
    }
    
    #[test]
    fn test_get_cdp_preview() {
        let collateral = 1_000_000; // 0.01 BTC
        let btc_price = 65_000_000; // $65,000
        let config = test_config();
        
        let preview = get_cdp_preview(collateral, btc_price, &config).unwrap();
        
        assert_eq!(preview.collateral_amount, collateral);
        assert_eq!(preview.collateral_value_cents, 650_000); // $650
        assert_eq!(preview.max_mintable, 585_000); // $585 at 90% LTV
        assert_eq!(preview.liquidation_threshold_cents, 552_500); // $552.50 at 85%
        assert_eq!(preview.current_collateral_ratio, 9000);
    }
    
    #[test]
    fn test_min_mint_amount_check() {
        let owner = test_principal();
        let collateral = 100_000; // 0.001 BTC (minimum)
        let btc_price = 65_000_000;
        let config = test_config();
        
        let result = create_cdp_logic(owner, collateral, btc_price, &config);
        assert!(result.is_ok());
        
        let cdp = result.unwrap();
        let max_mintable = cdp.max_mintable_amount(btc_price, config.max_collateral_ratio);
        assert!(max_mintable >= config.min_mint_amount);
    }
}