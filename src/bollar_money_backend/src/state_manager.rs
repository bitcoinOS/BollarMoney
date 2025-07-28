// state_manager.rs - 状态管理和一致性保证
// 这个模块实现系统状态的一致性管理和事务性操作

use crate::{Error, LogLevel, Result, types::*, ic_api, secure_logging::*};
use candid::{CandidType, Deserialize};
use ic_cdk_macros::{query, update};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

// 状态操作类型
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum StateOperation {
    CreatePosition,
    UpdatePosition,
    DeletePosition,
    UpdatePool,
    CreatePool,
    UpdatePrice,
    Liquidation,
    Emergency,
}

// 状态快照
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct StateSnapshot {
    pub id: String,
    pub timestamp: u64,
    pub operation: StateOperation,
    pub pools_snapshot: HashMap<String, Pool>,
    pub positions_snapshot: HashMap<String, Position>,
    pub metadata: HashMap<String, String>,
}

// 状态事务
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct StateTransaction {
    pub id: String,
    pub timestamp: u64,
    pub operation: StateOperation,
    pub principal: String,
    pub before_snapshot: Option<StateSnapshot>,
    pub after_snapshot: Option<StateSnapshot>,
    pub committed: bool,
    pub rollback_reason: Option<String>,
}

// 状态验证结果
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct StateValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub total_btc_locked: u64,
    pub total_bollar_supply: u64,
    pub positions_count: u64,
    pub pools_count: u64,
}

thread_local! {
    // 状态快照存储
    static STATE_SNAPSHOTS: RefCell<VecDeque<StateSnapshot>> = RefCell::new(VecDeque::new());
    
    // 状态事务日志
    static STATE_TRANSACTIONS: RefCell<VecDeque<StateTransaction>> = RefCell::new(VecDeque::new());
    
    // 状态锁
    static STATE_LOCK: RefCell<bool> = RefCell::new(false);
    
    // 自动快照配置
    static AUTO_SNAPSHOT_CONFIG: RefCell<AutoSnapshotConfig> = RefCell::new(AutoSnapshotConfig::default());
}

// 自动快照配置
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct AutoSnapshotConfig {
    pub enabled: bool,
    pub interval_minutes: u64,
    pub max_snapshots: usize,
    pub operations_threshold: u64,
}

impl Default for AutoSnapshotConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_minutes: 60, // 每小时一次
            max_snapshots: 168,   // 保留一周
            operations_threshold: 100, // 100次操作后强制快照
        }
    }
}

// 状态管理器
pub struct StateManager;

impl StateManager {
    // 创建状态快照
    pub fn create_snapshot(operation: StateOperation, metadata: Option<HashMap<String, String>>) -> Result<String> {
        let timestamp = ic_api::time();
        let id = format!("snapshot_{}_{}", timestamp, operation as u8);
        
        // 获取当前状态
        let pools_snapshot = crate::get_pools().into_iter()
            .map(|pool| (pool.addr.clone(), pool))
            .collect();
        
        let positions_snapshot = crate::get_positions().into_iter()
            .map(|pos| (pos.id.clone(), pos))
            .collect();
        
        let snapshot = StateSnapshot {
            id: id.clone(),
            timestamp,
            operation,
            pools_snapshot,
            positions_snapshot,
            metadata: metadata.unwrap_or_default(),
        };
        
        // 存储快照
        STATE_SNAPSHOTS.with_borrow_mut(|snapshots| {
            snapshots.push_back(snapshot.clone());
            
            // 维护快照数量限制
            let max_snapshots = AUTO_SNAPSHOT_CONFIG.with_borrow(|config| config.max_snapshots);
            while snapshots.len() > max_snapshots {
                snapshots.pop_front();
            }
        });
        
        secure_log_info!(
            LogCategory::System,
            format!("State snapshot created: {}", id),
            format!("Operation: {:?}", operation)
        );
        
        Ok(id)
    }
    
    // 开始状态事务
    pub fn begin_transaction(operation: StateOperation) -> Result<String> {
        // 检查状态锁
        let locked = STATE_LOCK.with_borrow(|lock| *lock);
        if locked {
            return Err(Error::SystemError("State is locked for transaction".to_string()));
        }
        
        // 设置状态锁
        STATE_LOCK.with_borrow_mut(|lock| *lock = true);
        
        let timestamp = ic_api::time();
        let id = format!("tx_{}_{}", timestamp, operation as u8);
        let principal = ic_api::caller().to_string();
        
        // 创建事务前快照
        let before_snapshot_id = Self::create_snapshot(operation.clone(), None)?;
        let before_snapshot = STATE_SNAPSHOTS.with_borrow(|snapshots| {
            snapshots.iter().find(|s| s.id == before_snapshot_id).cloned()
        });
        
        let transaction = StateTransaction {
            id: id.clone(),
            timestamp,
            operation,
            principal,
            before_snapshot,
            after_snapshot: None,
            committed: false,
            rollback_reason: None,
        };
        
        STATE_TRANSACTIONS.with_borrow_mut(|transactions| {
            transactions.push_back(transaction);
        });
        
        secure_log_info!(
            LogCategory::System,
            format!("State transaction started: {}", id),
            format!("Principal: {}", principal)
        );
        
        Ok(id)
    }
    
