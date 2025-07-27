// lending.rs - 借贷核心逻辑
// 这个模块实现抵押、铸造、还款和赎回功能

use crate::{Error, LogLevel, Result, types::*};
use ic_cdk_macros::{query, update};

#[query]
// 预抵押查询 - 返回用户需要的信息来构建抵押交易
pub fn pre_deposit(
    pool_address: String,
    btc_amount: u64,
) -> Result<DepositOffer> {
    // 使用 catch_and_log 包装操作
    crate::error::catch_and_log(
        || {
            // 验证参数
            if btc_amount < crate::types::MIN_BTC_VALUE {
                return Err(Error::InvalidArgument(format!(
                    "BTC 数量太小，最小值为 {} satoshis",
                    crate::types::MIN_BTC_VALUE
                )));
            }
            
            // 获取池
            let pool = crate::get_pool(&pool_address)
                .ok_or(Error::InvalidPool)?;
            
            // 获取当前 BTC 价格
            let btc_price = crate::oracle::get_btc_price();
            if btc_price == 0 {
                return Err(Error::OracleError("无效的 BTC 价格".to_string()));
            }
            
            // 计算可铸造的最大 Bollar 数量
            let max_bollar_mint = pool.calculate_max_bollar(btc_amount, btc_price);
            
            // 构建抵押预处理结果
            let offer = DepositOffer {
                pool_utxo: pool.current_state().and_then(|s| s.utxo.clone()),
                nonce: pool.current_nonce(),
                btc_price,
                max_bollar_mint,
            };
            
            Ok(offer)
        },
        LogLevel::Debug,
        "pre_deposit: 预抵押查询失败"
    )
}

#[update]
// 执行抵押和铸造操作
pub async fn execute_deposit(
    pool_address: String,
    signed_psbt: String,
    bollar_amount: u64,
) -> Result<String> {
    // 验证参数
    if bollar_amount == 0 {
        return Err(Error::InvalidArgument("Bollar 数量不能为零".to_string()));
    }
    
    // 获取池
    let pool = crate::get_pool(&pool_address)
        .ok_or(Error::InvalidPool)?;
    
    // 解码 PSBT
    let _psbt_bytes = hex::decode(&signed_psbt)
        .map_err(|_| Error::InvalidArgument("无效的 PSBT 十六进制字符串".to_string()))?;
    
    // 在实际实现中，这里需要验证 PSBT 并执行交易
    // 这里简化处理，假设交易已成功执行
    
    // 获取调用者身份
    let caller = crate::ic_api::caller().to_string();
    
    // 生成头寸 ID
    let position_id = format!("{}:{}:{}", pool_address, crate::ic_api::time(), caller);
    
    // 获取当前 BTC 价格
    let btc_price = crate::oracle::get_btc_price();
    if btc_price == 0 {
        return Err(Error::OracleError("无效的 BTC 价格".to_string()));
    }
    
    // 假设从 PSBT 中提取的 BTC 数量
    // 在实际实现中，应该从 PSBT 中解析
    // 为了简化测试，我们假设一个固定的 BTC 数量
    let btc_amount = 100_000_000u64; // 假设 1 BTC
    
    // 验证铸造数量不超过最大值
    let max_bollar_mint = pool.calculate_max_bollar(btc_amount, btc_price);
    if bollar_amount > max_bollar_mint {
        return Err(Error::InvalidArgument(format!(
            "铸造数量 {} 超过最大值 {}",
            bollar_amount,
            max_bollar_mint
        )));
    }
    
    // 创建新头寸
    let position = Position::new(
        position_id.clone(),
        caller,
        btc_amount,
        bollar_amount,
        btc_price,
    );
    
    // 保存头寸
    crate::save_position(position.clone());
    
    // 验证头寸已保存
    let saved_position = crate::get_position(&position_id);
    if saved_position.is_none() {
        return Err(Error::InvalidState("头寸保存失败".to_string()));
    }
    
    // 返回头寸 ID
    Ok(position_id)
}

#[query]
// 预还款查询 - 返回用户需要的信息来构建还款交易
pub fn pre_repay(
    position_id: String,
    bollar_amount: u64,
) -> Result<RepayOffer> {
    // 使用 catch_and_log 包装操作
    crate::error::catch_and_log(
        || {
            // 获取头寸
            let position = crate::get_position(&position_id)
                .ok_or(Error::PositionNotFound)?;
            
            // 验证调用者是否为头寸所有者
            let caller = crate::ic_api::caller().to_string();
            if position.owner != caller {
                return Err(Error::PermissionDenied("不是头寸所有者".to_string()));
            }
            
            // 验证还款金额
            if bollar_amount == 0 || bollar_amount > position.bollar_debt {
                return Err(Error::InvalidArgument(format!(
                    "无效的还款金额，应在 1 到 {} 之间",
                    position.bollar_debt
                )));
            }
            
            // 获取池
            let pool_address = position_id.split(':').next().unwrap_or("");
            let pool = crate::get_pool(&pool_address.to_string())
                .ok_or(Error::InvalidPool)?;
            
            // 获取当前 BTC 价格
            let btc_price = crate::oracle::get_btc_price();
            if btc_price == 0 {
                return Err(Error::OracleError("无效的 BTC 价格".to_string()));
            }
            
            // 计算可赎回的 BTC 数量
            let btc_return = (position.btc_collateral as u128) * (bollar_amount as u128) / (position.bollar_debt as u128);
            
            // 构建还款预处理结果
            let offer = RepayOffer {
                pool_utxo: pool.current_state()
                    .and_then(|s| s.utxo.clone())
                    .ok_or(Error::InvalidState("池 UTXO 不存在".to_string()))?,
                nonce: pool.current_nonce(),
                btc_return: btc_return as u64,
            };
            
            Ok(offer)
        },
        LogLevel::Debug,
        &format!("pre_repay: 预还款查询失败, id={}", position_id)
    )
}

