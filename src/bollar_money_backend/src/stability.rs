// stability.rs - 稳定机制
// 这个模块实现抵押率和清算阈值管理功能

use crate::{Error, LogLevel, Result, error::{catch_and_log}, types::*};
use ic_cdk_macros::{query, update};
use candid::{CandidType, Deserialize};
use serde::Serialize;

#[update]
// 更新抵押率
pub fn update_collateral_ratio(new_ratio: u8) -> Result<bool> {
    // 使用 catch_and_log 包装操作
    catch_and_log(
        || {
            // 验证调用者是否为控制者
            let caller = crate::ic_api::caller();
            if !crate::ic_api::is_controller(&caller) {
                return Err(Error::PermissionDenied("Not authorized".to_string()));
            }
            
            // 验证抵押率参数
            if new_ratio == 0 || new_ratio > 100 {
                return Err(Error::InvalidArgument(format!(
                    "无效的抵押率: {}，应在 1-100 之间",
                    new_ratio
                )));
            }
            
            // 获取所有池并更新抵押率
            let pools = crate::get_pools();
            let mut updated = false;
            
            for mut pool in pools {
                // 检查清算阈值是否仍然有效
                if new_ratio >= pool.liquidation_threshold {
                    return Err(Error::InvalidArgument(format!(
                        "抵押率 {} 不能大于或等于清算阈值 {}",
                        new_ratio,
                        pool.liquidation_threshold
                    )));
                }
                
                // 更新抵押率
                pool.update_collateral_ratio(new_ratio);
                crate::save_pool(pool);
                updated = true;
            }
            
            if updated {
                ic_cdk::println!(
                    "Collateral ratio updated to {}% by {}",
                    new_ratio,
                    caller
                );
            }
            
            Ok(updated)
        },
        LogLevel::Error,
        &format!("update_collateral_ratio: 更新抵押率失败, new_ratio={}", new_ratio)
    )
}

#[update]
// 更新清算阈值
pub fn update_liquidation_threshold(new_threshold: u8) -> Result<bool> {
    // 使用 catch_and_log 包装操作
    catch_and_log(
        || {
            // 验证调用者是否为控制者
            let caller = crate::ic_api::caller();
            if !crate::ic_api::is_controller(&caller) {
                return Err(Error::PermissionDenied("Not authorized".to_string()));
            }
            
            // 验证清算阈值参数
            if new_threshold == 0 || new_threshold > 100 {
                return Err(Error::InvalidArgument(format!(
                    "无效的清算阈值: {}，应在 1-100 之间",
                    new_threshold
                )));
            }
            
            // 获取所有池并更新清算阈值
            let pools = crate::get_pools();
            let mut updated = false;
            
            for mut pool in pools {
                // 检查抵押率是否仍然有效
                if pool.collateral_ratio >= new_threshold {
                    return Err(Error::InvalidArgument(format!(
                        "清算阈值 {} 不能小于或等于抵押率 {}",
                        new_threshold,
                        pool.collateral_ratio
                    )));
                }
                
                // 更新清算阈值
                pool.update_liquidation_threshold(new_threshold);
                crate::save_pool(pool);
                updated = true;
            }
            
            if updated {
                // 重新计算所有头寸的健康因子，因为清算阈值变化可能影响清算状态
                update_all_positions_liquidation_status(new_threshold);
                
                ic_cdk::println!(
                    "Liquidation threshold updated to {}% by {}",
                    new_threshold,
                    caller
                );
            }
            
            Ok(updated)
        },
        LogLevel::Error,
        &format!("update_liquidation_threshold: 更新清算阈值失败, new_threshold={}", new_threshold)
    )
}

#[query]
// 获取当前系统参数
pub fn get_system_parameters() -> Result<SystemParameters> {
    // 使用 catch_and_log 包装操作
    catch_and_log(
        || {
            // 获取第一个池的参数作为系统参数
            let pools = crate::get_pools();
            let pool = pools.first()
                .ok_or(Error::InvalidState("没有可用的资金池".to_string()))?;
            
            let params = SystemParameters {
                collateral_ratio: pool.collateral_ratio,
                liquidation_threshold: pool.liquidation_threshold,
                btc_price: crate::oracle::get_btc_price(),
                total_pools: pools.len() as u64,
                total_positions: crate::get_positions().len() as u64,
            };
            
            Ok(params)
        },
        LogLevel::Debug,
        "get_system_parameters: 获取系统参数失败"
    )
}

