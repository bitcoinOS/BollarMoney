// input_validation.rs - 输入验证和边界检查
// 这个模块提供全面的输入验证和边界检查功能

use crate::{Error, Result, types::*};
use candid::Principal;
use std::collections::HashSet;

// 验证配置
pub struct ValidationConfig {
    pub min_btc_amount: u64,
    pub max_btc_amount: u64,
    pub min_bollar_amount: u64,
    pub max_bollar_amount: u64,
    pub min_collateral_ratio: u8,
    pub max_collateral_ratio: u8,
    pub min_liquidation_threshold: u8,
    pub max_liquidation_threshold: u8,
    pub max_string_length: usize,
    pub max_array_length: usize,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            min_btc_amount: 10_000,           // 0.0001 BTC
            max_btc_amount: 100_000_000_000,  // 1000 BTC
            min_bollar_amount: 1,             // 0.01 USD
            max_bollar_amount: 100_000_000,   // 1M USD
            min_collateral_ratio: 50,         // 50%
            max_collateral_ratio: 95,         // 95%
            min_liquidation_threshold: 60,    // 60%
            max_liquidation_threshold: 99,    // 99%
            max_string_length: 1000,
            max_array_length: 1000,
        }
    }
}

// 输入验证器
pub struct InputValidator {
    config: ValidationConfig,
    blocked_principals: HashSet<Principal>,
    blocked_addresses: HashSet<String>,
}

impl Default for InputValidator {
    fn default() -> Self {
        Self {
            config: ValidationConfig::default(),
            blocked_principals: HashSet::new(),
            blocked_addresses: HashSet::new(),
        }
    }
}

impl InputValidator {
    // 验证 BTC 数量
    pub fn validate_btc_amount(&self, amount: u64, context: &str) -> Result<()> {
        if amount < self.config.min_btc_amount {
            return Err(Error::InvalidArgument(format!(
                "{}: BTC amount {} is below minimum {}",
                context, amount, self.config.min_btc_amount
            )));
        }
        
        if amount > self.config.max_btc_amount {
            return Err(Error::InvalidArgument(format!(
                "{}: BTC amount {} exceeds maximum {}",
                context, amount, self.config.max_btc_amount
            )));
        }
        
        // 检查是否为有效的 satoshi 值
        if amount == 0 {
            return Err(Error::InvalidArgument(format!(
                "{}: BTC amount cannot be zero", context
            )));
        }
        
        Ok(())
    }
    
    // 验证 Bollar 数量
    pub fn validate_bollar_amount(&self, amount: u64, context: &str) -> Result<()> {
        if amount < self.config.min_bollar_amount {
            return Err(Error::InvalidArgument(format!(
                "{}: Bollar amount {} is below minimum {}",
                context, amount, self.config.min_bollar_amount
            )));
        }
        
        if amount > self.config.max_bollar_amount {
            return Err(Error::InvalidArgument(format!(
                "{}: Bollar amount {} exceeds maximum {}",
                context, amount, self.config.max_bollar_amount
            )));
        }
        
        Ok(())
    }
    
    // 验证抵押率
    pub fn validate_collateral_ratio(&self, ratio: u8, context: &str) -> Result<()> {
        if ratio < self.config.min_collateral_ratio {
            return Err(Error::InvalidArgument(format!(
                "{}: Collateral ratio {}% is below minimum {}%",
                context, ratio, self.config.min_collateral_ratio
            )));
        }
        
        if ratio > self.config.max_collateral_ratio {
            return Err(Error::InvalidArgument(format!(
                "{}: Collateral ratio {}% exceeds maximum {}%",
                context, ratio, self.config.max_collateral_ratio
            )));
        }
        
        Ok(())
    }
    
    // 验证清算阈值
    pub fn validate_liquidation_threshold(&self, threshold: u8, collateral_ratio: u8, context: &str) -> Result<()> {
        if threshold < self.config.min_liquidation_threshold {
            return Err(Error::InvalidArgument(format!(
                "{}: Liquidation threshold {}% is below minimum {}%",
                context, threshold, self.config.min_liquidation_threshold
            )));
        }
        
        if threshold > self.config.max_liquidation_threshold {
            return Err(Error::InvalidArgument(format!(
                "{}: Liquidation threshold {}% exceeds maximum {}%",
                context, threshold, self.config.max_liquidation_threshold
            )));
        }
        
        // 清算阈值必须高于抵押率
        if threshold <= collateral_ratio {
            return Err(Error::InvalidArgument(format!(
                "{}: Liquidation threshold {}% must be higher than collateral ratio {}%",
                context, threshold, collateral_ratio
            )));
        }
        
        Ok(())
    }
    
