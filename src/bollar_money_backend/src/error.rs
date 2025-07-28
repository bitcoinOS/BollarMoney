use candid::{CandidType, Deserialize};
use serde::Serialize;
use thiserror::Error;

/// 系统错误类型
#[derive(Debug, Error, CandidType, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum Error {
    #[error("数学运算溢出")]
    Overflow,
    
    #[error("无效的资金池")]
    InvalidPool,
    
    #[error("资金不足")]
    InsufficientFunds,
    
    #[error("无效的交易ID")]
    InvalidTxid,
    
    #[error("资金池未初始化或已被移除")]
    EmptyPool,
    
    #[error("无效的资金池状态: {0}")]
    InvalidState(String),
    
    #[error("无效的签名参数: {0}")]
    InvalidSignatureArgs(String),
    
    #[error("资金池状态已过期，当前 = {0}")]
    PoolStateExpired(u64),
    
    #[error("头寸不存在")]
    PositionNotFound,
    
    #[error("头寸不可清算")]
    PositionNotLiquidatable,
    
    #[error("Oracle 错误: {0}")]
    OracleError(String),
    
    #[error("认证失败")]
    AuthenticationFailed,
    
    #[error("参数错误: {0}")]
    InvalidArgument(String),
    
    #[error("权限错误: {0}")]
    PermissionDenied(String),
    
    #[error("系统错误: {0}")]
    SystemError(String),
    
    #[error("未知错误: {0}")]
    Unknown(String),
}

/// 错误日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

/// 错误日志记录
pub fn log_error(level: LogLevel, error: &Error, context: Option<&str>) {
    let level_str = match level {
        LogLevel::Debug => "DEBUG",
        LogLevel::Info => "INFO",
        LogLevel::Warning => "WARNING",
        LogLevel::Error => "ERROR",
    };
    
    let context_str = context.unwrap_or("");
    
    let timestamp = crate::ic_api::time();
    
    let log_message = format!(
        "[{}] [{}] Error: {} Context: {}",
        level_str, timestamp, error, context_str
    );
    
    // 在 IC 环境中记录日志
    ic_cdk::println!("{}", log_message);
}

/// 从字符串创建错误
#[allow(dead_code)]
pub fn from_string(message: String) -> Error {
    Error::Unknown(message)
}

/// 从 &str 创建错误
#[allow(dead_code)]
pub fn from_str(message: &str) -> Error {
    Error::Unknown(message.to_string())
}

/// 从其他错误类型转换
pub fn from_error<E: std::error::Error>(err: E) -> Error {
    Error::SystemError(err.to_string())
}

/// 结果类型别名
pub type Result<T> = std::result::Result<T, Error>;

/// 错误处理宏
#[macro_export]
macro_rules! try_log {
    ($expr:expr, $level:expr, $context:expr) => {
        match $expr {
            Ok(val) => val,
            Err(err) => {
                $crate::error::log_error($level, &$crate::error::from_error(err), Some($context));
                return Err($crate::error::from_error(err));
            }
        }
    };
}

/// 统一的操作结果处理宏
#[macro_export]
macro_rules! handle_operation {
    ($operation:expr, $context:expr) => {
        $crate::error::catch_and_log(
            || $operation,
            $crate::LogLevel::Error,
            $context
        )
    };
    ($operation:expr, $context:expr, $level:expr) => {
        $crate::error::catch_and_log(
            || $operation,
            $level,
            $context
        )
    };
}

/// 安全的数值转换宏
#[macro_export]
macro_rules! safe_cast {
    ($value:expr, $target_type:ty) => {
        $value.try_into()
            .map_err(|_| $crate::Error::Overflow)
    };
}

/// 参数验证宏
#[macro_export]
macro_rules! validate_param {
    ($condition:expr, $message:expr) => {
        if !($condition) {
            return Err($crate::Error::InvalidArgument($message.to_string()));
        }
    };
}

/// 权限检查宏
#[macro_export]
macro_rules! require_permission {
    ($check:expr, $message:expr) => {
        if !($check) {
            return Err($crate::Error::PermissionDenied($message.to_string()));
        }
    };
}