#[update]
// 批量更新系统参数
pub fn update_system_parameters(
    collateral_ratio: Option<u8>,
    liquidation_threshold: Option<u8>,
) -> Result<bool> {
    // 使用 catch_and_log 包装操作
    catch_and_log(
        || {
            // 验证调用者是否为控制者
            let caller = crate::ic_api::caller();
            if !crate::ic_api::is_controller(&caller) {
                return Err(Error::PermissionDenied("Not authorized".to_string()));
            }
            
            let mut updated = false;
            
            // 如果提供了抵押率，则更新
            if let Some(ratio) = collateral_ratio {
                match update_collateral_ratio(ratio) {
                    Ok(result) => updated = updated || result,
                    Err(e) => return Err(e),
                }
            }
            
            // 如果提供了清算阈值，则更新
            if let Some(threshold) = liquidation_threshold {
                match update_liquidation_threshold(threshold) {
                    Ok(result) => updated = updated || result,
                    Err(e) => return Err(e),
                }
            }
            
            Ok(updated)
        },
        LogLevel::Error,
        "update_system_parameters: 批量更新系统参数失败"
    )
}

// 更新所有头寸的清算状态
fn update_all_positions_liquidation_status(new_threshold: u8) {
    // 获取所有头寸
    let positions = crate::get_positions();
    
    // 获取当前 BTC 价格
    let btc_price = crate::oracle::get_btc_price();
    
    // 检查每个头寸的清算状态
    for position in positions {
        let was_liquidatable = position.is_liquidatable(new_threshold);
        
        if was_liquidatable {
            ic_cdk::println!(
                "Position {} is now liquidatable with new threshold {}%",
                position.id,
                new_threshold
            );
        }
    }
}

// 系统参数结构
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SystemParameters {
    pub collateral_ratio: u8,        // 抵押率
    pub liquidation_threshold: u8,   // 清算阈值
    pub btc_price: u64,              // 当前 BTC 价格
    pub total_pools: u64,            // 总池数量
    pub total_positions: u64,        // 总头寸数量
}

// 系统健康状态
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SystemHealth {
    pub total_collateral_value: u64,     // 总抵押品价值 (USD cents)
    pub total_debt_value: u64,           // 总债务价值 (USD cents)
    pub system_collateral_ratio: u64,    // 系统整体抵押率
    pub liquidatable_positions: u64,     // 可清算头寸数量
    pub at_risk_positions: u64,          // 风险头寸数量 (健康因子 < 120%)
}

#[query]
// 获取系统健康状态
pub fn get_system_health() -> SystemHealth {
    // 获取所有头寸
    let positions = crate::get_positions();
    
    // 获取当前 BTC 价格
    let btc_price = crate::oracle::get_btc_price();
    
    // 获取系统参数
    let pools = crate::get_pools();
    let liquidation_threshold = pools.first()
        .map(|p| p.liquidation_threshold)
        .unwrap_or(80);
    
    // 计算系统指标
    let mut total_collateral_value = 0u64;
    let mut total_debt_value = 0u64;
    let mut liquidatable_positions = 0u64;
    let mut at_risk_positions = 0u64;
    
    for position in positions {
        // 计算抵押品价值
        let collateral_value = (position.btc_collateral as u128) * (btc_price as u128) / 100_000_000;
        total_collateral_value += collateral_value as u64;
        
        // 计算债务价值
        total_debt_value += position.bollar_debt;
        
        // 检查是否可清算
        if position.is_liquidatable(liquidation_threshold) {
            liquidatable_positions += 1;
        }
        
        // 检查是否为风险头寸 (健康因子 < 120%)
        if position.health_factor < 120 {
            at_risk_positions += 1;
        }
    }
    
    // 计算系统整体抵押率
    let system_collateral_ratio = if total_debt_value > 0 {
        (total_collateral_value as u128) * 100 / (total_debt_value as u128)
    } else {
        u128::MAX
    };
    
    SystemHealth {
        total_collateral_value,
        total_debt_value,
        system_collateral_ratio: system_collateral_ratio.try_into().unwrap_or(u64::MAX),
        liquidatable_positions,
        at_risk_positions,
    }
}