    // 验证字符串输入
    pub fn validate_string(&self, input: &str, field_name: &str, required: bool) -> Result<()> {
        if required && input.trim().is_empty() {
            return Err(Error::InvalidArgument(format!(
                "{} is required and cannot be empty", field_name
            )));
        }
        
        if input.len() > self.config.max_string_length {
            return Err(Error::InvalidArgument(format!(
                "{} exceeds maximum length of {} characters",
                field_name, self.config.max_string_length
            )));
        }
        
        // 检查是否包含危险字符
        if input.contains('\0') || input.contains('\x01') || input.contains('\x02') {
            return Err(Error::InvalidArgument(format!(
                "{} contains invalid control characters", field_name
            )));
        }
        
        Ok(())
    }
    
    // 验证比特币地址格式
    pub fn validate_bitcoin_address(&self, address: &str, context: &str) -> Result<()> {
        self.validate_string(address, "Bitcoin address", true)?;
        
        // 检查是否在黑名单中
        if self.blocked_addresses.contains(address) {
            return Err(Error::PermissionDenied(format!(
                "{}: Address {} is blocked", context, address
            )));
        }
        
        // 基本格式检查
        if address.len() < 26 || address.len() > 62 {
            return Err(Error::InvalidArgument(format!(
                "{}: Invalid Bitcoin address length", context
            )));
        }
        
        // 检查地址前缀
        let valid_prefixes = ["1", "3", "bc1", "tb1", "2", "m", "n"];
        let has_valid_prefix = valid_prefixes.iter().any(|prefix| address.starts_with(prefix));
        
        if !has_valid_prefix {
            return Err(Error::InvalidArgument(format!(
                "{}: Invalid Bitcoin address prefix", context
            )));
        }
        
        Ok(())
    }
    
    // 验证 Principal
    pub fn validate_principal(&self, principal: Principal, context: &str) -> Result<()> {
        // 检查是否在黑名单中
        if self.blocked_principals.contains(&principal) {
            return Err(Error::PermissionDenied(format!(
                "{}: Principal {} is blocked", context, principal
            )));
        }
        
        // 检查是否为匿名 Principal
        if principal == Principal::anonymous() {
            return Err(Error::InvalidArgument(format!(
                "{}: Anonymous principal not allowed", context
            )));
        }
        
        Ok(())
    }
    
