// exchange.rs - REE 交互接口实现
// 这个模块将实现与 REE 的交互，包括交易执行、回滚和区块处理

use crate::{ExecuteTxGuard, Error, LogLevel, Result, error::log_error};
use ic_cdk_macros::{query, update};
use ree_types::orchestrator_interfaces::ensure_testnet4_orchestrator;
use ree_types::{
    bitcoin::Network,
    CoinBalance, CoinId, Intention, bitcoin::psbt::Psbt,
    exchange_interfaces::*,
    psbt::ree_pool_sign,
    schnorr::request_ree_pool_address,
};
use ic_cdk::api::management_canister::bitcoin::BitcoinNetwork;

#[query]
// 返回所有资金池列表
pub fn get_pool_list() -> GetPoolListResponse {
    // 获取所有池并转换为 PoolBasic 列表
    let pools = crate::get_pools();
    pools
        .iter()
        .map(|p| PoolBasic {
            name: p.meta.symbol.clone(),
            address: p.addr.clone(),
        })
        .collect()
}

#[query]
// 返回指定资金池的详细信息
pub fn get_pool_info(args: GetPoolInfoArgs) -> GetPoolInfoResponse {
    let GetPoolInfoArgs { pool_address } = args;
    
    // 获取指定池
    let p = match crate::get_pool(&pool_address) {
        Some(pool) => pool,
        None => return None,
    };

    // 构建池信息
    Some(PoolInfo {
        key: p.pubkey.clone(),
        name: p.meta.symbol.clone(),
        key_derivation_path: p.derivation_path(),
        address: p.addr.clone(),
        nonce: p.current_nonce(),
        btc_reserved: p.btc_balance(),
        coin_reserved: vec![CoinBalance {
            id: p.meta.id,
            value: p.bollar_balance(),
        }],
        utxos: p.current_state()
            .and_then(|s| s.utxo.clone())
            .map(|utxo| vec![utxo])
            .unwrap_or_default(),
        attributes: format!(
            "collateral_ratio={},liquidation_threshold={}",
            p.collateral_ratio,
            p.liquidation_threshold
        ),
    })
}

#[query]
// 返回交易所接受的最小交易金额
fn get_minimal_tx_value(args: GetMinimalTxValueArgs) -> GetMinimalTxValueResponse {
    // 根据未确认交易队列长度调整最小交易金额
    // 队列越长，要求的最小金额越高，以防止垃圾交易
    let min_value = if args.zero_confirmed_tx_queue_length > 10 {
        // 队列较长时，提高最小金额要求
        crate::types::MIN_BTC_VALUE * 2
    } else {
        crate::types::MIN_BTC_VALUE
    };
    
    min_value
}

#[update]
// 处理交易回滚
pub fn rollback_tx(args: RollbackTxArgs) -> std::result::Result<(), String> {
    // 使用错误日志记录
    log_error(
        LogLevel::Warning,
        &Error::SystemError(format!("回滚交易: {}", args.txid)),
        Some("rollback_tx")
    );
    
    // 查找交易记录
    crate::TX_RECORDS.with_borrow_mut(|m| {
        // 查找未确认和已确认的交易记录
        let maybe_unconfirmed_record = m.get(&(args.txid.clone(), false));
        let maybe_confirmed_record = m.get(&(args.txid.clone(), true));
        
        // 获取交易记录
        let record = maybe_confirmed_record
            .or(maybe_unconfirmed_record)
            .ok_or(format!("No record found for txid: {}", args.txid))?;

        ic_cdk::println!(
            "rollback txid: {} with pools: {:?}",
            args.txid,
            record.pools
        );

        // 回滚每个受影响的池
        record.pools.iter().for_each(|pool_address| {
            crate::POOLS.with_borrow_mut(|pools| {
                if let Some(mut pool) = pools.get(pool_address) {
                    if let Err(e) = pool.rollback(args.txid) {
                        ic_cdk::println!("Rollback failed: {:?}", e);
                    } else {
                        pools.insert(pool_address.clone(), pool);
                    }
                } else {
                    ic_cdk::println!("Pool not found: {}", pool_address);
                }
            });
        });

        // 删除交易记录
        m.remove(&(args.txid.clone(), false));
        m.remove(&(args.txid.clone(), true));

        Ok(())
    })
}

