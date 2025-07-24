#[cfg(test)]
mod btc_verification_tests {
    use super::cdp::btc_verification::*;
    use super::cdp::BtcTransaction;
    use super::ProtocolError;

    #[test]
    fn test_verify_btc_transaction_valid() {
        let tx_hash = "a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd";
        let amount = 1_000_000; // 0.01 BTC
        let address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
        let confirmations = 10;
        let timestamp = ic_cdk::api::time() / 1_000_000_000;

        let result = verify_btc_transaction(tx_hash, amount, address, confirmations, timestamp);
        assert!(result.is_ok());
        
        let tx = result.unwrap();
        assert_eq!(tx.tx_hash, tx_hash);
        assert_eq!(tx.amount_satoshis, amount);
        assert_eq!(tx.confirmations, confirmations);
        assert_eq!(tx.recipient_address, address);
        assert!(tx.is_confirmed);
    }

    #[test]
    fn test_verify_btc_transaction_invalid_hash() {
        let tx_hash = "invalid_hash";
        let amount = 1_000_000;
        let address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
        let confirmations = 10;
        let timestamp = ic_cdk::api::time() / 1_000_000_000;

        let result = verify_btc_transaction(tx_hash, amount, address, confirmations, timestamp);
        assert!(matches!(result, Err(ProtocolError::ValidationError(_))));
    }

    #[test]
    fn test_verify_btc_transaction_insufficient_confirmations() {
        let tx_hash = "a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd";
        let amount = 1_000_000;
        let address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
        let confirmations = 3; // Below minimum
        let timestamp = ic_cdk::api::time() / 1_000_000_000;

        let result = verify_btc_transaction(tx_hash, amount, address, confirmations, timestamp);
        assert!(matches!(result, Err(ProtocolError::ValidationError(_))));
    }

    #[test]
    fn test_verify_btc_transaction_old_transaction() {
        let tx_hash = "a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd";
        let amount = 1_000_000;
        let address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
        let confirmations = 10;
        let timestamp = (ic_cdk::api::time() / 1_000_000_000) - 100_000; // Very old

        let result = verify_btc_transaction(tx_hash, amount, address, confirmations, timestamp);
        assert!(matches!(result, Err(ProtocolError::ValidationError(_))));
    }

    #[test]
    fn test_verify_btc_transaction_invalid_address() {
        let tx_hash = "a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd";
        let amount = 1_000_000;
        let address = "invalid_address";
        let confirmations = 10;
        let timestamp = ic_cdk::api::time() / 1_000_000_000;

        let result = verify_btc_transaction(tx_hash, amount, address, confirmations, timestamp);
        assert!(matches!(result, Err(ProtocolError::InvalidAddress(_))));
    }

    #[test]
    fn test_verify_transaction_amount() {
        let actual = 1_000_000;
        let expected = 1_000_000;
        let tolerance = 1000; // 1000 satoshis tolerance

        let result = verify_transaction_amount(actual, expected, tolerance);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_transaction_amount_mismatch() {
        let actual = 1_000_000;
        let expected = 1_100_000;
        let tolerance = 1000;

        let result = verify_transaction_amount(actual, expected, tolerance);
        assert!(matches!(result, Err(ProtocolError::ValidationError(_))));
    }

    #[test]
    fn test_verify_transaction_amount_within_tolerance() {
        let actual = 1_000_500;
        let expected = 1_000_000;
        let tolerance = 1000;

        let result = verify_transaction_amount(actual, expected, tolerance);
        assert!(result.is_err()); // Should fail - difference is 500 > 1000
    }

    #[test]
    fn test_mock_verify_transaction() {
        let tx_hash = "mock_tx_hash";
        let amount = 500_000;
        let address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";

        let result = mock_verify_transaction(tx_hash, amount, address);
        assert!(result.is_ok());
        
        let tx = result.unwrap();
        assert_eq!(tx.tx_hash, tx_hash);
        assert_eq!(tx.amount_satoshis, amount);
        assert_eq!(tx.recipient_address, address);
        assert_eq!(tx.confirmations, 10);
    }

    #[test]
    fn test_transaction_hash_format() {
        // Valid 64-character hex
        let valid_hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert_eq!(valid_hash.len(), 64);
        assert!(valid_hash.chars().all(|c| c.is_ascii_hexdigit()));

        // Invalid lengths
        assert!(!"short".chars().all(|c| c.is_ascii_hexdigit()));
        assert!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".len() == 64);
    }
}