    // 验证 PSBT 十六进制字符串
    pub fn validate_psbt_hex(&self, psbt_hex: &str, context: &str) -> Result<()> {
        self.validate_string(psbt_hex, "PSBT hex", true)?;
        
        // 检查是否为有效的十六进制
        if psbt_hex.len() % 2 != 0 {
            return Err(Error::InvalidArgument(format!(
                "{}: PSBT hex string must have even length", context
            )));
        }
        
        // 检查十六进制字符
        if !psbt_hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::InvalidArgument(format!(
                "{}: PSBT contains invalid hex characters", context
            )));
        }
        
        // 检查最小长度（基本 PSBT 结构）
        if psbt_hex.len() < 20 {
            return Err(Error::InvalidArgument(format!(
                "{}: PSBT hex string too short", context
            )));
        }
        
        // 检查最大长度（防止过大的 PSBT）
        if psbt_hex.len() > 1_000_000 {
            return Err(Error::InvalidArgument(format!(
                "{}: PSBT hex string too long", context
            )));
        }
        
        Ok(())
    }
    
    // 验证数组长度
    pub fn validate_array_length<T>(&self, array: &[T], field_name: &str) -> Result<()> {
        if array.len() > self.config.max_array_length {
            return Err(Error::InvalidArgument(format!(
                "{} array exceeds maximum length of {}",
                field_name, self.config.max_array_length
            )));
        }
        Ok(())
    }
    
    // 验证时间戳
    pub fn validate_timestamp(&self, timestamp: u64, context: &str) -> Result<()> {
        let current_time = crate::ic_api::time();
        
        // 检查时间戳是否在合理范围内
        if timestamp == 0 {
            return Err(Error::InvalidArgument(format!(
                "{}: Timestamp cannot be zero", context
            )));
        }
        
        // 检查是否为未来时间（允许5分钟的时钟偏差）
        let max_future_time = current_time + (5 * 60 * 1_000_000_000);
        if timestamp > max_future_time {
            return Err(Error::InvalidArgument(format!(
                "{}: Timestamp is too far in the future", context
            )));
        }
        
        // 检查是否为过去时间（允许1年的历史数据）
        let min_past_time = current_time.saturating_sub(365 * 24 * 60 * 60 * 1_000_000_000);
        if timestamp < min_past_time {
            return Err(Error::InvalidArgument(format!(
                "{}: Timestamp is too old", context
            )));
        }
        
        Ok(())
    }
    
    // 验证价格
    pub fn validate_price(&self, price: u64, context: &str) -> Result<()> {
        if price == 0 {
            return Err(Error::InvalidArgument(format!(
                "{}: Price cannot be zero", context
            )));
        }
        
        // BTC 价格合理范围检查 (USD cents)
        let min_price = 100_000; // $1,000
        let max_price = 10_000_000; // $100,000
        
        if price < min_price {
            return Err(Error::InvalidArgument(format!(
                "{}: Price {} is below reasonable minimum {}",
                context, price, min_price
            )));
        }
        
        if price > max_price {
            return Err(Error::InvalidArgument(format!(
                "{}: Price {} exceeds reasonable maximum {}",
                context, price, max_price
            )));
        }
        
        Ok(())
    }
    
    // 验证健康因子
    pub fn validate_health_factor(&self, health_factor: u64, context: &str) -> Result<()> {
        // 健康因子不能为0（除非是特殊情况）
        if health_factor == 0 {
            return Err(Error::InvalidArgument(format!(
                "{}: Health factor cannot be zero", context
            )));
        }
        
        // 健康因子过高可能表示计算错误
        if health_factor > 10000 { // 100x
            return Err(Error::InvalidArgument(format!(
                "{}: Health factor {} is unreasonably high", context, health_factor
            )));
        }
        
        Ok(())
    }
    
    // 添加到黑名单
    pub fn add_blocked_principal(&mut self, principal: Principal) {
        self.blocked_principals.insert(principal);
    }
    
    pub fn add_blocked_address(&mut self, address: String) {
        self.blocked_addresses.insert(address);
    }
    
    // 从黑名单移除
    pub fn remove_blocked_principal(&mut self, principal: &Principal) {
        self.blocked_principals.remove(principal);
    }
    
    pub fn remove_blocked_address(&mut self, address: &str) {
        self.blocked_addresses.remove(address);
    }
    
    // 批量验证头寸数据
    pub fn validate_position(&self, position: &Position) -> Result<()> {
        self.validate_string(&position.id, "Position ID", true)?;
        self.validate_string(&position.owner, "Position owner", true)?;
        self.validate_btc_amount(position.btc_collateral, "Position BTC collateral")?;
        self.validate_bollar_amount(position.bollar_debt, "Position Bollar debt")?;
        self.validate_timestamp(position.created_at, "Position created_at")?;
        self.validate_timestamp(position.last_updated_at, "Position last_updated_at")?;
        self.validate_health_factor(position.health_factor, "Position health factor")?;
        
        // 验证时间戳逻辑
        if position.last_updated_at < position.created_at {
            return Err(Error::InvalidArgument(
                "Position last_updated_at cannot be before created_at".to_string()
            ));
        }
        
        Ok(())
    }
    
    // 批量验证池数据
    pub fn validate_pool(&self, pool: &Pool) -> Result<()> {
        self.validate_string(&pool.addr, "Pool address", true)?;
        self.validate_string(&pool.meta.symbol, "Pool symbol", true)?;
        self.validate_collateral_ratio(pool.collateral_ratio, "Pool collateral ratio")?;
        self.validate_liquidation_threshold(
            pool.liquidation_threshold, 
            pool.collateral_ratio, 
            "Pool liquidation threshold"
        )?;
        
        // 验证池状态
        if pool.states.is_empty() {
            return Err(Error::InvalidArgument("Pool must have at least one state".to_string()));
        }
        
        // 验证每个池状态
        for (i, state) in pool.states.iter().enumerate() {
            if let Some(utxo) = &state.utxo {
                self.validate_string(&utxo.outpoint, &format!("Pool state {} outpoint", i), true)?;
                if utxo.sats == 0 {
                    return Err(Error::InvalidArgument(format!(
                        "Pool state {} UTXO cannot have zero sats", i
                    )));
                }
            }
            
            if state.btc_price > 0 {
                self.validate_price(state.btc_price, &format!("Pool state {} BTC price", i))?;
            }
        }
        
        Ok(())
    }
}

