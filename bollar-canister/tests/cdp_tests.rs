#[cfg(test)]
mod cdp_tests {
    use super::*;
    use bollar_canister::types::*;
    
    // Import the functions we want to test
    use bollar_canister::cdp::{validate_btc_address, create_cdp_logic};
    
    fn test_principal() -> candid::Principal {
        candid::Principal::anonymous()
    }
    
    fn test_config() -> SystemConfig {
        SystemConfig {
            max_collateral_ratio: 9000,
            liquidation_threshold: 8500,
            liquidation_penalty: 500,
            min_collateral_amount: 100_000, // 0.001 BTC
            min_mint_amount: 1_000,         // $0.01
        }
    }
    
    #[test]
    fn test_btc_address_validation_p2pkh() {
        // Valid P2PKH addresses
        assert!(validate_btc_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa").is_ok());
        assert!(validate_btc_address("1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2").is_ok());
        
        // Invalid P2PKH addresses
        assert!(validate_btc_address("1a1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa").is_err()); // lowercase
        assert!(validate_btc_address("1A").is_err()); // too short
    }
    
    #[test]
    fn test_btc_address_validation_p2sh() {
        // Valid P2SH addresses
        assert!(validate_btc_address("3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy").is_ok());
        assert!(validate_btc_address("3FZbgi29cpjq2GjdwV8eyHuJJnkLtktZc5").is_ok());
        
        // Invalid P2SH addresses
        assert!(validate_btc_address("3j98t1WpEZ73CNmQviecrnyiWrnqRhWNLy").is_err()); // lowercase
    }
    
    #[test]
    fn test_btc_address_validation_bech32() {
        // Valid Bech32 addresses
        assert!(validate_btc_address("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4").is_ok());
        assert!(validate_btc_address("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq").is_ok());
        
        // Invalid Bech32 addresses
        assert!(validate_btc_address("BC1QW508D6QEJXTDG4Y5R3ZARVARY0C5XW7KV8F3T4").is_err()); // uppercase
        assert!(validate_btc_address("bc1q").is_err()); // too short
    }
    
    #[test]
    fn test_btc_address_validation_errors() {
        // Empty address
        assert!(validate_btc_address("").is_err());
        
        // Wrong format
        assert!(validate_btc_address("invalid").is_err());
        assert!(validate_btc_address("0x1234567890abcdef").is_err());
        assert!(validate_btc_address("2MzQwSSnBHWHqSAqtTVQ6v47XtaisrJa1Vc").is_err());
    }
    
    #[test]
    fn test_collateral_amount_validation() {
        let owner = test_principal();
        let btc_price = 50_000_000; // $50,000 per BTC
        let config = test_config();
        
        // Valid amounts
        assert!(create_cdp_logic(owner, 100_000, btc_price, &config).is_ok()); // 0.001 BTC
        assert!(create_cdp_logic(owner, 1_000_000, btc_price, &config).is_ok()); // 0.01 BTC
        assert!(create_cdp_logic(owner, 100_000_000, btc_price, &config).is_ok()); // 1 BTC
        
        // Invalid amounts
        assert!(create_cdp_logic(owner, 50_000, btc_price, &config).is_err()); // below minimum
        assert!(create_cdp_logic(owner, 200_000_000_000, btc_price, &config).is_err()); // above maximum
    }
    
    #[test]
    fn test_cdp_creation_calculation() {
        let owner = test_principal();
        let collateral = 1_000_000; // 0.01 BTC
        let btc_price = 50_000_000; // $50,000 per BTC
        let config = test_config();
        
        let cdp = create_cdp_logic(owner, collateral, btc_price, &config).unwrap();
        
        assert_eq!(cdp.collateral_amount, collateral);
        assert_eq!(cdp.minted_amount, 0);
        assert_eq!(cdp.owner, owner);
        assert!(!cdp.is_liquidated);
        
        // Calculate expected max mintable
        let collateral_value_cents = collateral * btc_price / 100_000_000;
        let expected_max_mintable = collateral_value_cents * config.max_collateral_ratio as u64 / 10_000;
        assert!(expected_max_mintable >= config.min_mint_amount);
    }
}