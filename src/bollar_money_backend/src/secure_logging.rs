// secure_logging.rs - 安全日志记录
// 这个模块实现安全的日志记录，防止敏感信息泄露

use crate::{Error, LogLevel, Result, ic_api};
use candid::{CandidType, Deserialize, Principal};
use ic_cdk_macros::{query, update};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::VecDeque;
use sha2::{Sha256, Digest};

// 日志级别
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecureLogLevel {
    Debug = 0,
    Info = 1,
    Warning = 2,
    Error = 3,
    Critical = 4,
}

// 日志类别
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum LogCategory {
    Authentication,
    Transaction,
    Liquidation,
    Emergency,
    Security,
    System,
    Audit,
}

// 安全日志条目
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SecureLogEntry {
    pub id: String,
    pub timestamp: u64,
    pub level: SecureLogLevel,
    pub category: LogCategory,
    pub principal: Option<Principal>,
    pub message: String,
    pub context: Option<String>,
    pub sensitive_data_hash: Option<String>, // 敏感数据的哈希值
}

impl SecureLogEntry {
    // 创建新的日志条目
    pub fn new(
        level: SecureLogLevel,
        category: LogCategory,
        message: String,
        context: Option<String>,
        sensitive_data: Option<&str>,
    ) -> Self {
        let timestamp = ic_api::time();
        let principal = Some(ic_api::caller());
        
        // 生成唯一ID
        let id = format!("{}:{}", timestamp, generate_log_id(&message, timestamp));
        
        // 对敏感数据进行哈希处理
        let sensitive_data_hash = sensitive_data.map(|data| {
            let mut hasher = Sha256::new();
            hasher.update(data.as_bytes());
            hex::encode(hasher.finalize())
        });
        
        Self {
            id,
            timestamp,
            level,
            category,
            principal,
            message: sanitize_message(message),
            context: context.map(sanitize_message),
            sensitive_data_hash,
        }
    }
}

// 日志配置
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct LogConfig {
    pub min_level: SecureLogLevel,
    pub max_entries: usize,
    pub enable_debug: bool,
    pub enable_sensitive_logging: bool,
    pub retention_hours: u64,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            min_level: SecureLogLevel::Info,
            max_entries: 10000,
            enable_debug: false,
            enable_sensitive_logging: false,
            retention_hours: 168, // 7 days
        }
    }
}

thread_local! {
    // 日志存储
    static LOG_ENTRIES: RefCell<VecDeque<SecureLogEntry>> = RefCell::new(VecDeque::new());
    
    // 日志配置
    static LOG_CONFIG: RefCell<LogConfig> = RefCell::new(LogConfig::default());
    
    // 敏感词列表
    static SENSITIVE_PATTERNS: RefCell<Vec<String>> = RefCell::new(vec![
        "password".to_string(),
        "private_key".to_string(),
        "secret".to_string(),
        "token".to_string(),
        "signature".to_string(),
        "seed".to_string(),
        "mnemonic".to_string(),
    ]);
}

// 安全日志记录函数
pub fn secure_log(
    level: SecureLogLevel,
    category: LogCategory,
    message: String,
    context: Option<String>,
    sensitive_data: Option<&str>,
) {
    // 检查日志级别
    let min_level = LOG_CONFIG.with_borrow(|config| config.min_level.clone());
    if level < min_level {
        return;
    }
    
    // 创建日志条目
    let entry = SecureLogEntry::new(level.clone(), category, message, context, sensitive_data);
    
    // 存储日志
    LOG_ENTRIES.with_borrow_mut(|entries| {
        entries.push_back(entry.clone());
        
        // 维护日志大小限制
        let max_entries = LOG_CONFIG.with_borrow(|config| config.max_entries);
        while entries.len() > max_entries {
            entries.pop_front();
        }
    });
    
    // 输出到系统日志（仅非敏感信息）
    let safe_message = if entry.sensitive_data_hash.is_some() {
        format!("{} [SENSITIVE_DATA_HASH: {}]", 
                entry.message, 
                entry.sensitive_data_hash.as_ref().unwrap())
    } else {
        entry.message.clone()
    };
    
    ic_cdk::println!("[{:?}][{:?}] {}", level, category, safe_message);
}

