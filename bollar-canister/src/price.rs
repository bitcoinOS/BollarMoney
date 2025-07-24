//! Price oracle and BTC price management

use crate::types::*;
use std::cell::RefCell;

/// Configuration for price validation
#[derive(Debug, Clone)]
pub struct OracleConfig {
    pub max_price_change_percent: u32,
    pub max_age_seconds: u64,
    pub min_confidence_level: u8,
    pub default_price_cents: u64,
}

impl Default for OracleConfig {
    fn default() -> Self {
        Self {
            max_price_change_percent: 10,
            max_age_seconds: 300,
            min_confidence_level: 95,
            default_price_cents: 65_000_000, // $65,000
        }
    }
}

/// Cached price data
#[derive(Debug, Clone)]
pub struct PriceCache {
    pub price_cents: u64,
    pub source: String,
    pub confidence: u8,
    pub timestamp: u64,
    pub expiry: u64,
}

impl PriceCache {
    pub fn new(price_cents: u64, source: String, confidence: u8, ttl_seconds: u64) -> Self {
        let timestamp = ic_cdk::api::time();
        let expiry = timestamp + ttl_seconds * 1_000_000_000; // Convert to nanoseconds
        Self {
            price_cents,
            source,
            confidence,
            timestamp,
            expiry,
        }
    }

    pub fn is_expired(&self) -> bool {
        ic_cdk::api::time() > self.expiry
    }

    pub fn is_valid(&self) -> bool {
        !self.is_expired() && self.confidence >= 95
    }
}

/// Price oracle implementation
#[derive(Debug)]
pub struct PriceOracle {
    pub config: OracleConfig,
    pub cache: Option<PriceCache>,
    pub last_price: u64,
}

impl PriceOracle {
    pub fn new(config: OracleConfig) -> Self {
        Self {
            config,
            cache: None,
            last_price: config.default_price_cents,
        }
    }

    pub fn get_btc_price(&mut self) -> Result<u64, ProtocolError> {
        if let Some(cache) = &self.cache {
            if cache.is_valid() {
                self.last_price = cache.price_cents;
                return Ok(cache.price_cents);
            }
        }

        // In production, this would fetch from external oracle
        // For now, return cached or default price
        Ok(self.last_price)
    }

    pub fn get_cached_price(&self) -> Option<u64> {
        self.cache.as_ref()
            .filter(|c| c.is_valid())
            .map(|c| c.price_cents)
    }

    pub fn update_price(&mut self, price_cents: u64, source: String, confidence: u8) -> Result<(), ProtocolError> {
        if confidence < self.config.min_confidence_level {
            return Err(ProtocolError::OraclePriceError("Insufficient confidence level".to_string()));
        }

        // Validate price change
        let price_change = if self.last_price > 0 {
            ((price_cents as i64 - self.last_price as i64).abs() * 100 / self.last_price as i64) as u32
        } else {
            0
        };

        if price_change > self.config.max_price_change_percent {
            return Err(ProtocolError::OraclePriceError("Price change too large".to_string()));
        }

        self.cache = Some(PriceCache::new(
            price_cents,
            source,
            confidence,
            self.config.max_age_seconds,
        ));
        self.last_price = price_cents;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_oracle_initialization() {
        let oracle = PriceOracle::new(OracleConfig::default());
        assert_eq!(oracle.last_price, 65_000_000);
        assert!(oracle.cache.is_none());
    }

    #[test]
    fn test_price_validation() {
        let mut oracle = PriceOracle::new(OracleConfig::default());
        
        // Valid price update
        let result = oracle.update_price(66_000_000, "test".to_string(), 95);
        assert!(result.is_ok());
        assert_eq!(oracle.get_btc_price().unwrap(), 66_000_000);

        // Invalid confidence
        let result = oracle.update_price(67_000_000, "test".to_string(), 90);
        assert!(matches!(result, Err(ProtocolError::OraclePriceError(_))));

        // Too large price change
        let result = oracle.update_price(80_000_000, "test".to_string(), 95);
        assert!(matches!(result, Err(ProtocolError::OraclePriceError(_))));
    }
}