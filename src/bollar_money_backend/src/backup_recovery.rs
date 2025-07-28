// backup_recovery.rs - 数据备份和恢复
// 这个模块实现数据备份、恢复和灾难恢复功能

use crate::{Error, Result, types::*, secure_logging::*, ic_api};
use candid::{CandidType, Deserialize};
use ic_cdk_macros::{query, update};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;

// 备份类型
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum BackupType {
    Full,        // 完整备份
    Incremental, // 增量备份
    Emergency,   // 紧急备份
}

// 备份状态
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum BackupStatus {
    InProgress,
    Completed,
    Failed,
    Corrupted,
}

// 备份元数据
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct BackupMetadata {
    pub id: String,
    pub backup_type: BackupType,
    pub status: BackupStatus,
    pub created_at: u64,
    pub completed_at: Option<u64>,
    pub size_bytes: u64,
    pub checksum: String,
    pub description: String,
    pub created_by: String,
}

// 完整备份数据
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct FullBackupData {
    pub metadata: BackupMetadata,
    pub pools: HashMap<String, Pool>,
    pub positions: HashMap<String, Position>,
    pub system_config: SystemConfig,
    pub version: String,
}

// 增量备份数据
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct IncrementalBackupData {
    pub metadata: BackupMetadata,
    pub base_backup_id: String,
    pub changes: Vec<DataChange>,
}

// 数据变更记录
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DataChange {
    pub timestamp: u64,
    pub change_type: ChangeType,
    pub entity_type: EntityType,
    pub entity_id: String,
    pub old_data: Option<String>, // JSON 序列化的数据
    pub new_data: Option<String>, // JSON 序列化的数据
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ChangeType {
    Create,
    Update,
    Delete,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum EntityType {
    Pool,
    Position,
    Config,
}

// 系统配置
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SystemConfig {
    pub emergency_state: crate::emergency::EmergencyControls,
    pub performance_config: crate::performance::PerformanceConfig,
    pub log_config: crate::secure_logging::LogConfig,
}

// 恢复选项
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct RecoveryOptions {
    pub backup_id: String,
    pub restore_pools: bool,
    pub restore_positions: bool,
    pub restore_config: bool,
    pub verify_integrity: bool,
    pub create_recovery_point: bool,
}

// 恢复结果
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct RecoveryResult {
    pub success: bool,
    pub restored_pools: u64,
    pub restored_positions: u64,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub recovery_time_ms: u64,
}

thread_local! {
    // 备份存储
    static BACKUPS: RefCell<HashMap<String, FullBackupData>> = RefCell::new(HashMap::new());
    
    // 增量备份存储
    static INCREMENTAL_BACKUPS: RefCell<HashMap<String, IncrementalBackupData>> = RefCell::new(HashMap::new());
    
    // 数据变更日志
    static CHANGE_LOG: RefCell<Vec<DataChange>> = RefCell::new(Vec::new());
    
    // 备份配置
    static BACKUP_CONFIG: RefCell<BackupConfig> = RefCell::new(BackupConfig::default());
}

// 备份配置
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct BackupConfig {
    pub auto_backup_enabled: bool,
    pub auto_backup_interval_hours: u64,
    pub max_backups: usize,
    pub max_incremental_backups: usize,
    pub compression_enabled: bool,
    pub encryption_enabled: bool,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            auto_backup_enabled: true,
            auto_backup_interval_hours: 24, // 每天备份
            max_backups: 30,                // 保留30个备份
            max_incremental_backups: 100,   // 保留100个增量备份
            compression_enabled: true,
            encryption_enabled: false,      // 暂不启用加密
        }
    }
}

// 备份管理器
pub struct BackupManager;

