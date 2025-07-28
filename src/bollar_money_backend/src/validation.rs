// validation.rs - 输入验证和边界检查
// 这个模块提供全面的输入验证和数据完整性检查

use crate::{Error, Result};
use bitcoin::Address;
use std::str::FromStr;

/// 比特币地址验证
pub fn validate_bitcoin_address(address: &str) -> Result<()> {
    validate_input!(!address.is_empty(), "比特币地址不能为空");
    validate_input!(address.len() >= 26 && address.len() <= 62, "比特币地址长度无效");
    
    // 验证地址格式
    Address::from_str(address)
        .map_err(|_| Error::InvalidArgument("无效的比特币地址格式".to_string()))?;
    
    Ok(())
}

/// 金额验证
pub fn validate_amount(amount: u64, min_amount: u64, max_amount: u64, name: &str) -> Result<()> {
    validate_input!(amount > 0, &format!("{} 必须大于零", name));
    validate_input!(amount >= min_amount, &format!("{} 不能小于最小值 {}", name, min_amount));
    validate_input!(amount <= max_amount, &format!("{} 不能超过最大值 {}", name, max_amount));
    
    Ok(())
}

/// BTC 金额验证
pub fn validate_btc_amount(amount: u64) -> Result<()> {
    const MIN_BTC_AMOUNT: u64 = 10_000; // 0.0001 BTC
    const MAX_BTC_AMOUNT: u64 = 100_000_000_000; // 1000 BTC
    
    validate_amount(amount, MIN_BTC_AMOUNT, MAX_BTC_AMOUNT, "BTC金额")
}

/// Bollar 金额验证
pub fn validate_bollar_amount(amount: u64) -> Result<()> {
    const MIN_BOLLAR_AMOUNT: u64 = 1; // 0.01 USD
    const MAX_BOLLAR_AMOUNT: u64 = 10_000_000_000; // $100,000,000
    
    validate_amount(amount, MIN_BOLLAR_AMOUNT, MAX_BOLLAR_AMOUNT, "Bollar金额")
}

/// 百分比验证
pub fn validate_percentage(percentage: u8, max_percentage: u8, name: &str) -> Result<()> {
    validate_input!(percentage <= max_percentage, 
                   &format!("{} 不能超过 {}%", name, max_percentage));
    Ok(())
}

/// 抵押率验证
pub fn validate_collateral_ratio(ratio: u8) -> Result<()> {
    validate_input!(ratio > 0, "抵押率必须大于0");
    validate_input!(ratio <= 95, "抵押率不能超过95%");
    Ok(())
}

/// 清算阈值验证
pub fn validate_liquidation_threshold(threshold: u8, collateral_ratio: u8) -> Result<()> {
    validate_input!(threshold > collateral_ratio, 
                   "清算阈值必须大于抵押率");
    validate_input!(threshold <= 100, "清算阈值不能超过100%");
    Ok(())
}

/// 字符串长度验证
pub fn validate_string_length(s: &str, min_len: usize, max_len: usize, name: &str) -> Result<()> {
    let len = s.len();
    validate_input!(len >= min_len, &format!("{} 长度不能少于 {} 字符", name, min_len));
    validate_input!(len <= max_len, &format!("{} 长度不能超过 {} 字符", name, max_len));
    Ok(())
}

/// 原因字符串验证
pub fn validate_reason(reason: &str) -> Result<()> {
    validate_string_length(reason.trim(), 5, 200, "原因说明")?;
    
    // 检查是否包含有害字符
    let forbidden_chars = ['<', '>', '"', '\'', '&', '\0'];
    validate_input!(!reason.chars().any(|c| forbidden_chars.contains(&c)), 
                   "原因说明包含非法字符");
    
    Ok(())
}

/// PSBT 十六进制字符串验证
pub fn validate_psbt_hex(psbt_hex: &str) -> Result<()> {
    validate_input!(!psbt_hex.is_empty(), "PSBT 不能为空");
    validate_input!(psbt_hex.len() >= 100, "PSBT 长度过短");
    validate_input!(psbt_hex.len() <= 100_000, "PSBT 长度过长");
    
    // 验证是否为有效的十六进制字符串
    validate_input!(psbt_hex.chars().all(|c| c.is_ascii_hexdigit()), 
                   "PSBT 必须是有效的十六进制字符串");
    
    Ok(())
}

/// 头寸ID验证
pub fn validate_position_id(position_id: &str) -> Result<()> {
    validate_input!(!position_id.is_empty(), "头寸ID不能为空");
    
    // 头寸ID格式: pool_address:timestamp:user
    let parts: Vec<&str> = position_id.split(':').collect();
    validate_input!(parts.len() == 3, "头寸ID格式无效");
    
    // 验证各部分
    validate_bitcoin_address(parts[0])?;
    validate_input!(parts[1].parse::<u64>().is_ok(), "头寸ID中的时间戳无效");
    validate_input!(!parts[2].is_empty(), "头寸ID中的用户标识无效");
    
    Ok(())
}

/// 时间戳验证
pub fn validate_timestamp(timestamp: u64) -> Result<()> {
    let current_time = crate::ic_api::time();
    let one_year_ns = 365 * 24 * 60 * 60 * 1_000_000_000u64;
    
    validate_input!(timestamp > 0, "时间戳必须大于0");
    validate_input!(timestamp <= current_time + one_year_ns, 
                   "时间戳不能超过未来一年");
    
    Ok(())
}

