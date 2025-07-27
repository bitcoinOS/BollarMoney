// safe_math.rs - 安全数学运算
// 这个模块提供防止整数溢出的安全数学运算

use crate::{Error, Result};

/// 安全的加法运算
pub fn safe_add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or(Error::Overflow)
}

/// 安全的减法运算
pub fn safe_sub(a: u64, b: u64) -> Result<u64> {
    a.checked_sub(b).ok_or(Error::Overflow)
}

/// 安全的乘法运算
pub fn safe_mul(a: u64, b: u64) -> Result<u64> {
    a.checked_mul(b).ok_or(Error::Overflow)
}

/// 安全的除法运算
pub fn safe_div(a: u64, b: u64) -> Result<u64> {
    if b == 0 {
        return Err(Error::InvalidArgument("除数不能为零".to_string()));
    }
    Ok(a / b)
}

/// 安全的 u128 加法运算
pub fn safe_add_u128(a: u128, b: u128) -> Result<u128> {
    a.checked_add(b).ok_or(Error::Overflow)
}

/// 安全的 u128 乘法运算
pub fn safe_mul_u128(a: u128, b: u128) -> Result<u128> {
    a.checked_mul(b).ok_or(Error::Overflow)
}

/// 安全的 u128 除法运算
pub fn safe_div_u128(a: u128, b: u128) -> Result<u128> {
    if b == 0 {
        return Err(Error::InvalidArgument("除数不能为零".to_string()));
    }
    Ok(a / b)
}

/// 安全的类型转换 u128 -> u64
pub fn safe_cast_u128_to_u64(value: u128) -> Result<u64> {
    value.try_into().map_err(|_| Error::Overflow)
}

/// 安全的类型转换 u64 -> u128
pub fn safe_cast_u64_to_u128(value: u64) -> u128 {
    value as u128
}

/// 安全的百分比计算
pub fn safe_percentage(value: u64, percentage: u8) -> Result<u64> {
    if percentage > 100 {
        return Err(Error::InvalidArgument("百分比不能超过100".to_string()));
    }
    
    let value_u128 = safe_cast_u64_to_u128(value);
    let percentage_u128 = safe_cast_u64_to_u128(percentage as u64);
    
    let result_u128 = safe_mul_u128(value_u128, percentage_u128)?;
    let result_u128 = safe_div_u128(result_u128, 100)?;
    
    safe_cast_u128_to_u64(result_u128)
}

/// 安全的健康因子计算
pub fn safe_calculate_health_factor(
    btc_collateral: u64,
    bollar_debt: u64,
    btc_price: u64,
) -> Result<u64> {
    if bollar_debt == 0 {
        return Ok(u64::MAX); // 无债务，健康因子无限大
    }
    
    // 计算抵押品价值 (USD cents)
    let btc_collateral_u128 = safe_cast_u64_to_u128(btc_collateral);
    let btc_price_u128 = safe_cast_u64_to_u128(btc_price);
    let satoshis_per_btc = safe_cast_u64_to_u128(100_000_000);
    
    let collateral_value = safe_mul_u128(btc_collateral_u128, btc_price_u128)?;
    let collateral_value = safe_div_u128(collateral_value, satoshis_per_btc)?;
    
    // 计算健康因子 (抵押价值/债务价值 * 100)
    let bollar_debt_u128 = safe_cast_u64_to_u128(bollar_debt);
    let health_factor_u128 = safe_mul_u128(collateral_value, 100)?;
    let health_factor_u128 = safe_div_u128(health_factor_u128, bollar_debt_u128)?;
    
    safe_cast_u128_to_u64(health_factor_u128)
}