// 全局验证器实例
use std::cell::RefCell;

thread_local! {
    static GLOBAL_VALIDATOR: RefCell<InputValidator> = RefCell::new(InputValidator::default());
}

// 便捷的验证函数
pub fn validate_btc_amount(amount: u64, context: &str) -> Result<()> {
    GLOBAL_VALIDATOR.with_borrow(|validator| validator.validate_btc_amount(amount, context))
}

pub fn validate_bollar_amount(amount: u64, context: &str) -> Result<()> {
    GLOBAL_VALIDATOR.with_borrow(|validator| validator.validate_bollar_amount(amount, context))
}

pub fn validate_bitcoin_address(address: &str, context: &str) -> Result<()> {
    GLOBAL_VALIDATOR.with_borrow(|validator| validator.validate_bitcoin_address(address, context))
}

pub fn validate_psbt_hex(psbt_hex: &str, context: &str) -> Result<()> {
    GLOBAL_VALIDATOR.with_borrow(|validator| validator.validate_psbt_hex(psbt_hex, context))
}

pub fn validate_principal(principal: Principal, context: &str) -> Result<()> {
    GLOBAL_VALIDATOR.with_borrow(|validator| validator.validate_principal(principal, context))
}

pub fn validate_position(position: &Position) -> Result<()> {
    GLOBAL_VALIDATOR.with_borrow(|validator| validator.validate_position(position))
}

pub fn validate_pool(pool: &Pool) -> Result<()> {
    GLOBAL_VALIDATOR.with_borrow(|validator| validator.validate_pool(pool))
}

// 验证宏
#[macro_export]
macro_rules! validate_input {
    ($validator:expr, $value:expr, $context:expr) => {
        $validator($value, $context)?;
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btc_amount_validation() {
        let validator = InputValidator::default();
        
        // 有效数量
        assert!(validator.validate_btc_amount(100_000, "test").is_ok());
        
        // 过小数量
        assert!(validator.validate_btc_amount(1000, "test").is_err());
        
        // 过大数量
        assert!(validator.validate_btc_amount(u64::MAX, "test").is_err());
        
        // 零数量
        assert!(validator.validate_btc_amount(0, "test").is_err());
    }

    #[test]
    fn test_string_validation() {
        let validator = InputValidator::default();
        
        // 有效字符串
        assert!(validator.validate_string("valid string", "test", true).is_ok());
        
        // 空字符串（必需）
        assert!(validator.validate_string("", "test", true).is_err());
        
        // 空字符串（非必需）
        assert!(validator.validate_string("", "test", false).is_ok());
        
        // 过长字符串
        let long_string = "a".repeat(2000);
        assert!(validator.validate_string(&long_string, "test", false).is_err());
        
        // 包含控制字符
        assert!(validator.validate_string("test\0string", "test", false).is_err());
    }

    #[test]
    fn test_bitcoin_address_validation() {
        let validator = InputValidator::default();
        
        // 有效地址格式
        assert!(validator.validate_bitcoin_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa", "test").is_ok());
        assert!(validator.validate_bitcoin_address("3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy", "test").is_ok());
        assert!(validator.validate_bitcoin_address("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4", "test").is_ok());
        
        // 无效地址格式
        assert!(validator.validate_bitcoin_address("invalid", "test").is_err());
        assert!(validator.validate_bitcoin_address("", "test").is_err());
        assert!(validator.validate_bitcoin_address("x".repeat(100), "test").is_err());
    }

    #[test]
    fn test_collateral_ratio_validation() {
        let validator = InputValidator::default();
        
        // 有效抵押率
        assert!(validator.validate_collateral_ratio(75, "test").is_ok());
        
        // 过低抵押率
        assert!(validator.validate_collateral_ratio(30, "test").is_err());
        
        // 过高抵押率
        assert!(validator.validate_collateral_ratio(99, "test").is_err());
    }

    #[test]
    fn test_liquidation_threshold_validation() {
        let validator = InputValidator::default();
        
        // 有效清算阈值
        assert!(validator.validate_liquidation_threshold(80, 75, "test").is_ok());
        
        // 清算阈值低于抵押率
        assert!(validator.validate_liquidation_threshold(70, 75, "test").is_err());
        
        // 清算阈值等于抵押率
        assert!(validator.validate_liquidation_threshold(75, 75, "test").is_err());
    }
}