impl BackupManager {
    // 创建完整备份
    pub fn create_full_backup(description: String) -> Result<String> {
        let caller = ic_api::caller().to_string();
        let backup_id = format!("full_{}_{}", ic_api::time(), caller);
        
        // 收集所有数据
        let pools: HashMap<String, Pool> = crate::get_pools()
            .into_iter()
            .map(|pool| (pool.addr.clone(), pool))
            .collect();
        
        let positions: HashMap<String, Position> = crate::get_positions()
            .into_iter()
            .map(|pos| (pos.id.clone(), pos))
            .collect();
        
        let system_config = SystemConfig {
            emergency_state: crate::emergency::get_emergency_state(),
            performance_config: crate::performance::PerformanceConfig::default(), // 简化处理
            log_config: crate::secure_logging::LogConfig::default(),
        };
        
        // 计算数据大小和校验和
        let data_size = Self::calculate_backup_size(&pools, &positions);
        let checksum = Self::calculate_checksum(&pools, &positions, &system_config);
        
        // 创建备份元数据
        let metadata = BackupMetadata {
            id: backup_id.clone(),
            backup_type: BackupType::Full,
            status: BackupStatus::InProgress,
            created_at: ic_api::time(),
            completed_at: None,
            size_bytes: data_size,
            checksum,
            description,
            created_by: caller,
        };
        
        // 创建备份数据
        let mut backup_data = FullBackupData {
            metadata,
            pools,
            positions,
            system_config,
            version: env!("CARGO_PKG_VERSION").to_string(),
        };
        
        // 标记为完成
        backup_data.metadata.status = BackupStatus::Completed;
        backup_data.metadata.completed_at = Some(ic_api::time());
        
        // 存储备份
        BACKUPS.with_borrow_mut(|backups| {
            backups.insert(backup_id.clone(), backup_data);
            
            // 维护备份数量限制
            let max_backups = BACKUP_CONFIG.with_borrow(|config| config.max_backups);
            if backups.len() > max_backups {
                // 删除最旧的备份
                let oldest_id = backups.values()
                    .min_by_key(|b| b.metadata.created_at)
                    .map(|b| b.metadata.id.clone());
                
                if let Some(id) = oldest_id {
                    backups.remove(&id);
                }
            }
        });
        
        secure_log_info!(
            LogCategory::System,
            format!("Full backup created: {}", backup_id),
            format!("Size: {} bytes", data_size)
        );
        
        Ok(backup_id)
    }
    
    // 创建增量备份
    pub fn create_incremental_backup(base_backup_id: String, description: String) -> Result<String> {
        let caller = ic_api::caller().to_string();
        let backup_id = format!("inc_{}_{}", ic_api::time(), caller);
        
        // 验证基础备份存在
        let base_exists = BACKUPS.with_borrow(|backups| backups.contains_key(&base_backup_id));
        if !base_exists {
            return Err(Error::InvalidArgument("Base backup not found".to_string()));
        }
        
        // 获取自基础备份以来的变更
        let base_backup = BACKUPS.with_borrow(|backups| {
            backups.get(&base_backup_id).cloned()
        }).unwrap();
        
        let changes = CHANGE_LOG.with_borrow(|log| {
            log.iter()
                .filter(|change| change.timestamp > base_backup.metadata.created_at)
                .cloned()
                .collect()
        });
        
        // 创建增量备份元数据
        let metadata = BackupMetadata {
            id: backup_id.clone(),
            backup_type: BackupType::Incremental,
            status: BackupStatus::Completed,
            created_at: ic_api::time(),
            completed_at: Some(ic_api::time()),
            size_bytes: changes.len() as u64 * 1000, // 粗略估算
            checksum: Self::calculate_changes_checksum(&changes),
            description,
            created_by: caller,
        };
        
        // 创建增量备份数据
        let incremental_backup = IncrementalBackupData {
            metadata,
            base_backup_id,
            changes,
        };
        
        // 存储增量备份
        INCREMENTAL_BACKUPS.with_borrow_mut(|backups| {
            backups.insert(backup_id.clone(), incremental_backup);
            
            // 维护增量备份数量限制
            let max_incremental = BACKUP_CONFIG.with_borrow(|config| config.max_incremental_backups);
            if backups.len() > max_incremental {
                let oldest_id = backups.values()
                    .min_by_key(|b| b.metadata.created_at)
                    .map(|b| b.metadata.id.clone());
                
                if let Some(id) = oldest_id {
                    backups.remove(&id);
                }
            }
        });
        
        secure_log_info!(
            LogCategory::System,
            format!("Incremental backup created: {}", backup_id),
            format!("Base: {}, Changes: {}", base_backup_id, changes.len())
        );
        
        Ok(backup_id)
    }
    