#[query]
// 获取日志条目
pub fn get_logs(
    level_filter: Option<SecureLogLevel>,
    category_filter: Option<LogCategory>,
    limit: Option<usize>,
) -> Vec<SecureLogEntry> {
    // 检查权限
    let caller = ic_api::caller();
    if !crate::access_control::has_permission(caller, crate::access_control::Permission::ViewAllPositions) {
        return vec![];
    }
    
    LOG_ENTRIES.with_borrow(|entries| {
        let limit = limit.unwrap_or(100).min(1000);
        
        entries.iter()
            .rev()
            .filter(|entry| {
                if let Some(ref level) = level_filter {
                    if entry.level < *level {
                        return false;
                    }
                }
                if let Some(ref category) = category_filter {
                    if entry.category != *category {
                        return false;
                    }
                }
                true
            })
            .take(limit)
            .cloned()
            .collect()
    })
}

#[update]
// 更新日志配置
pub fn update_log_config(config: LogConfig) -> Result<()> {
    // 检查权限
    let caller = ic_api::caller();
    crate::access_control::check_permission(caller, crate::access_control::Permission::SystemAdmin)?;
    
    // 验证配置
    validate_param!(config.max_entries > 0 && config.max_entries <= 50000, 
                   "Max entries must be between 1 and 50000");
    validate_param!(config.retention_hours > 0 && config.retention_hours <= 8760, 
                   "Retention hours must be between 1 and 8760 (1 year)");
    
    LOG_CONFIG.with_borrow_mut(|current_config| {
        *current_config = config.clone();
    });
    
    secure_log(
        SecureLogLevel::Info,
        LogCategory::System,
        "Log configuration updated".to_string(),
        Some(format!("Updated by: {}", caller)),
        None,
    );
    
    Ok(())
}

#[query]
// 获取日志配置
pub fn get_log_config() -> Result<LogConfig> {
    // 检查权限
    let caller = ic_api::caller();
    crate::access_control::check_permission(caller, crate::access_control::Permission::SystemAdmin)?;
    
    Ok(LOG_CONFIG.with_borrow(|config| config.clone()))
}

#[update]
// 清理过期日志
pub fn cleanup_expired_logs() -> u64 {
    let current_time = ic_api::time();
    let retention_ns = LOG_CONFIG.with_borrow(|config| config.retention_hours * 60 * 60 * 1_000_000_000);
    
    let removed_count = LOG_ENTRIES.with_borrow_mut(|entries| {
        let initial_len = entries.len();
        entries.retain(|entry| current_time - entry.timestamp <= retention_ns);
        initial_len - entries.len()
    });
    
    if removed_count > 0 {
        secure_log(
            SecureLogLevel::Info,
            LogCategory::System,
            format!("Cleaned up {} expired log entries", removed_count),
            None,
            None,
        );
    }
    
    removed_count as u64
}

#[update]
// 添加敏感词模式
pub fn add_sensitive_pattern(pattern: String) -> Result<()> {
    // 检查权限
    let caller = ic_api::caller();
    crate::access_control::check_permission(caller, crate::access_control::Permission::SystemAdmin)?;
    
    validate_param!(!pattern.trim().is_empty(), "Pattern cannot be empty");
    validate_param!(pattern.len() <= 100, "Pattern too long");
    
    SENSITIVE_PATTERNS.with_borrow_mut(|patterns| {
        if !patterns.contains(&pattern) {
            patterns.push(pattern.clone());
        }
    });
    
    secure_log(
        SecureLogLevel::Info,
        LogCategory::Security,
        "Sensitive pattern added".to_string(),
        Some(format!("Added by: {}", caller)),
        Some(&pattern),
    );
    
    Ok(())
}

#[query]
// 获取日志统计
pub fn get_log_statistics() -> Result<LogStatistics> {
    // 检查权限
    let caller = ic_api::caller();
    crate::access_control::check_permission(caller, crate::access_control::Permission::ViewMetrics)?;
    
    let stats = LOG_ENTRIES.with_borrow(|entries| {
        let mut stats = LogStatistics::default();
        stats.total_entries = entries.len() as u64;
        
        for entry in entries.iter() {
            match entry.level {
                SecureLogLevel::Debug => stats.debug_count += 1,
                SecureLogLevel::Info => stats.info_count += 1,
                SecureLogLevel::Warning => stats.warning_count += 1,
                SecureLogLevel::Error => stats.error_count += 1,
                SecureLogLevel::Critical => stats.critical_count += 1,
            }
            
            match entry.category {
                LogCategory::Authentication => stats.auth_count += 1,
                LogCategory::Transaction => stats.transaction_count += 1,
                LogCategory::Liquidation => stats.liquidation_count += 1,
                LogCategory::Emergency => stats.emergency_count += 1,
                LogCategory::Security => stats.security_count += 1,
                LogCategory::System => stats.system_count += 1,
                LogCategory::Audit => stats.audit_count += 1,
            }
        }
        
        stats
    });
    
    Ok(stats)
}