/// 签名验证
pub fn validate_signature(signature: &str) -> Result<()> {
    validate_input!(!signature.is_empty(), "签名不能为空");
    validate_input!(signature.len() >= 64, "签名长度过短");
    validate_input!(signature.len() <= 200, "签名长度过长");
    
    // 验证是否为有效的 base64 字符串
    base64::decode(signature)
        .map_err(|_| Error::InvalidArgument("签名必须是有效的base64字符串".to_string()))?;
    
    Ok(())
}

/// 消息验证
pub fn validate_message(message: &str) -> Result<()> {
    validate_string_length(message, 1, 1000, "消息")?;
    
    // 检查消息内容的合理性
    validate_input!(!message.trim().is_empty(), "消息内容不能为空白");
    
    Ok(())
}

/// 操作类型验证
pub fn validate_operation_type(operation: &str) -> Result<()> {
    const VALID_OPERATIONS: &[&str] = &[
        "deposit", "repay", "liquidate", "withdraw", 
        "update_collateral", "emergency_pause", "emergency_resume"
    ];
    
    validate_input!(VALID_OPERATIONS.contains(&operation), 
                   &format!("无效的操作类型: {}", operation));
    
    Ok(())
}

/// 网络类型验证
pub fn validate_network(network: &str) -> Result<()> {
    const VALID_NETWORKS: &[&str] = &["mainnet", "testnet", "regtest"];
    
    validate_input!(VALID_NETWORKS.contains(&network), 
                   &format!("无效的网络类型: {}", network));
    
    Ok(())
}

/// 批量验证结果
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
        }
    }
    
    pub fn add_error(&mut self, error: String) {
        self.is_valid = false;
        self.errors.push(error);
    }
    
    pub fn merge(&mut self, other: ValidationResult) {
        if !other.is_valid {
            self.is_valid = false;
            self.errors.extend(other.errors);
        }
    }
    
    pub fn to_result(self) -> Result<()> {
        if self.is_valid {
            Ok(())
        } else {
            Err(Error::InvalidArgument(self.errors.join("; ")))
        }
    }
}

/// 批量验证宏
#[macro_export]
macro_rules! batch_validate {
    ($result:expr, $($validation:expr),+) => {
        $(
            if let Err(e) = $validation {
                $result.add_error(e.to_string());
            }
        )+
    };
}

/// 复合验证：抵押操作
pub fn validate_deposit_operation(
    pool_address: &str,
    btc_amount: u64,
    bollar_amount: u64,
    psbt_hex: &str,
) -> Result<()> {
    let mut result = ValidationResult::new();
    
    batch_validate!(result,
        validate_bitcoin_address(pool_address),
        validate_btc_amount(btc_amount),
        validate_bollar_amount(bollar_amount),
        validate_psbt_hex(psbt_hex)
    );
    
    result.to_result()
}

/// 复合验证：还款操作
pub fn validate_repay_operation(
    position_id: &str,
    psbt_hex: &str,
) -> Result<()> {
    let mut result = ValidationResult::new();
    
    batch_validate!(result,
        validate_position_id(position_id),
        validate_psbt_hex(psbt_hex)
    );
    
    result.to_result()
}

/// 复合验证：清算操作
pub fn validate_liquidation_operation(
    position_id: &str,
    bollar_repay_amount: u64,
    psbt_hex: &str,
) -> Result<()> {
    let mut result = ValidationResult::new();
    
    batch_validate!(result,
        validate_position_id(position_id),
        validate_bollar_amount(bollar_repay_amount),
        validate_psbt_hex(psbt_hex)
    );
    
    result.to_result()
}

/// 复合验证：认证操作
pub fn validate_authentication(
    address: &str,
    signature: &str,
    message: &str,
) -> Result<()> {
    let mut result = ValidationResult::new();
    
    batch_validate!(result,
        validate_bitcoin_address(address),
        validate_signature(signature),
        validate_message(message)
    );
    
    result.to_result()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_bitcoin_address() {
        // 有效地址
        assert!(validate_bitcoin_address("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4").is_ok());
        assert!(validate_bitcoin_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa").is_ok());
        
        // 无效地址
        assert!(validate_bitcoin_address("").is_err());
        assert!(validate_bitcoin_address("invalid_address").is_err());
    }

    #[test]
    fn test_validate_amounts() {
        assert!(validate_btc_amount(100_000).is_ok());
        assert!(validate_btc_amount(0).is_err());
        assert!(validate_btc_amount(1).is_err()); // 太小
        
        assert!(validate_bollar_amount(1000).is_ok());
        assert!(validate_bollar_amount(0).is_err());
    }

    #[test]
    fn test_validate_percentages() {
        assert!(validate_collateral_ratio(75).is_ok());
        assert!(validate_collateral_ratio(0).is_err());
        assert!(validate_collateral_ratio(96).is_err());
        
        assert!(validate_liquidation_threshold(80, 75).is_ok());
        assert!(validate_liquidation_threshold(70, 75).is_err()); // 小于抵押率
    }

    #[test]
    fn test_batch_validation() {
        let mut result = ValidationResult::new();
        batch_validate!(result,
            validate_btc_amount(0), // 会失败
            validate_bollar_amount(1000) // 会成功
        );
        
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
    }
}