    // 恢复数据
    pub fn restore_from_backup(options: RecoveryOptions) -> Result<RecoveryResult> {
        let start_time = ic_api::time();
        let mut result = RecoveryResult {
            success: false,
            restored_pools: 0,
            restored_positions: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
            recovery_time_ms: 0,
        };
        
        // 创建恢复点
        if options.create_recovery_point {
            match Self::create_full_backup("Pre-recovery backup".to_string()) {
                Ok(backup_id) => {
                    result.warnings.push(format!("Recovery point created: {}", backup_id));
                }
                Err(e) => {
                    result.warnings.push(format!("Failed to create recovery point: {:?}", e));
                }
            }
        }
        
        // 获取备份数据
        let backup_data = BACKUPS.with_borrow(|backups| {
            backups.get(&options.backup_id).cloned()
        });
        
        let Some(backup) = backup_data else {
            result.errors.push("Backup not found".to_string());
            return Ok(result);
        };
        
        // 验证备份完整性
        if options.verify_integrity {
            if let Err(e) = Self::verify_backup_integrity(&backup) {
                result.errors.push(format!("Backup integrity check failed: {:?}", e));
                return Ok(result);
            }
        }
        
        // 恢复池数据
        if options.restore_pools {
            for (addr, pool) in backup.pools {
                match crate::input_validation::validate_pool(&pool) {
                    Ok(_) => {
                        crate::save_pool(pool);
                        result.restored_pools += 1;
                    }
                    Err(e) => {
                        result.errors.push(format!("Failed to restore pool {}: {:?}", addr, e));
                    }
                }
            }
        }
        
        // 恢复头寸数据
        if options.restore_positions {
            for (id, position) in backup.positions {
                match crate::input_validation::validate_position(&position) {
                    Ok(_) => {
                        crate::save_position(position);
                        result.restored_positions += 1;
                    }
                    Err(e) => {
                        result.errors.push(format!("Failed to restore position {}: {:?}", id, e));
                    }
                }
            }
        }
        
        // 恢复系统配置
        if options.restore_config {
            // 这里可以恢复各种系统配置
            result.warnings.push("System config restoration not fully implemented".to_string());
        }
        
        let end_time = ic_api::time();
        result.recovery_time_ms = (end_time - start_time) / 1_000_000;
        result.success = result.errors.is_empty();
        
        secure_log_info!(
            LogCategory::System,
            format!("Data recovery completed: success={}", result.success),
            format!("Pools: {}, Positions: {}, Time: {}ms", 
                   result.restored_pools, result.restored_positions, result.recovery_time_ms)
        );
        
        Ok(result)
    }
    
    // 记录数据变更
    pub fn log_data_change(
        change_type: ChangeType,
        entity_type: EntityType,
        entity_id: String,
        old_data: Option<String>,
        new_data: Option<String>,
    ) {
        let change = DataChange {
            timestamp: ic_api::time(),
            change_type,
            entity_type,
            entity_id,
            old_data,
            new_data,
        };
        
        CHANGE_LOG.with_borrow_mut(|log| {
            log.push(change);
            
            // 维护变更日志大小
            if log.len() > 10000 {
                log.drain(0..1000); // 删除最旧的1000条记录
            }
        });
    }
    
    // 计算备份大小
    fn calculate_backup_size(pools: &HashMap<String, Pool>, positions: &HashMap<String, Position>) -> u64 {
        let pools_size = pools.len() * std::mem::size_of::<Pool>();
        let positions_size = positions.len() * std::mem::size_of::<Position>();
        (pools_size + positions_size) as u64
    }
    
    // 计算校验和
    fn calculate_checksum(
        pools: &HashMap<String, Pool>,
        positions: &HashMap<String, Position>,
        _config: &SystemConfig,
    ) -> String {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        
        // 对池数据进行哈希
        for (addr, pool) in pools {
            hasher.update(addr.as_bytes());
            hasher.update(&pool.collateral_ratio.to_be_bytes());
            hasher.update(&pool.liquidation_threshold.to_be_bytes());
        }
        
        // 对头寸数据进行哈希
        for (id, position) in positions {
            hasher.update(id.as_bytes());
            hasher.update(&position.btc_collateral.to_be_bytes());
            hasher.update(&position.bollar_debt.to_be_bytes());
        }
        
        hex::encode(hasher.finalize())
    }
    