/// 统一的操作结果处理宏
#[macro_export]
macro_rules! handle_operation {
    ($operation:expr, $context:expr) => {
        $crate::error::catch_and_log(
            || $operation,
            $crate::LogLevel::Error,
            $context
        )
    };
    ($operation:expr, $context:expr, $level:expr) => {
        $crate::error::catch_and_log(
            || $operation,
            $level,
            $context
        )
    };
}

/// 输入验证宏
#[macro_export]
macro_rules! validate_input {
    ($condition:expr, $error_msg:expr) => {
        if !($condition) {
            return Err($crate::Error::InvalidArgument($error_msg.to_string()));
        }
    };
}

/// 权限检查宏
#[macro_export]
macro_rules! require_permission {
    ($condition:expr, $error_msg:expr) => {
        if !($condition) {
            return Err($crate::Error::PermissionDenied($error_msg.to_string()));
        }
    };
}

/// 安全日志宏（过滤敏感信息）
#[macro_export]
macro_rules! safe_log {
    ($level:expr, $message:expr) => {
        ic_cdk::println!("[{}] {}", 
            match $level {
                $crate::LogLevel::Debug => "DEBUG",
                $crate::LogLevel::Info => "INFO", 
                $crate::LogLevel::Warning => "WARN",
                $crate::LogLevel::Error => "ERROR",
            },
            $crate::error::sanitize_log_message($message)
        );
    };
    ($level:expr, $format:expr, $($args:expr),+) => {
        ic_cdk::println!("[{}] {}", 
            match $level {
                $crate::LogLevel::Debug => "DEBUG",
                $crate::LogLevel::Info => "INFO",
                $crate::LogLevel::Warning => "WARN", 
                $crate::LogLevel::Error => "ERROR",
            },
            $crate::error::sanitize_log_message(&format!($format, $($args),+))
        );
    };
}

/// 捕获并记录错误
pub fn catch_and_log<F, T>(f: F, level: LogLevel, context: &str) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    match f() {
        Ok(val) => Ok(val),
        Err(err) => {
            log_error(level, &err, Some(context));
            Err(err)
        }
    }
}

/// 错误转换特性
#[allow(dead_code)]
pub trait IntoError<T> {
    fn into_error(self, context: &str) -> Result<T>;
}

impl<T, E: std::error::Error> IntoError<T> for std::result::Result<T, E> {
    fn into_error(self, context: &str) -> Result<T> {
        match self {
            Ok(val) => Ok(val),
            Err(err) => {
                let error = from_error(err);
                log_error(LogLevel::Error, &error, Some(context));
                Err(error)
            }
        }
    }
}

/// 清理日志消息，移除敏感信息
pub fn sanitize_log_message(message: &str) -> String {
    let mut sanitized = message.to_string();
    
    // 移除可能的私钥、签名等敏感信息
    let sensitive_patterns = [
        (r"[0-9a-fA-F]{64}", "***PRIVATE_KEY***"),
        (r"[0-9a-fA-F]{128}", "***SIGNATURE***"),
        (r"bc1[a-zA-Z0-9]{39,59}", "***BTC_ADDRESS***"),
        (r"[13][a-km-zA-HJ-NP-Z1-9]{25,34}", "***BTC_ADDRESS***"),
        (r"0x[a-fA-F0-9]{40}", "***ETH_ADDRESS***"),
    ];
    
    for (pattern, replacement) in &sensitive_patterns {
        if let Ok(regex) = regex::Regex::new(pattern) {
            sanitized = regex.replace_all(&sanitized, *replacement).to_string();
        }
    }
    
    sanitized
}

/// 操作审计日志
pub fn audit_log(operation: &str, user: &str, details: &str) {
    let timestamp = crate::ic_api::time();
    let sanitized_details = sanitize_log_message(details);
    
    ic_cdk::println!("[AUDIT] {} | User: {} | Operation: {} | Details: {}", 
                     timestamp, user, operation, sanitized_details);
}