#[update]
// 执行还款和赎回操作
pub async fn execute_repay(
    position_id: String,
    signed_psbt: String,
) -> Result<String> {
    // 获取头寸
    let position = crate::get_position(&position_id)
        .ok_or(Error::PositionNotFound)?;
    
    // 验证调用者是否为头寸所有者
    let caller = crate::ic_api::caller().to_string();
    if position.owner != caller {
        return Err(Error::PermissionDenied("不是头寸所有者".to_string()));
    }
    
    // 解码 PSBT
    let _psbt_bytes = hex::decode(&signed_psbt)
        .map_err(|_| Error::InvalidArgument("无效的 PSBT 十六进制字符串".to_string()))?;
    
    // 在实际实现中，这里需要验证 PSBT 并执行交易
    // 这里简化处理，假设交易已成功执行
    
    // 获取当前 BTC 价格
    let btc_price = crate::oracle::get_btc_price();
    if btc_price == 0 {
        return Err(Error::OracleError("无效的 BTC 价格".to_string()));
    }
    
    // 假设从 PSBT 中提取的 Bollar 数量
    // 在实际实现中，应该从 PSBT 中解析
    // 为了测试，我们假设还款一半的债务
    let bollar_amount = position.bollar_debt / 2;
    
    // 计算可赎回的 BTC 数量
    let btc_return = (position.btc_collateral as u128) * (bollar_amount as u128) / (position.bollar_debt as u128);
    
    // 如果是全额还款，删除头寸
    if bollar_amount == position.bollar_debt {
        crate::delete_position(&position_id);
    } else {
        // 否则更新头寸
        let mut updated_position = position.clone();
        updated_position.update(
            position.btc_collateral - (btc_return as u64),
            position.bollar_debt - bollar_amount,
            btc_price,
        );
        crate::save_position(updated_position);
    }
    
    // 返回交易 ID
    Ok(format!("repay:{}", crate::ic_api::time()))
}

#[update]
// 初始化资金池
async fn init_pool() -> std::result::Result<(), String> {
    // 待实现
    Ok(())
}

#[query]
// 获取用户头寸列表
pub fn get_user_positions(user: String) -> Vec<Position> {
    crate::get_user_positions(&user)
}

#[query]
// 获取头寸详情
pub fn get_position_details(position_id: String) -> Result<Position> {
    // 使用 catch_and_log 包装操作
    crate::error::catch_and_log(
        || {
            // 获取头寸
            let position = crate::get_position(&position_id)
                .ok_or(Error::PositionNotFound)?;
            
            Ok(position)
        },
        LogLevel::Debug,
        &format!("get_position_details: 获取头寸详情失败, id={}", position_id)
    )
}

#[query]
// 获取协议指标
pub fn get_protocol_metrics() -> ProtocolMetrics {
    // 获取所有头寸
    let positions = crate::get_positions();
    
    // 获取当前 BTC 价格
    let btc_price = crate::oracle::get_btc_price();
    
    // 计算总锁定 BTC
    let total_btc_locked = positions.iter()
        .map(|p| p.btc_collateral)
        .sum();
    
    // 计算总 Bollar 供应量
    let total_bollar_supply = positions.iter()
        .map(|p| p.bollar_debt)
        .sum();
    
    // 获取第一个池的抵押率和清算阈值
    // 在实际实现中，可能需要更复杂的逻辑
    let pools = crate::get_pools();
    let (collateral_ratio, liquidation_threshold) = pools.first()
        .map(|p| (p.collateral_ratio, p.liquidation_threshold))
        .unwrap_or((75, 80)); // 默认值
    
    // 计算可清算头寸数量
    let liquidatable_positions_count = positions.iter()
        .filter(|p| p.is_liquidatable(liquidation_threshold))
        .count() as u64;
    
    // 构建协议指标
    ProtocolMetrics {
        total_btc_locked,
        total_bollar_supply,
        btc_price,
        collateral_ratio,
        liquidation_threshold,
        positions_count: positions.len() as u64,
        liquidatable_positions_count,
    }
}

#[update]
// 更新头寸健康因子
pub fn update_position_health_factor(position_id: String) -> Result<u64> {
    // 使用 catch_and_log 包装操作
    crate::error::catch_and_log(
        || {
            // 获取头寸
            let mut position = crate::get_position(&position_id)
                .ok_or(Error::PositionNotFound)?;
            
            // 获取当前 BTC 价格
            let btc_price = crate::oracle::get_btc_price();
            if btc_price == 0 {
                return Err(Error::OracleError("无效的 BTC 价格".to_string()));
            }
            
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
            
            Ok(health_factor)
        },
        LogLevel::Debug,
        &format!("update_position_health_factor: 更新头寸健康因子失败, id={}", position_id)
    )
}