    // 计算变更校验和
    fn calculate_changes_checksum(changes: &[DataChange]) -> String {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        for change in changes {
            hasher.update(&change.timestamp.to_be_bytes());
            hasher.update(change.entity_id.as_bytes());
        }
        
        hex::encode(hasher.finalize())
    }
    
    // 验证备份完整性
    fn verify_backup_integrity(backup: &FullBackupData) -> Result<()> {
        // 重新计算校验和
        let calculated_checksum = Self::calculate_checksum(
            &backup.pools,
            &backup.positions,
            &backup.system_config,
        );
        
        if calculated_checksum != backup.metadata.checksum {
            return Err(Error::InvalidState("Backup checksum mismatch".to_string()));
        }
        
        // 验证数据完整性
        for pool in backup.pools.values() {
            crate::input_validation::validate_pool(pool)?;
        }
        
        for position in backup.positions.values() {
            crate::input_validation::validate_position(position)?;
        }
        
        Ok(())
    }
    
    // 自动备份任务
    pub fn auto_backup_task() {
        let config = BACKUP_CONFIG.with_borrow(|config| config.clone());
        
        if !config.auto_backup_enabled {
            return;
        }
        
        // 检查是否需要创建备份
        let should_backup = BACKUPS.with_borrow(|backups| {
            if backups.is_empty() {
                return true;
            }
            
            let latest_backup = backups.values()
                .max_by_key(|b| b.metadata.created_at)
                .unwrap();
            
            let elapsed_hours = (ic_api::time() - latest_backup.metadata.created_at) / (60 * 60 * 1_000_000_000);
            elapsed_hours >= config.auto_backup_interval_hours
        });
        
        if should_backup {
            match Self::create_full_backup("Automatic backup".to_string()) {
                Ok(backup_id) => {
                    secure_log_info!(
                        LogCategory::System,
                        format!("Automatic backup created: {}", backup_id)
                    );
                }
                Err(e) => {
                    secure_log_error!(
                        LogCategory::System,
                        format!("Automatic backup failed: {:?}", e)
                    );
                }
            }
        }
    }
}

#[query]
// 获取备份列表
pub fn get_backups() -> Vec<BackupMetadata> {
    // 检查权限
    let caller = ic_api::caller();
    if !crate::access_control::has_permission(caller, crate::access_control::Permission::ViewAllPositions) {
        return vec![];
    }
    
    let mut all_backups = Vec::new();
    
    // 获取完整备份
    BACKUPS.with_borrow(|backups| {
        all_backups.extend(backups.values().map(|b| b.metadata.clone()));
    });
    
    // 获取增量备份
    INCREMENTAL_BACKUPS.with_borrow(|backups| {
        all_backups.extend(backups.values().map(|b| b.metadata.clone()));
    });
    
    // 按创建时间排序
    all_backups.sort_by_key(|b| std::cmp::Reverse(b.created_at));
    
    all_backups
}

#[update]
// 创建完整备份
pub fn create_full_backup(description: String) -> Result<String> {
    // 检查权限
    let caller = ic_api::caller();
    crate::access_control::check_permission(caller, crate::access_control::Permission::SystemAdmin)?;
    
    crate::input_validation::validate_string(&description, "description", true)?;
    
    BackupManager::create_full_backup(description)
}

#[update]
// 恢复数据
pub fn restore_from_backup(options: RecoveryOptions) -> Result<RecoveryResult> {
    // 检查权限
    let caller = ic_api::caller();
    crate::access_control::check_permission(caller, crate::access_control::Permission::SuperAdmin)?;
    
    crate::input_validation::validate_string(&options.backup_id, "backup_id", true)?;
    
    BackupManager::restore_from_backup(options)
}

// 心跳函数中的备份任务
#[ic_cdk_macros::heartbeat]
async fn backup_recovery_heartbeat() {
    BackupManager::auto_backup_task();
}