    // 提交状态事务
    pub fn commit_transaction(transaction_id: String) -> Result<()> {
        let mut transaction = STATE_TRANSACTIONS.with_borrow_mut(|transactions| {
            transactions.iter_mut()
                .find(|tx| tx.id == transaction_id)
                .ok_or(Error::InvalidArgument("Transaction not found".to_string()))?
                .clone()
        })?;
        
        // 创建事务后快照
        let after_snapshot_id = Self::create_snapshot(transaction.operation.clone(), None)?;
        let after_snapshot = STATE_SNAPSHOTS.with_borrow(|snapshots| {
            snapshots.iter().find(|s| s.id == after_snapshot_id).cloned()
        });
        
        // 更新事务状态
        transaction.after_snapshot = after_snapshot;
        transaction.committed = true;
        
        STATE_TRANSACTIONS.with_borrow_mut(|transactions| {
            if let Some(tx) = transactions.iter_mut().find(|tx| tx.id == transaction_id) {
                *tx = transaction.clone();
            }
        });
        
        // 释放状态锁
        STATE_LOCK.with_borrow_mut(|lock| *lock = false);
        
        secure_log_info!(
            LogCategory::System,
            format!("State transaction committed: {}", transaction_id),
            format!("Operation: {:?}", transaction.operation)
        );
        
        Ok(())
    }
    
    // 回滚状态事务
    pub fn rollback_transaction(transaction_id: String, reason: String) -> Result<()> {
        let mut transaction = STATE_TRANSACTIONS.with_borrow_mut(|transactions| {
            transactions.iter_mut()
                .find(|tx| tx.id == transaction_id)
                .ok_or(Error::InvalidArgument("Transaction not found".to_string()))?
                .clone()
        })?;
        
        // 恢复到事务前状态
        if let Some(before_snapshot) = &transaction.before_snapshot {
            Self::restore_from_snapshot(&before_snapshot.id)?;
        }
        
        // 更新事务状态
        transaction.rollback_reason = Some(reason.clone());
        
        STATE_TRANSACTIONS.with_borrow_mut(|transactions| {
            if let Some(tx) = transactions.iter_mut().find(|tx| tx.id == transaction_id) {
                *tx = transaction.clone();
            }
        });
        
        // 释放状态锁
        STATE_LOCK.with_borrow_mut(|lock| *lock = false);
        
        secure_log_warning!(
            LogCategory::System,
            format!("State transaction rolled back: {}", transaction_id),
            format!("Reason: {}", reason)
        );
        
        Ok(())
    }
    
    // 从快照恢复状态
    pub fn restore_from_snapshot(snapshot_id: &str) -> Result<()> {
        let snapshot = STATE_SNAPSHOTS.with_borrow(|snapshots| {
            snapshots.iter()
                .find(|s| s.id == snapshot_id)
                .cloned()
                .ok_or(Error::InvalidArgument("Snapshot not found".to_string()))
        })?;
        
        // 恢复池状态
        for (addr, pool) in snapshot.pools_snapshot {
            crate::save_pool(pool);
        }
        
        // 恢复头寸状态
        // 首先清除所有现有头寸
        let current_positions = crate::get_positions();
        for position in current_positions {
            crate::delete_position(&position.id);
        }
        
        // 然后恢复快照中的头寸
        for (id, position) in snapshot.positions_snapshot {
            crate::save_position(position);
        }
        
        secure_log_info!(
            LogCategory::System,
            format!("State restored from snapshot: {}", snapshot_id),
            format!("Timestamp: {}", snapshot.timestamp)
        );
        
        Ok(())
    }
    
    // 验证系统状态一致性
    pub fn validate_state() -> StateValidationResult {
        let mut result = StateValidationResult {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            total_btc_locked: 0,
            total_bollar_supply: 0,
            positions_count: 0,
            pools_count: 0,
        };
        
        let pools = crate::get_pools();
        let positions = crate::get_positions();
        
        result.pools_count = pools.len() as u64;
        result.positions_count = positions.len() as u64;
        
        // 验证池状态
        for pool in &pools {
            if pool.states.is_empty() {
                result.warnings.push(format!("Pool {} has no states", pool.addr));
            }
            
            if pool.collateral_ratio == 0 || pool.collateral_ratio > 100 {
                result.errors.push(format!("Pool {} has invalid collateral ratio: {}", 
                                         pool.addr, pool.collateral_ratio));
                result.valid = false;
            }
            
            if pool.liquidation_threshold <= pool.collateral_ratio {
                result.errors.push(format!("Pool {} liquidation threshold ({}) must be higher than collateral ratio ({})", 
                                         pool.addr, pool.liquidation_threshold, pool.collateral_ratio));
                result.valid = false;
            }
        }
        
        // 验证头寸状态
        for position in &positions {
            result.total_btc_locked += position.btc_collateral;
            result.total_bollar_supply += position.bollar_debt;
            
            if position.btc_collateral == 0 {
                result.errors.push(format!("Position {} has zero collateral", position.id));
                result.valid = false;
            }
            
            if position.bollar_debt == 0 {
                result.warnings.push(format!("Position {} has zero debt", position.id));
            }
            
            if position.health_factor == 0 {
                result.errors.push(format!("Position {} has zero health factor", position.id));
                result.valid = false;
            }
            
            // 验证头寸所属的池是否存在
            let pool_addr = position.id.split(':').next().unwrap_or("");
            if !pools.iter().any(|p| p.addr == pool_addr) {
                result.errors.push(format!("Position {} references non-existent pool {}", 
                                         position.id, pool_addr));
                result.valid = false;
            }
        }
        
        // 验证数据一致性
        let calculated_metrics = crate::lending::get_protocol_metrics();
        if calculated_metrics.total_btc_locked != result.total_btc_locked {
            result.warnings.push("BTC locked amount mismatch in metrics".to_string());
        }
        
        if calculated_metrics.total_bollar_supply != result.total_bollar_supply {
            result.warnings.push("Bollar supply amount mismatch in metrics".to_string());
        }
        
        result
    }
}

