// liquidation.rs - 清算逻辑
// 这个模块实现清算条件检查和清算执行功能

use crate::{Error, LogLevel, Result, types::*};
use ic_cdk_macros::{query, update};

// 清算阈值
const LIQUIDATION_BONUS_PERCENT: u8 = 10; // 10% 奖励

#[query]
// 获取可清算头寸列表
pub fn get_liquidatable_positions() -> Vec<LiquidationOffer> {
    // 获取所有头寸
    let positions = crate::get_positions();
    
    // 获取当前 BTC 价格
    let btc_price = crate::oracle::get_btc_price();
    if btc_price == 0 {
        return vec![];
    }
    
    // 筛选可清算的头寸
    positions.iter()
        .filter_map(|position| {
            // 获取池信息以获取清算阈值
            let pool_address = position.id.split(':').next().unwrap_or("");
            let pool = crate::get_pool(&pool_address.to_string())?;
            
            // 重新计算当前健康因子
            let current_health_factor = crate::types::calculate_health_factor(
                position.btc_collateral,
                position.bollar_debt,
                btc_price
            );
            
            // 检查头寸是否可清算
            if current_health_factor < (pool.liquidation_threshold as u64) {
                // 计算清算奖励
                let liquidation_bonus = calculate_liquidation_reward(
                    position.bollar_debt,
                    position.btc_collateral,
                    position.bollar_debt,
                    btc_price
                );
                
                // 构建清算预处理结果
                Some(LiquidationOffer {
                    position_id: position.id.clone(),
                    owner: position.owner.clone(),
                    btc_collateral: position.btc_collateral,
                    bollar_debt: position.bollar_debt,
                    health_factor: current_health_factor,
                    liquidation_bonus,
                })
            } else {
                None
            }
        })
        .collect()
}

#[query]
// 预清算查询 - 返回用户需要的信息来构建清算交易
pub fn pre_liquidate(
    position_id: String,
    bollar_repay_amount: u64,
) -> Result<LiquidationOffer> {
    // 使用 catch_and_log 包装操作
    crate::error::catch_and_log(
        || {
            // 获取头寸
            let position = crate::get_position(&position_id)
                .ok_or(Error::PositionNotFound)?;
            
            // 获取池信息以获取清算阈值
            let pool_address = position_id.split(':').next().unwrap_or("");
            let pool = crate::get_pool(&pool_address.to_string())
                .ok_or(Error::InvalidPool)?;
            
            // 获取当前 BTC 价格
            let btc_price = crate::oracle::get_btc_price();
            if btc_price == 0 {
                return Err(Error::OracleError("无效的 BTC 价格".to_string()));
            }
            
            // 重新计算当前健康因子
            let current_health_factor = crate::types::calculate_health_factor(
                position.btc_collateral,
                position.bollar_debt,
                btc_price
            );
            
            // 检查头寸是否可清算
            if current_health_factor >= (pool.liquidation_threshold as u64) {
                return Err(Error::PositionNotLiquidatable);
            }
            
            // 验证还款金额
            if bollar_repay_amount == 0 || bollar_repay_amount > position.bollar_debt {
                return Err(Error::InvalidArgument(format!(
                    "无效的还款金额，应在 1 到 {} 之间",
                    position.bollar_debt
                )));
            }
            
            // 计算清算奖励
            let liquidation_bonus = calculate_liquidation_reward(
                bollar_repay_amount,
                position.btc_collateral,
                position.bollar_debt,
                btc_price
            );
            
            // 构建清算预处理结果
            let offer = LiquidationOffer {
                position_id: position.id.clone(),
                owner: position.owner.clone(),
                btc_collateral: position.btc_collateral,
                bollar_debt: position.bollar_debt,
                health_factor: position.health_factor,
                liquidation_bonus,
            };
            
            Ok(offer)
        },
        LogLevel::Warning,
        &format!("pre_liquidate: 预清算查询失败, id={}", position_id)
    )
}

