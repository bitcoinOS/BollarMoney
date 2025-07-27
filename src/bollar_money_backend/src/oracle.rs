// oracle.rs - BTC 价格 Oracle 集成
// 这个模块负责获取和管理 BTC 价格数据

use candid::{CandidType, Deserialize, Principal};
use ic_cdk::api::call::call;
use ic_cdk_macros::{query, update};
use serde::Serialize;
use std::cell::RefCell;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::{Error, LogLevel, Result, error::log_error};

// Oracle canister ID
const ORACLE_CANISTER_ID: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai"; // 示例 ID，需替换为实际 Oracle canister ID

// 价格更新间隔 (毫秒)
const PRICE_UPDATE_INTERVAL_MS: u64 = 60_000; // 1分钟

// 价格有效期 (毫秒)
const PRICE_VALIDITY_PERIOD_MS: u64 = 300_000; // 5分钟

// 价格数据结构
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct PriceData {
    price: u64,           // 价格 (USD cents)
    timestamp: u64,       // 时间戳 (毫秒)
    source: String,       // 价格来源
}

// Oracle 响应结构
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct OracleResponse {
    btc_price: u64,       // BTC 价格 (USD cents)
    timestamp: u64,       // 时间戳 (毫秒)
    source: String,       // 价格来源
}

thread_local! {
    // 价格数据
    static PRICE_DATA: RefCell<PriceData> = RefCell::new(PriceData {
        price: 3000000,   // 默认 $30,000.00
        timestamp: current_time_millis(),
        source: "default".to_string(),
    });
    
    // 最后更新时间
    static LAST_UPDATE_TIME: RefCell<u64> = RefCell::new(0);
}

// 获取当前时间 (毫秒)
fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as u64
}

// 获取 BTC 价格 (USD cents)
#[query]
pub fn get_btc_price() -> u64 {
    PRICE_DATA.with_borrow(|data| data.price)
}

// 获取完整价格数据
#[query]
pub fn get_price_data() -> PriceData {
    PRICE_DATA.with_borrow(|data| data.clone())
}

// 检查价格是否需要更新
fn needs_update() -> bool {
    let now = current_time_millis();
    LAST_UPDATE_TIME.with_borrow(|last| {
        now - *last > PRICE_UPDATE_INTERVAL_MS
    })
}

// 检查价格是否有效
pub fn is_price_valid() -> bool {
    let now = current_time_millis();
    PRICE_DATA.with_borrow(|data| {
        now - data.timestamp < PRICE_VALIDITY_PERIOD_MS
    })
}

// 存储价格数据
fn store_price_data(data: PriceData) {
    PRICE_DATA.with_borrow_mut(|p| {
        *p = data;
    });
    
    LAST_UPDATE_TIME.with_borrow_mut(|t| {
        *t = current_time_millis();
    });
}

// 从 Oracle canister 获取价格
async fn fetch_price_from_oracle() -> Result<PriceData> {
    // 调用 Oracle canister
    let oracle_id = Principal::from_text(ORACLE_CANISTER_ID)
        .map_err(|_| Error::OracleError("Invalid Oracle canister ID".to_string()))?;
    
    // 调用 Oracle 的 get_btc_price 方法
    let response: OracleResponse = match call(oracle_id, "get_btc_price", ()).await {
        Ok((response,)) => response,
        Err((code, msg)) => {
            return Err(Error::OracleError(format!(
                "Failed to call Oracle: code={:?}, message={}",
                code, msg
            )));
        }
    };
    
    // 构建价格数据
    let price_data = PriceData {
        price: response.btc_price,
        timestamp: response.timestamp,
        source: response.source,
    };
    
    Ok(price_data)
}