#[update]
// 处理新区块通知
pub fn new_block(args: NewBlockArgs) -> std::result::Result<(), String> {
    // 检查区块链重组
    match crate::reorg::detect_reorg(BitcoinNetwork::Testnet, args.clone()) {
        Ok(_) => {
            // 没有重组，正常处理
        }
        Err(crate::reorg::Error::DuplicateBlock { height, hash }) => {
            // 重复区块，记录日志
            ic_cdk::println!(
                "Duplicate block detected at height {} with hash {}",
                height,
                hash
            );
        }
        Err(crate::reorg::Error::Unrecoverable) => {
            // 不可恢复的重组，返回错误
            return Err("Unrecoverable reorg detected".to_string());
        }
        Err(crate::reorg::Error::Recoverable { height, depth }) => {
            // 可恢复的重组，处理重组
            crate::reorg::handle_reorg(height, depth);
        }
    }
    
    // 解构区块信息
    let NewBlockArgs {
        block_height,
        block_hash: _,
        block_timestamp: _,
        confirmed_txids,
    } = args.clone();

    // 存储新区块信息
    crate::BLOCKS.with_borrow_mut(|m| {
        m.insert(block_height, args);
        ic_cdk::println!("new block {} inserted into blocks", block_height);
    });

    // 将交易标记为已确认
    for txid in confirmed_txids {
        crate::TX_RECORDS.with_borrow_mut(|m| {
            if let Some(record) = m.remove(&(txid.clone(), false)) {
                m.insert((txid.clone(), true), record.clone());
                ic_cdk::println!("confirm txid: {} with pools: {:?}", txid, record.pools);
            }
        });
    }
    
    // 计算完全确认的区块高度（超过重组风险）
    let max_reorg_depth = crate::reorg::get_max_recoverable_reorg_depth(BitcoinNetwork::Testnet);
    let confirmed_height = block_height.saturating_sub(max_reorg_depth) + 1;

    // 确认已确认区块中的交易
    crate::BLOCKS.with_borrow(|m| {
        m.iter()
            .take_while(|(height, _)| *height <= confirmed_height)
            .for_each(|(height, block_info)| {
                ic_cdk::println!("finalizing txs in block: {}", height);
                
                // 确认区块中的每个交易
                block_info.confirmed_txids.iter().for_each(|txid| {
                    crate::TX_RECORDS.with_borrow_mut(|m| {
                        if let Some(record) = m.get(&(txid.clone(), true)) {
                            ic_cdk::println!(
                                "finalize txid: {} with pools: {:?}",
                                txid,
                                record.pools
                            );
                            
                            // 在每个受影响的池中确认交易
                            record.pools.iter().for_each(|pool_address| {
                                crate::POOLS.with_borrow_mut(|p| {
                                    if let Some(mut pool) = p.get(pool_address) {
                                        if let Err(e) = pool.finalize(txid.clone()) {
                                            ic_cdk::println!("Finalize failed: {:?}", e);
                                        } else {
                                            p.insert(pool_address.clone(), pool);
                                        }
                                    } else {
                                        ic_cdk::println!("Pool not found: {}", pool_address);
                                    }
                                });
                            });
                            
                            // 删除已确认的交易记录
                            m.remove(&(txid.clone(), true));
                        }
                    });
                });
            });
    });

    // 清理旧区块数据
    crate::BLOCKS.with_borrow_mut(|m| {
        let heights_to_remove: Vec<u32> = m
            .iter()
            .take_while(|(height, _)| *height <= confirmed_height)
            .map(|(height, _)| height)
            .collect();
            
        for height in heights_to_remove {
            ic_cdk::println!("removing block: {}", height);
            m.remove(&height);
        }
    });
    
    Ok(())
}

