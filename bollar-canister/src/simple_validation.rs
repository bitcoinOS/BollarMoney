//! Simple standalone validation for CDP creation

/// System configuration
#[derive(Debug, Clone)]
pub struct SystemConfig {
    pub max_collateral_ratio: u32, // Basis points (9000 = 90%)
    pub min_collateral_amount: u64, // Minimum BTC amount in satoshis
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            max_collateral_ratio: 9000, // 90%
            min_collateral_amount: 100_000, // 0.001 BTC
        }
    }
}

/// CDP structure
#[derive(Debug, Clone)]
pub struct CDP {
    pub id: u64,
    pub owner: [u8; 32], // Simplified owner identifier
    pub collateral_amount: u64, // BTC amount in satoshis
    pub minted_amount: u64, // Bollar amount in cents
    pub is_liquidated: bool,
}

/// Validation errors
#[derive(Debug, PartialEq)]
pub enum ValidationError {
    InvalidAddress(String),
    AmountTooSmall(u64, u64),
    InvalidAmount,
}

/// Validate BTC address format
pub fn validate_btc_address(address: &str) -> Result<(), ValidationError> {
    if address.is_empty() {
        return Err(ValidationError::InvalidAddress("Address cannot be empty".to_string()));
    }

    let address = address.trim();

    // P2PKH addresses (Legacy) - start with 1
    if address.starts_with('1') {
        if address.len() != 34 {
            return Err(ValidationError::InvalidAddress("P2PKH address must be 34 characters".to_string()));
        }

        // Check base58 characters (alphanumeric, excluding 0, O, I, l)
        let valid_base58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        for c in address.chars() {
            if !valid_base58.contains(c) {
                return Err(ValidationError::InvalidAddress("P2PKH address contains invalid characters".to_string()));
            }
        }

        return Ok(());
    }

    // P2SH addresses - start with 3
    if address.starts_with('3') {
        if address.len() != 34 {
            return Err(ValidationError::InvalidAddress("P2SH address must be 34 characters".to_string()));
        }

        // Check base58 characters
        let valid_base58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        for c in address.chars() {
            if !valid_base58.contains(c) {
                return Err(ValidationError::InvalidAddress("P2SH address contains invalid characters".to_string()));
            }
        }

        return Ok(());
    }

    // Bech32 addresses (SegWit) - start with bc1
    if address.starts_with("bc1") {
        if address.len() < 39 || address.len() > 62 {
            return Err(ValidationError::InvalidAddress("Bech32 address length invalid".to_string()));
        }

        // Check bech32 characters (lowercase alphanumeric excluding 1, b, i, o)
        let valid_bech32 = "023456789acdefghjklmnpqrstuvwxyz";
        for c in address[3..].chars() {
            if !valid_bech32.contains(c) {
                return Err(ValidationError::InvalidAddress("Bech32 address contains invalid characters".to_string()));
            }
        }

        return Ok(());
    }

    Err(ValidationError::InvalidAddress("Invalid BTC address format".to_string()))
}

/// Validate collateral amount within bounds
pub fn validate_collateral_amount(amount: u64) -> Result<(), ValidationError> {
    const MIN_COLLATERAL: u64 = 100_000; // 0.001 BTC in satoshis
    const MAX_COLLATERAL: u64 = 100_000_000_000; // 1000 BTC in satoshis

    if amount < MIN_COLLATERAL {
        return Err(ValidationError::AmountTooSmall(amount, MIN_COLLATERAL));
    }

    if amount > MAX_COLLATERAL {
        return Err(ValidationError::InvalidAmount);
    }

    Ok(())
}