// 定期更新价格的任务
#[update]
pub async fn update_price() -> Result<()> {
    // 检查是否需要更新
    if !needs_update() {
        return Ok(());
    }
    
    // 尝试从 Oracle 获取价格
    match fetch_price_from_oracle().await {
        Ok(price_data) => {
            // 获取旧价格
            let old_price = PRICE_DATA.with_borrow(|data| data.price);
            
            // 存储价格数据
            store_price_data(price_data.clone());
            
            // 检查价格变化
            if old_price > 0 {
                let price_change_percent = ((price_data.price as f64 - old_price as f64) / old_price as f64 * 100.0).abs();
                
                // 如果价格变化超过阈值，重新计算所有头寸的健康因子
                if price_change_percent > 5.0 {
                    // 记录重大价格变化
                    ic_cdk::println!(
                        "Significant price change detected: {}% (${}.{} -> ${}.{})",
                        price_change_percent,
                        old_price / 100,
                        old_price % 100,
                        price_data.price / 100,
                        price_data.price % 100
                    );
                    
                    // 更新所有头寸的健康因子
                    update_all_positions_health_factor(price_data.price);
                }
            }
            
            Ok(())
        }
        Err(e) => {
            // 记录错误
            log_error(
                LogLevel::Warning,
                &e,
                Some("Failed to fetch price from Oracle")
            );
            
            // 如果价格仍然有效，则不返回错误
            if is_price_valid() {
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

// 更新所有头寸的健康因子
fn update_all_positions_health_factor(btc_price: u64) {
    // 获取所有头寸
    let positions = crate::get_positions();
    
    // 更新每个头寸的健康因子
    for mut position in positions {
        // 计算新的健康因子
        let health_factor = crate::types::calculate_health_factor(
            position.btc_collateral,
            position.bollar_debt,
            btc_price
        );
        
        // 更新头寸
        position.health_factor = health_factor;
        position.last_updated_at = crate::ic_api::time();
        
        // 保存更新后的头寸
        crate::save_position(position);
    }
}

// 心跳函数，定期更新价格
#[ic_cdk_macros::heartbeat]
async fn heartbeat() {
    // 尝试更新价格
    match update_price().await {
        Ok(_) => {
            // 价格更新成功
        }
        Err(e) => {
            // 记录错误
            log_error(
                LogLevel::Error,
                &e,
                Some("Heartbeat price update failed")
            );
        }
    }
    
    // 检查是否有可清算的头寸
    check_liquidatable_positions();
}

// 检查可清算的头寸
fn check_liquidatable_positions() {
    // 获取所有头寸
    let positions = crate::get_positions();
    
    // 获取当前 BTC 价格
    let _btc_price = get_btc_price();
    
    // 检查每个头寸是否可清算
    for position in positions {
        // 获取池信息以获取清算阈值
        if let Some(pool) = crate::get_pool(&position.owner) {
            // 检查头寸是否可清算
            if position.is_liquidatable(pool.liquidation_threshold) {
                // 记录可清算的头寸
                ic_cdk::println!(
                    "Liquidatable position detected: id={}, health_factor={}, threshold={}",
                    position.id,
                    position.health_factor,
                    pool.liquidation_threshold
                );
            }
        }
    }
}

// 获取最后一个有效价格
#[query]
pub fn get_last_valid_price() -> Option<u64> {
    if is_price_valid() {
        Some(get_btc_price())
    } else {
        None
    }
}

// 模拟价格更新 (仅用于测试环境)
#[cfg(feature = "test-mode")]
#[update]
pub fn mock_price_update(price: u64) -> Result<()> {
    // 检查调用者是否为控制者
    let caller = crate::ic_api::caller();
    if !crate::ic_api::is_controller(&caller) {
        return Err(Error::PermissionDenied("Not authorized".to_string()));
    }
    
    // 额外的安全检查：验证价格合理性
    if price == 0 || price > 10_000_000 { // 最大 $100,000
        return Err(Error::InvalidArgument("价格超出合理范围".to_string()));
    }
    
    // 记录模拟价格更新
    ic_cdk::println!("WARNING: Using mock price update in test mode: ${}.{}", 
                     price / 100, price % 100);
    
    // 创建模拟价格数据
    let price_data = PriceData {
        price,
        timestamp: current_time_millis(),
        source: "mock_test_only".to_string(),
    };
    
    // 存储价格数据
    store_price_data(price_data);
    
    Ok(())
}

// 生产环境的紧急价格更新 (需要多重签名)
#[update]
pub fn emergency_price_update(price: u64, signatures: Vec<String>) -> Result<()> {
    // 验证调用者权限
    let caller = crate::ic_api::caller();
    if !crate::ic_api::is_controller(&caller) {
        return Err(Error::PermissionDenied("Not authorized".to_string()));
    }
    
    // 验证价格合理性
    if price == 0 || price > 10_000_000 {
        return Err(Error::InvalidArgument("价格超出合理范围".to_string()));
    }
    
    // 验证多重签名 (简化实现)
    if signatures.len() < 3 {
        return Err(Error::PermissionDenied("需要至少3个签名".to_string()));
    }
    
    // 在实际实现中，这里应该验证每个签名的有效性
    for (i, signature) in signatures.iter().enumerate() {
        if signature.is_empty() {
            return Err(Error::InvalidArgument(format!("签名 {} 无效", i)));
        }
    }
    
    // 记录紧急价格更新
    ic_cdk::println!("EMERGENCY: Price updated to ${}.{} with {} signatures", 
                     price / 100, price % 100, signatures.len());
    
    // 创建紧急价格数据
    let price_data = PriceData {
        price,
        timestamp: current_time_millis(),
        source: "emergency_update".to_string(),
    };
    
    // 存储价格数据
    store_price_data(price_data);
    
    Ok(())
}