#[update]
// 执行清算操作
pub async fn execute_liquidate(
    position_id: String,
    signed_psbt: String,
) -> Result<String> {
    // 检查紧急状态
    check_emergency_state!("liquidate");
    
    // 检查权限
    require_permission!(
        crate::access_control::has_permission(ic_api::caller(), crate::access_control::Permission::Liquidate),
        "Liquidation permission required"
    );
    
    // 开始状态事务
    let tx_id = crate::state_manager::StateManager::begin_transaction(
        crate::state_manager::StateOperation::Liquidation
    )?;
    // 获取头寸
    let position = crate::get_position(&position_id)
        .ok_or(Error::PositionNotFound)?;
    
    // 获取池信息以获取清算阈值
    let pool_address = position_id.split(':').next().unwrap_or("");
    let pool = crate::get_pool(&pool_address.to_string())
        .ok_or(Error::InvalidPool)?;
    
    // 获取当前 BTC 价格
    let btc_price = crate::oracle::get_btc_price();
    if btc_price == 0 {
        return Err(Error::OracleError("无效的 BTC 价格".to_string()));
    }
    
    // 重新计算当前健康因子
    let current_health_factor = crate::types::calculate_health_factor(
        position.btc_collateral,
        position.bollar_debt,
        btc_price
    );
    
    // 检查头寸是否可清算
    if current_health_factor >= (pool.liquidation_threshold as u64) {
        return Err(Error::PositionNotLiquidatable);
    }
    
    // 解码 PSBT
    let _psbt_bytes = hex::decode(&signed_psbt)
        .map_err(|_| Error::InvalidArgument("无效的 PSBT 十六进制字符串".to_string()))?;
    
    // 在实际实现中，这里需要验证 PSBT 并执行交易
    // 这里简化处理，假设交易已成功执行
    
    // 获取调用者身份
    let caller = crate::ic_api::caller().to_string();
    
    // 假设从 PSBT 中提取的 Bollar 数量
    // 在实际实现中，应该从 PSBT 中解析
    // 如果健康因子非常低（< 60%），进行全额清算，否则部分清算
    ic_cdk::println!(
        "Liquidation decision: health_factor={}, threshold=60, debt={}",
        current_health_factor,
        position.bollar_debt
    );
    
    let bollar_repay_amount = if current_health_factor < 60 {
        ic_cdk::println!("Full liquidation");
        position.bollar_debt // 全额清算
    } else {
        ic_cdk::println!("Partial liquidation");
        position.bollar_debt / 2 // 部分清算
    };
    
    // 计算清算奖励
    let liquidation_bonus = calculate_liquidation_reward(
        bollar_repay_amount,
        position.btc_collateral,
        position.bollar_debt,
        btc_price
    );
    
    // 计算清算人获得的 BTC 数量
    let liquidator_btc = (position.btc_collateral as u128) * (bollar_repay_amount as u128) / (position.bollar_debt as u128);
    let liquidator_btc_with_bonus = liquidator_btc + (liquidation_bonus as u128);
    
    // 如果是全额清算，删除头寸
    if bollar_repay_amount == position.bollar_debt {
        crate::delete_position(&position_id);
    } else {
        // 否则更新头寸
        let mut updated_position = position.clone();
        updated_position.update(
            position.btc_collateral - (liquidator_btc_with_bonus as u64),
            position.bollar_debt - bollar_repay_amount,
            btc_price,
        );
        crate::save_position(updated_position);
    }
    
    // 提交事务
    if let Err(e) = crate::state_manager::StateManager::commit_transaction(tx_id) {
        secure_log_error!(LogCategory::System, format!("Liquidation transaction commit failed: {:?}", e));
        return Err(e);
    }
    
    // 记录清算事件
    secure_log_info!(
        LogCategory::Liquidation,
        format!("Liquidation executed: position_id={}", position_id),
        format!("Liquidator: {}, Bollar repaid: {}, BTC reward: {}", 
                caller, bollar_repay_amount, liquidator_btc_with_bonus)
    );
    
    // 返回交易 ID
    Ok(format!("liquidate:{}", crate::ic_api::time()))
}

// 使用 types 模块中的健康因子计算函数
#[allow(dead_code)]
pub fn calculate_health_factor(
    btc_collateral: u64,
    bollar_debt: u64,
    btc_price: u64,
) -> u64 {
    crate::types::calculate_health_factor(btc_collateral, bollar_debt, btc_price)
}

// 计算清算奖励 (使用安全数学运算)
pub fn calculate_liquidation_reward(
    bollar_repay_amount: u64,
    btc_collateral: u64,
    bollar_debt: u64,
    btc_price: u64,
) -> u64 {
    match crate::safe_math::safe_calculate_liquidation_reward(
        bollar_repay_amount,
        btc_collateral,
        bollar_debt,
        btc_price,
        LIQUIDATION_BONUS_PERCENT,
    ) {
        Ok(reward) => reward,
        Err(e) => {
            crate::error::log_error(
                crate::LogLevel::Error,
                &e,
                Some("calculate_liquidation_reward failed")
            );
            0 // 返回 0，防止奖励计算错误
        }
    }
}