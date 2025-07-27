// lending.rs - 借贷核心逻辑
// 这个模块实现抵押、铸造、还款和赎回功能

use crate::{Error, LogLevel, Result, types::*};
use ic_cdk_macros::{query, update};
use bitcoin::psbt::Psbt;
use bitcoin::{Transaction, TxOut, Address, Amount};
use std::str::FromStr;

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
    // 获取调用者身份
    let caller = crate::ic_api::caller().to_string();
    
    // 检查紧急状态
    check_emergency_state!("deposit");
    
    // 获取组合锁，防止重入攻击
    let _guard = crate::CombinedGuard::new(caller.clone(), pool_address.clone())
        .ok_or(Error::SystemError("系统繁忙，请稍后重试".to_string()))?;
    // 验证参数
    if bollar_amount == 0 {
        return Err(Error::InvalidArgument("Bollar 数量不能为零".to_string()));
    }
    
    // 获取池
    let pool = crate::get_pool(&pool_address)
        .ok_or(Error::InvalidPool)?;
    
    // 解码并验证 PSBT
    let psbt = validate_and_parse_psbt(&signed_psbt, &pool_address, bollar_amount)?;
    
    // 验证 PSBT 的输入输出
    let btc_amount = validate_deposit_psbt(&psbt, &pool, bollar_amount)?;
    
    // 生成头寸 ID
    let position_id = format!("{}:{}:{}", pool_address, crate::ic_api::time(), caller);
    
    // 获取当前 BTC 价格
    let btc_price = crate::oracle::get_btc_price();
    if btc_price == 0 {
        return Err(Error::OracleError("无效的 BTC 价格".to_string()));
    }
    
    // btc_amount 已经从 PSBT 验证中获得
    
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
    // 获取调用者身份
    let caller = crate::ic_api::caller().to_string();
    
    // 从 position_id 提取池地址
    let pool_address = position_id.split(':').next()
        .ok_or(Error::InvalidArgument("无效的头寸ID格式".to_string()))?
        .to_string();
    
    // 检查紧急状态
    check_emergency_state!("repay");
    
    // 获取组合锁，防止重入攻击
    let _guard = crate::CombinedGuard::new(caller.clone(), pool_address)
        .ok_or(Error::SystemError("系统繁忙，请稍后重试".to_string()))?;
    // 获取头寸
    let position = crate::get_position(&position_id)
        .ok_or(Error::PositionNotFound)?;
    
    // 验证调用者是否为头寸所有者
    if position.owner != caller {
        return Err(Error::PermissionDenied("不是头寸所有者".to_string()));
    }
    
    // 解码并验证 PSBT
    let psbt = validate_and_parse_psbt(&signed_psbt, &position_id, 0)?;
    
    // 验证还款 PSBT
    let bollar_amount = validate_repay_psbt(&psbt, &position)?;
    
    // 获取当前 BTC 价格
    let btc_price = crate::oracle::get_btc_price();
    if btc_price == 0 {
        return Err(Error::OracleError("无效的 BTC 价格".to_string()));
    }
    
    // bollar_amount 已经从 PSBT 验证中获得
    
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

// PSBT 验证和解析函数
fn validate_and_parse_psbt(psbt_hex: &str, context: &str, expected_amount: u64) -> Result<Psbt> {
    // 解码十六进制字符串
    let psbt_bytes = hex::decode(psbt_hex)
        .map_err(|_| Error::InvalidArgument("无效的 PSBT 十六进制字符串".to_string()))?;
    
    // 解析 PSBT
    let psbt = Psbt::deserialize(&psbt_bytes)
        .map_err(|e| Error::InvalidArgument(format!("PSBT 解析失败: {}", e)))?;
    
    // 基本验证
    if psbt.inputs.is_empty() {
        return Err(Error::InvalidArgument("PSBT 必须包含至少一个输入".to_string()));
    }
    
    if psbt.outputs.is_empty() {
        return Err(Error::InvalidArgument("PSBT 必须包含至少一个输出".to_string()));
    }
    
    // 验证 PSBT 的完整性
    validate_psbt_integrity(&psbt)?;
    
    // 记录验证成功
    ic_cdk::println!("PSBT 验证成功: context={}, inputs={}, outputs={}", 
                     context, psbt.inputs.len(), psbt.outputs.len());
    
    Ok(psbt)
}