// 日志统计结构
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, Default)]
pub struct LogStatistics {
    pub total_entries: u64,
    pub debug_count: u64,
    pub info_count: u64,
    pub warning_count: u64,
    pub error_count: u64,
    pub critical_count: u64,
    pub auth_count: u64,
    pub transaction_count: u64,
    pub liquidation_count: u64,
    pub emergency_count: u64,
    pub security_count: u64,
    pub system_count: u64,
    pub audit_count: u64,
}

// 消息清理函数
fn sanitize_message(message: String) -> String {
    let mut sanitized = message;
    
    SENSITIVE_PATTERNS.with_borrow(|patterns| {
        for pattern in patterns.iter() {
            // 使用正则表达式替换敏感信息
            let re_pattern = format!(r"(?i){}\s*[:=]\s*\S+", regex::escape(pattern));
            if let Ok(re) = regex::Regex::new(&re_pattern) {
                sanitized = re.replace_all(&sanitized, &format!("{}:[REDACTED]", pattern)).to_string();
            }
        }
    });
    
    // 移除可能的私钥、地址等敏感信息
    sanitized = regex::Regex::new(r"\b[13][a-km-zA-HJ-NP-Z1-9]{25,34}\b")
        .unwrap()
        .replace_all(&sanitized, "[BTC_ADDRESS]")
        .to_string();
    
    sanitized = regex::Regex::new(r"\b[0-9a-fA-F]{64}\b")
        .unwrap()
        .replace_all(&sanitized, "[HASH]")
        .to_string();
    
    sanitized
}

// 生成日志ID
fn generate_log_id(message: &str, timestamp: u64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(message.as_bytes());
    hasher.update(timestamp.to_be_bytes());
    hex::encode(hasher.finalize())[0..8].to_string()
}

// 便捷的日志记录宏
#[macro_export]
macro_rules! secure_log_debug {
    ($category:expr, $message:expr) => {
        $crate::secure_logging::secure_log(
            $crate::secure_logging::SecureLogLevel::Debug,
            $category,
            $message.to_string(),
            None,
            None,
        )
    };
    ($category:expr, $message:expr, $context:expr) => {
        $crate::secure_logging::secure_log(
            $crate::secure_logging::SecureLogLevel::Debug,
            $category,
            $message.to_string(),
            Some($context.to_string()),
            None,
        )
    };
}

#[macro_export]
macro_rules! secure_log_info {
    ($category:expr, $message:expr) => {
        $crate::secure_logging::secure_log(
            $crate::secure_logging::SecureLogLevel::Info,
            $category,
            $message.to_string(),
            None,
            None,
        )
    };
    ($category:expr, $message:expr, $context:expr) => {
        $crate::secure_logging::secure_log(
            $crate::secure_logging::SecureLogLevel::Info,
            $category,
            $message.to_string(),
            Some($context.to_string()),
            None,
        )
    };
}

#[macro_export]
macro_rules! secure_log_warning {
    ($category:expr, $message:expr) => {
        $crate::secure_logging::secure_log(
            $crate::secure_logging::SecureLogLevel::Warning,
            $category,
            $message.to_string(),
            None,
            None,
        )
    };
    ($category:expr, $message:expr, $context:expr) => {
        $crate::secure_logging::secure_log(
            $crate::secure_logging::SecureLogLevel::Warning,
            $category,
            $message.to_string(),
            Some($context.to_string()),
            None,
        )
    };
}

#[macro_export]
macro_rules! secure_log_error {
    ($category:expr, $message:expr) => {
        $crate::secure_logging::secure_log(
            $crate::secure_logging::SecureLogLevel::Error,
            $category,
            $message.to_string(),
            None,
            None,
        )
    };
    ($category:expr, $message:expr, $context:expr) => {
        $crate::secure_logging::secure_log(
            $crate::secure_logging::SecureLogLevel::Error,
            $category,
            $message.to_string(),
            Some($context.to_string()),
            None,
        )
    };
}

#[macro_export]
macro_rules! secure_log_critical {
    ($category:expr, $message:expr) => {
        $crate::secure_logging::secure_log(
            $crate::secure_logging::SecureLogLevel::Critical,
            $category,
            $message.to_string(),
            None,
            None,
        )
    };
    ($category:expr, $message:expr, $context:expr) => {
        $crate::secure_logging::secure_log(
            $crate::secure_logging::SecureLogLevel::Critical,
            $category,
            $message.to_string(),
            Some($context.to_string()),
            None,
        )
    };
}

// 心跳函数中的清理任务
#[ic_cdk_macros::heartbeat]
async fn secure_logging_heartbeat() {
    cleanup_expired_logs();
}