/// Create CDP with validation
pub fn create_cdp(
    owner: [u8; 32],
    collateral_amount: u64,
    btc_price_cents: u64,
    btc_address: &str,
) -> Result<CDP, ValidationError> {
    // Validate BTC address
    validate_btc_address(btc_address)?;

    // Validate collateral amount
    validate_collateral_amount(collateral_amount)?;

    // Calculate maximum mintable amount at 90% LTV
    let collateral_value_cents = collateral_amount * btc_price_cents / 100_000_000;
    let max_mintable = collateral_value_cents * 9000 / 10_000;

    // Ensure minimum mint amount ($0.01)
    if max_mintable < 1_000 {
        return Err(ValidationError::AmountTooSmall(max_mintable, 1_000));
    }

    Ok(CDP {
        id: 1, // Simplified for testing
        owner,
        collateral_amount,
        minted_amount: 0,
        is_liquidated: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btc_address_validation() {
        // Valid P2PKH addresses
        assert!(validate_btc_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa").is_ok());
        assert!(validate_btc_address("1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2").is_ok());
        assert!(validate_btc_address("1FeexV6bAHb8ybZjqQMjJrcCrHGW9sb6uF").is_ok());

        // Valid P2SH addresses
        assert!(validate_btc_address("3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy").is_ok());
        assert!(validate_btc_address("3FZbgi29cpjq2GjdwV8eyHuJJnkLtktZc5").is_ok());
        assert!(validate_btc_address("3QJmV3qfvL9SuYo34YihAf3sRCW3qSinyC").is_ok());

        // Valid Bech32 addresses
        assert!(validate_btc_address("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4").is_ok());
        assert!(validate_btc_address("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq").is_ok());

        // Invalid addresses
        assert!(matches!(validate_btc_address(""), Err(ValidationError::InvalidAddress(_))));
        assert!(matches!(validate_btc_address("invalid"), Err(ValidationError::InvalidAddress(_))));
        assert!(matches!(validate_btc_address("0x1234567890abcdef"), Err(ValidationError::InvalidAddress(_))));
        assert!(matches!(validate_btc_address("2MzQwSSnBHWHqSAqtTVQ6v47XtaisrJa1Vc"), Err(ValidationError::InvalidAddress(_))));
    }

    #[test]
    fn test_collateral_amount_validation() {
        // Valid amounts
        assert!(validate_collateral_amount(100_000).is_ok()); // 0.001 BTC (minimum)
        assert!(validate_collateral_amount(1_000_000).is_ok()); // 0.01 BTC
        assert!(validate_collateral_amount(100_000_000).is_ok()); // 1 BTC
        assert!(validate_collateral_amount(100_000_000_000).is_ok()); // 1000 BTC (maximum)

        // Invalid amounts
        assert!(matches!(validate_collateral_amount(50_000), Err(ValidationError::AmountTooSmall(_, _)))); // below minimum
        assert!(matches!(validate_collateral_amount(200_000_000_000), Err(ValidationError::InvalidAmount))); // above maximum
    }

    #[test]
    fn test_cdp_creation() {
        let owner = [0u8; 32];
        let btc_price = 50_000_000; // $50,000 per BTC

        // Valid CDP creation
        let result = create_cdp(owner, 1_000_000, btc_price, "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa");
        assert!(result.is_ok());

        let cdp = result.unwrap();
        assert_eq!(cdp.collateral_amount, 1_000_000); // 0.01 BTC
        assert_eq!(cdp.minted_amount, 0);

        // CDP creation with insufficient collateral
        let result = create_cdp(owner, 50_000, btc_price, "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa");
        assert!(matches!(result, Err(ValidationError::AmountTooSmall(_, _))));

        // CDP creation with excessive collateral
        let result = create_cdp(owner, 200_000_000_000, btc_price, "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa");
        assert!(matches!(result, Err(ValidationError::InvalidAmount)));

        // CDP creation with invalid BTC address
        let result = create_cdp(owner, 1_000_000, btc_price, "invalid_address");
        assert!(matches!(result, Err(ValidationError::InvalidAddress(_))));
    }

    #[test]
    fn test_calculations() {
        let owner = [0u8; 32];
        let btc_price = 65_000_000; // $65,000 per BTC
        let collateral = 1_000_000; // 0.01 BTC

        let result = create_cdp(owner, collateral, btc_price, "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa");
        let cdp = result.unwrap();

        // Verify calculations
        let collateral_value_cents = collateral * btc_price / 100_000_000; // $650
        let max_mintable = collateral_value_cents * 9000 / 10_000; // $585
        
        assert!(collateral_value_cents == 650_000); // $650 in cents
        assert!(max_mintable == 585_000); // $585 in cents
    }
}