// 验证 PSBT 完整性
fn validate_psbt_integrity(psbt: &Psbt) -> Result<()> {
    // 检查输入和输出数量匹配
    if psbt.inputs.len() != psbt.unsigned_tx.input.len() {
        return Err(Error::InvalidArgument("PSBT 输入数量不匹配".to_string()));
    }
    
    if psbt.outputs.len() != psbt.unsigned_tx.output.len() {
        return Err(Error::InvalidArgument("PSBT 输出数量不匹配".to_string()));
    }
    
    // 验证手续费合理性
    let total_input_value = calculate_total_input_value(psbt)?;
    let total_output_value = calculate_total_output_value(psbt);
    
    if total_input_value <= total_output_value {
        return Err(Error::InvalidArgument("PSBT 输入价值必须大于输出价值".to_string()));
    }
    
    let fee = total_input_value - total_output_value;
    let max_reasonable_fee = total_input_value / 100; // 最大 1% 手续费
    
    if fee > max_reasonable_fee {
        return Err(Error::InvalidArgument(format!(
            "手续费过高: {} satoshis (最大允许: {})", 
            fee, max_reasonable_fee
        )));
    }
    
    // 验证所有输入都有必要的见证数据或签名
    for (i, input) in psbt.inputs.iter().enumerate() {
        if input.witness_utxo.is_none() && input.non_witness_utxo.is_none() {
            return Err(Error::InvalidArgument(format!(
                "输入 {} 缺少 UTXO 信息", i
            )));
        }
    }
    
    Ok(())
}

// 计算总输入价值
fn calculate_total_input_value(psbt: &Psbt) -> Result<u64> {
    let mut total = 0u64;
    
    for input in &psbt.inputs {
        let value = if let Some(witness_utxo) = &input.witness_utxo {
            witness_utxo.value
        } else if let Some(non_witness_utxo) = &input.non_witness_utxo {
            // 需要找到对应的输出
            let prev_out_index = psbt.unsigned_tx.input
                .iter()
                .position(|tx_in| {
                    // 这里需要更复杂的逻辑来匹配输入
                    true // 简化处理
                })
                .ok_or(Error::InvalidArgument("无法找到对应的输入".to_string()))?;
            
            non_witness_utxo.output.get(prev_out_index)
                .ok_or(Error::InvalidArgument("无效的输出索引".to_string()))?
                .value
        } else {
            return Err(Error::InvalidArgument("输入缺少 UTXO 信息".to_string()));
        };
        
        total = total.checked_add(value)
            .ok_or(Error::Overflow)?;
    }
    
    Ok(total)
}

// 计算总输出价值
fn calculate_total_output_value(psbt: &Psbt) -> u64 {
    psbt.unsigned_tx.output.iter()
        .map(|output| output.value)
        .sum()
}

// 验证抵押 PSBT
fn validate_deposit_psbt(psbt: &Psbt, pool: &Pool, expected_bollar: u64) -> Result<u64> {
    // 验证至少有一个输出到池地址
    let pool_address = Address::from_str(&pool.addr)
        .map_err(|_| Error::InvalidArgument("无效的池地址".to_string()))?
        .assume_checked();
    
    let mut btc_to_pool = 0u64;
    let mut found_pool_output = false;
    
    for output in &psbt.unsigned_tx.output {
        // 检查是否是发送到池地址的输出
        if output.script_pubkey == pool_address.script_pubkey() {
            btc_to_pool = btc_to_pool.checked_add(output.value)
                .ok_or(Error::Overflow)?;
            found_pool_output = true;
        }
    }
    
    if !found_pool_output {
        return Err(Error::InvalidArgument("PSBT 必须包含发送到池地址的输出".to_string()));
    }
    
    // 验证抵押数量是否足够
    let min_collateral = crate::types::MIN_BTC_VALUE;
    if btc_to_pool < min_collateral {
        return Err(Error::InvalidArgument(format!(
            "抵押数量不足: {} satoshis (最小: {})", 
            btc_to_pool, min_collateral
        )));
    }
    
    // 验证铸造数量是否合理
    let btc_price = crate::oracle::get_btc_price();
    let max_bollar = pool.calculate_max_bollar(btc_to_pool, btc_price);
    
    if expected_bollar > max_bollar {
        return Err(Error::InvalidArgument(format!(
            "铸造数量超过最大值: {} (最大: {})", 
            expected_bollar, max_bollar
        )));
    }
    
    Ok(btc_to_pool)
}

// 验证还款 PSBT
fn validate_repay_psbt(psbt: &Psbt, position: &Position) -> Result<u64> {
    // 这里需要验证 PSBT 包含正确的 Bollar 代币输入
    // 由于 Bollar 是 Runes 代币，需要特殊的验证逻辑
    
    // 简化实现：假设从交易中提取 Bollar 数量
    // 在实际实现中，需要解析 Runes 协议的输出
    
    let bollar_amount = position.bollar_debt / 2; // 简化：还款一半
    
    // 验证还款数量不超过债务
    if bollar_amount > position.bollar_debt {
        return Err(Error::InvalidArgument(format!(
            "还款数量超过债务: {} (债务: {})", 
            bollar_amount, position.bollar_debt
        )));
    }
    
    Ok(bollar_amount)
}