#[query]
// 获取状态快照列表
pub fn get_state_snapshots(limit: Option<usize>) -> Vec<StateSnapshot> {
    // 检查权限
    let caller = ic_api::caller();
    if !crate::access_control::has_permission(caller, crate::access_control::Permission::ViewAllPositions) {
        return vec![];
    }
    
    STATE_SNAPSHOTS.with_borrow(|snapshots| {
        let limit = limit.unwrap_or(10).min(100);
        snapshots.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    })
}

#[query]
// 获取状态事务日志
pub fn get_state_transactions(limit: Option<usize>) -> Vec<StateTransaction> {
    // 检查权限
    let caller = ic_api::caller();
    if !crate::access_control::has_permission(caller, crate::access_control::Permission::ViewAllPositions) {
        return vec![];
    }
    
    STATE_TRANSACTIONS.with_borrow(|transactions| {
        let limit = limit.unwrap_or(10).min(100);
        transactions.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    })
}

#[update]
// 手动创建状态快照
pub fn create_manual_snapshot(reason: String) -> Result<String> {
    // 检查权限
    let caller = ic_api::caller();
    crate::access_control::check_permission(caller, crate::access_control::Permission::SystemAdmin)?;
    
    validate_param!(!reason.trim().is_empty(), "Reason cannot be empty");
    
    let mut metadata = HashMap::new();
    metadata.insert("reason".to_string(), reason);
    metadata.insert("created_by".to_string(), caller.to_string());
    
    StateManager::create_snapshot(StateOperation::Emergency, Some(metadata))
}

#[update]
// 验证系统状态
pub fn validate_system_state() -> Result<StateValidationResult> {
    // 检查权限
    let caller = ic_api::caller();
    crate::access_control::check_permission(caller, crate::access_control::Permission::ViewMetrics)?;
    
    let result = StateManager::validate_state();
    
    secure_log_info!(
        LogCategory::Audit,
        format!("State validation completed: valid={}", result.valid),
        format!("Errors: {}, Warnings: {}", result.errors.len(), result.warnings.len())
    );
    
    Ok(result)
}

#[update]
// 更新自动快照配置
pub fn update_auto_snapshot_config(config: AutoSnapshotConfig) -> Result<()> {
    // 检查权限
    let caller = ic_api::caller();
    crate::access_control::check_permission(caller, crate::access_control::Permission::SystemAdmin)?;
    
    // 验证配置
    validate_param!(config.interval_minutes > 0 && config.interval_minutes <= 1440, 
                   "Interval must be between 1 and 1440 minutes");
    validate_param!(config.max_snapshots > 0 && config.max_snapshots <= 1000, 
                   "Max snapshots must be between 1 and 1000");
    
    AUTO_SNAPSHOT_CONFIG.with_borrow_mut(|current_config| {
        *current_config = config.clone();
    });
    
    secure_log_info!(
        LogCategory::System,
        "Auto snapshot configuration updated".to_string(),
        format!("Updated by: {}", caller)
    );
    
    Ok(())
}

// 自动快照任务
pub fn auto_snapshot_task() {
    let config = AUTO_SNAPSHOT_CONFIG.with_borrow(|config| config.clone());
    
    if !config.enabled {
        return;
    }
    
    // 检查是否需要创建快照
    let should_create = STATE_SNAPSHOTS.with_borrow(|snapshots| {
        if snapshots.is_empty() {
            return true;
        }
        
        let last_snapshot = snapshots.back().unwrap();
        let elapsed_minutes = (ic_api::time() - last_snapshot.timestamp) / (60 * 1_000_000_000);
        
        elapsed_minutes >= config.interval_minutes
    });
    
    if should_create {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "auto".to_string());
        
        if let Err(e) = StateManager::create_snapshot(StateOperation::Emergency, Some(metadata)) {
            secure_log_error!(
                LogCategory::System,
                format!("Auto snapshot creation failed: {:?}", e)
            );
        }
    }
}

// 心跳函数中的状态管理任务
#[ic_cdk_macros::heartbeat]
async fn state_manager_heartbeat() {
    auto_snapshot_task();
}