#[update]
// 执行交易
pub async fn execute_tx(args: ExecuteTxArgs) -> std::result::Result<String, String> {
    let ExecuteTxArgs {
        psbt_hex,
        txid,
        intention_set,
        intention_index,
        zero_confirmed_tx_queue_length: _,
    } = args;
    
    // 解码 PSBT
    let raw = hex::decode(&psbt_hex).map_err(|_| "invalid psbt".to_string())?;
    let mut psbt = Psbt::deserialize(raw.as_slice()).map_err(|_| "invalid psbt".to_string())?;

    // 获取意图
    let intention = intention_set.intentions[intention_index as usize].clone();
    let Intention {
        exchange_id: _,
        action,
        action_params: _,
        pool_address,
        nonce,
        pool_utxo_spent,
        pool_utxo_received,
        input_coins,
        output_coins,
    } = intention;

    // 创建交易执行锁，防止同一个池的并发交易
    let _guard = ExecuteTxGuard::new(pool_address.clone())
        .ok_or(format!("Pool {} Executing", pool_address))?;

    // 获取池
    let pool = crate::POOLS
        .with_borrow(|m| m.get(&pool_address).ok_or("Pool not found".to_string()))?;

    // 根据操作类型处理交易
    match action.as_ref() {
        "deposit" => {
            // 验证抵押交易
            let (new_state, consumed) = pool
                .validate_deposit(
                    txid,
                    nonce,
                    pool_utxo_spent,
                    pool_utxo_received,
                    input_coins,
                    output_coins,
                    0, // 这里需要从 action_params 中获取 bollar_mint_amount
                )
                .map_err(|e| e.to_string())?;

            // 如果有 UTXO 需要签名，则签名
            if let Some(ref utxo) = consumed {
                ree_pool_sign(
                    &mut psbt,
                    vec![utxo],
                    crate::SCHNORR_KEY_NAME,
                    pool.derivation_path(),
                )
                .await
                .map_err(|e| e.to_string())?;
            }

            // 更新池状态
            crate::POOLS.with_borrow_mut(|m| {
                let mut pool = m
                    .get(&pool_address)
                    .expect("already checked pool exists");
                pool.commit(new_state);
                m.insert(pool_address.clone(), pool);
            });
        }
        "repay" => {
            // 验证还款交易
            let (new_state, consumed) = pool
                .validate_repay(
                    txid,
                    nonce,
                    pool_utxo_spent,
                    pool_utxo_received,
                    input_coins,
                    output_coins,
                    "".to_string(), // 这里需要从 action_params 中获取 position_id
                )
                .map_err(|e| e.to_string())?;

            // 签名 UTXO
            ree_pool_sign(
                &mut psbt,
                vec![&consumed],
                crate::SCHNORR_KEY_NAME,
                pool.derivation_path(),
            )
            .await
            .map_err(|e| e.to_string())?;

            // 更新池状态
            crate::POOLS.with_borrow_mut(|m| {
                let mut pool = m
                    .get(&pool_address)
                    .expect("already checked pool exists");
                pool.commit(new_state);
                m.insert(pool_address.clone(), pool);
            });
        }
        "liquidate" => {
            // 验证清算交易
            let (new_state, consumed) = pool
                .validate_repay(
                    txid,
                    nonce,
                    pool_utxo_spent,
                    pool_utxo_received,
                    input_coins,
                    output_coins,
                    "".to_string(), // 这里需要从 action_params 中获取 position_id
                )
                .map_err(|e| e.to_string())?;

            // 签名 UTXO
            ree_pool_sign(
                &mut psbt,
                vec![&consumed],
                crate::SCHNORR_KEY_NAME,
                pool.derivation_path(),
            )
            .await
            .map_err(|e| e.to_string())?;

            // 更新池状态
            crate::POOLS.with_borrow_mut(|m| {
                let mut pool = m
                    .get(&pool_address)
                    .expect("already checked pool exists");
                pool.commit(new_state);
                m.insert(pool_address.clone(), pool);
            });
        }
        _ => {
            return Err("invalid action".to_string());
        }
    }

    // 记录未确认交易
    crate::TX_RECORDS.with_borrow_mut(|m| {
        ic_cdk::println!("new unconfirmed txid: {} in pool: {} ", txid, pool_address);
        
        // 创建或更新交易记录
        let mut record = m.get(&(txid.clone(), false)).unwrap_or_default();
        if !record.pools.contains(&pool_address) {
            record.pools.push(pool_address.clone());
        }
        
        // 设置交易记录的其他字段
        record.timestamp = crate::ic_api::time();
        record.action = action;
        record.user = crate::ic_api::caller().to_string();
        
        // 保存交易记录
        m.insert((txid.clone(), false), record);
    });

    // 返回序列化的 PSBT
    Ok(psbt.serialize_hex())
}
#[update]
// 初始化 Bollar 资金池
pub async fn init_bollar_pool(
    collateral_ratio: u8,
    liquidation_threshold: u8,
) -> Result<String> {
    // 验证调用者是否为控制者
    let caller = ic_cdk::api::caller();
    if !ic_cdk::api::is_controller(&caller) {
        return Err(Error::PermissionDenied("Not authorized".to_string()));
    }

    // 验证参数
    if collateral_ratio > 100 || collateral_ratio == 0 {
        return Err(Error::InvalidArgument("Invalid collateral ratio".to_string()));
    }
    if liquidation_threshold > 100 || liquidation_threshold <= collateral_ratio {
        return Err(Error::InvalidArgument("Invalid liquidation threshold".to_string()));
    }

    // 创建 Bollar 代币元数据
    let id = CoinId::rune(72798, 1058); // 示例 Rune ID，实际使用时需要替换
    let meta = crate::types::CoinMeta {
        id,
        symbol: "BOLLAR".to_string(),
        min_amount: 1,
    };

    // 请求 REE 池地址
    let (untweaked, tweaked, addr) = request_ree_pool_address(
        crate::SCHNORR_KEY_NAME,
        vec![id.to_string().as_bytes().to_vec()],
        Network::Testnet,
    )
    .await
    .map_err(|e| Error::SystemError(e.to_string()))?;

    // 初始化池
    let pool = crate::types::Pool {
        meta,
        pubkey: untweaked.clone(),
        tweaked,
        addr: addr.to_string(),
        states: vec![],
        collateral_ratio,
        liquidation_threshold,
    };
    
    // 存储池
    crate::POOLS.with_borrow_mut(|p| {
        p.insert(addr.to_string(), pool);
    });
    
    Ok(addr.to_string())
}