/// 安全的最大 Bollar 计算
pub fn safe_calculate_max_bollar(
    btc_amount: u64,
    btc_price: u64,
    collateral_ratio: u8,
) -> Result<u64> {
    if collateral_ratio > 100 {
        return Err(Error::InvalidArgument("抵押率不能超过100%".to_string()));
    }
    
    let btc_amount_u128 = safe_cast_u64_to_u128(btc_amount);
    let btc_price_u128 = safe_cast_u64_to_u128(btc_price);
    let satoshis_per_btc = safe_cast_u64_to_u128(100_000_000);
    let collateral_ratio_u128 = safe_cast_u64_to_u128(collateral_ratio as u64);
    
    // 计算 BTC 价值 (USD cents)
    let btc_value = safe_mul_u128(btc_amount_u128, btc_price_u128)?;
    let btc_value = safe_div_u128(btc_value, satoshis_per_btc)?;
    
    // 应用抵押率
    let max_bollar = safe_mul_u128(btc_value, collateral_ratio_u128)?;
    let max_bollar = safe_div_u128(max_bollar, 100)?;
    
    safe_cast_u128_to_u64(max_bollar)
}

/// 安全的清算奖励计算
pub fn safe_calculate_liquidation_reward(
    bollar_repay_amount: u64,
    btc_collateral: u64,
    bollar_debt: u64,
    btc_price: u64,
    liquidation_bonus_percent: u8,
) -> Result<u64> {
    if liquidation_bonus_percent > 50 {
        return Err(Error::InvalidArgument("清算奖励不能超过50%".to_string()));
    }
    
    // 计算等值的 BTC 数量
    let bollar_repay_u128 = safe_cast_u64_to_u128(bollar_repay_amount);
    let satoshis_per_btc = safe_cast_u64_to_u128(100_000_000);
    let btc_price_u128 = safe_cast_u64_to_u128(btc_price);
    
    let btc_equivalent = safe_mul_u128(bollar_repay_u128, satoshis_per_btc)?;
    let btc_equivalent = safe_div_u128(btc_equivalent, btc_price_u128)?;
    
    // 添加奖励
    let bonus_percent_u128 = safe_cast_u64_to_u128(liquidation_bonus_percent as u64);
    let bonus_multiplier = safe_add_u128(100, bonus_percent_u128)?;
    let btc_with_bonus = safe_mul_u128(btc_equivalent, bonus_multiplier)?;
    let btc_with_bonus = safe_div_u128(btc_with_bonus, 100)?;
    
    // 确保不超过抵押品总量的比例
    let btc_collateral_u128 = safe_cast_u64_to_u128(btc_collateral);
    let bollar_debt_u128 = safe_cast_u64_to_u128(bollar_debt);
    
    let max_btc = safe_mul_u128(btc_collateral_u128, bollar_repay_u128)?;
    let max_btc = safe_div_u128(max_btc, bollar_debt_u128)?;
    
    let result = std::cmp::min(btc_with_bonus, max_btc);
    safe_cast_u128_to_u64(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_add() {
        assert_eq!(safe_add(1, 2).unwrap(), 3);
        assert!(safe_add(u64::MAX, 1).is_err());
    }

    #[test]
    fn test_safe_sub() {
        assert_eq!(safe_sub(5, 3).unwrap(), 2);
        assert!(safe_sub(3, 5).is_err());
    }

    #[test]
    fn test_safe_mul() {
        assert_eq!(safe_mul(3, 4).unwrap(), 12);
        assert!(safe_mul(u64::MAX, 2).is_err());
    }

    #[test]
    fn test_safe_div() {
        assert_eq!(safe_div(10, 2).unwrap(), 5);
        assert!(safe_div(10, 0).is_err());
    }

    #[test]
    fn test_safe_percentage() {
        assert_eq!(safe_percentage(1000, 10).unwrap(), 100);
        assert!(safe_percentage(1000, 101).is_err());
    }

    #[test]
    fn test_safe_calculate_health_factor() {
        let health_factor = safe_calculate_health_factor(
            100_000_000, // 1 BTC
            3000000,     // $30,000 debt
            3000000      // $30,000 BTC price
        ).unwrap();
        assert_eq!(health_factor, 100); // 100% health factor
    }
}