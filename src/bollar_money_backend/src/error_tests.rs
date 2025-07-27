#[cfg(test)]
mod error_tests {
    use crate::error::{Error, LogLevel, from_str, from_error, IntoError};
    use std::io::{Error as IoError, ErrorKind};
    
    // 测试错误创建
    #[test]
    fn test_error_creation() {
        // 测试基本错误类型
        let error = Error::InvalidArgument("测试参数".to_string());
        assert!(error.to_string().contains("参数错误"));
        
        // 测试从字符串创建错误
        let error = from_str("测试错误");
        assert!(error.to_string().contains("未知错误"));
        assert!(error.to_string().contains("测试错误"));
        
        // 测试不同错误类型
        let errors = vec![
            Error::Overflow,
            Error::InvalidPool,
            Error::InsufficientFunds,
            Error::InvalidTxid,
            Error::EmptyPool,
            Error::InvalidState("测试状态".to_string()),
            Error::InvalidSignatureArgs("测试签名".to_string()),
            Error::PoolStateExpired(42),
            Error::PositionNotFound,
            Error::PositionNotLiquidatable,
            Error::OracleError("测试 Oracle".to_string()),
            Error::AuthenticationFailed,
            Error::SystemError("系统错误".to_string()),
            Error::PermissionDenied("权限错误".to_string()),
            Error::Unknown("未知错误".to_string()),
        ];
        
        // 验证每种错误类型都有唯一的错误消息
        let mut error_messages = std::collections::HashSet::new();
        for error in errors {
            let message = error.to_string();
            assert!(!message.is_empty());
            error_messages.insert(message);
        }
        
        // 确保所有错误消息都是唯一的
        assert_eq!(error_messages.len(), 15);
    }
    
    // 测试错误处理和转换
    #[test]
    fn test_error_handling() {
        // 测试错误创建和转换
        let error = Error::InvalidPool;
        assert_eq!(error.to_string(), "无效的资金池");
        
        // 测试从 std::io::Error 转换
        let io_errors = vec![
            IoError::new(ErrorKind::NotFound, "文件未找到"),
            IoError::new(ErrorKind::PermissionDenied, "权限被拒绝"),
            IoError::new(ErrorKind::ConnectionRefused, "连接被拒绝"),
            IoError::new(ErrorKind::ConnectionReset, "连接重置"),
            IoError::new(ErrorKind::ConnectionAborted, "连接中止"),
            IoError::new(ErrorKind::NotConnected, "未连接"),
            IoError::new(ErrorKind::AddrInUse, "地址已使用"),
            IoError::new(ErrorKind::AddrNotAvailable, "地址不可用"),
            IoError::new(ErrorKind::BrokenPipe, "管道已断开"),
            IoError::new(ErrorKind::AlreadyExists, "已存在"),
            IoError::new(ErrorKind::WouldBlock, "操作将阻塞"),
            IoError::new(ErrorKind::InvalidInput, "无效输入"),
            IoError::new(ErrorKind::InvalidData, "无效数据"),
            IoError::new(ErrorKind::TimedOut, "操作超时"),
            IoError::new(ErrorKind::WriteZero, "写入零字节"),
            IoError::new(ErrorKind::Interrupted, "操作被中断"),
            IoError::new(ErrorKind::Unsupported, "不支持的操作"),
            IoError::new(ErrorKind::UnexpectedEof, "意外的 EOF"),
            IoError::new(ErrorKind::OutOfMemory, "内存不足"),
            IoError::new(ErrorKind::Other, "其他错误"),
        ];
        
        for io_error in io_errors {
            let error_message = io_error.to_string();
            let error = from_error(io_error);
            assert!(error.to_string().contains("系统错误"));
            assert!(error.to_string().contains(&error_message));
        }
    }
    
    // 测试错误转换特性
    #[test]
    fn test_error_conversion() {
        // 创建一个自定义结果类型
        struct CustomResult;
        
        // 实现 IntoError 特性
        impl<T> IntoError<T> for Result<T, CustomResult> {
            fn into_error(self, context: &str) -> crate::error::Result<T> {
                match self {
                    Ok(val) => Ok(val),
                    Err(_) => Err(Error::SystemError(context.to_string())),
                }
            }
        }
        
        // 测试成功情况
        let result: Result<i32, CustomResult> = Ok(42);
        let converted = result.into_error("测试上下文");
        assert_eq!(converted.unwrap(), 42);
        
        // 测试失败情况
        let result: Result<i32, CustomResult> = Err(CustomResult);
        let converted = result.into_error("测试上下文");
        assert!(converted.is_err());
        let err = converted.unwrap_err();
        assert!(matches!(err, Error::SystemError(_)));
        assert!(err.to_string().contains("测试上下文"));
    }
    
    // 测试错误类型的克隆和比较
    #[test]
    fn test_error_clone_and_eq() {
        let error1 = Error::InvalidPool;
        let error2 = error1.clone();
        
        assert_eq!(error1, error2);
        assert_eq!(error1.to_string(), error2.to_string());
        
        let error3 = Error::InvalidState("测试".to_string());
        let error4 = Error::InvalidState("测试".to_string());
        let error5 = Error::InvalidState("不同".to_string());
        
        assert_eq!(error3, error4);
        assert_ne!(error3, error5);
        assert_ne!(error1, error3);
    }
    
    // 测试复杂错误场景
    #[test]
    fn test_complex_error_scenarios() {
        // 模拟一个简单的错误处理流程
        let result = (|| {
            // 创建一个直接的错误
            Err::<i32, _>(Error::SystemError("系统错误示例".to_string()))
        })();
        
        // 验证结果
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, Error::SystemError(_)));
        assert!(error.to_string().contains("系统错误"));
    }
    
    // 测试错误日志格式
    #[test]
    fn test_error_log_format() {
        // 创建一个模拟的日志函数，用于测试日志格式
        fn mock_log(level: LogLevel, error: &Error, context: Option<&str>) -> String {
            let level_str = match level {
                LogLevel::Debug => "DEBUG",
                LogLevel::Info => "INFO",
                LogLevel::Warning => "WARNING",
                LogLevel::Error => "ERROR",
            };
            
            let context_str = context.unwrap_or("");
            let timestamp = 1234567890; // 模拟时间戳
            
            format!(
                "[{}] [{}] Error: {} Context: {}",
                level_str, timestamp, error, context_str
            )
        }
        
        // 测试不同日志级别
        let error = Error::InvalidPool;
        let debug_log = mock_log(LogLevel::Debug, &error, Some("调试上下文"));
        let info_log = mock_log(LogLevel::Info, &error, Some("信息上下文"));
        let warning_log = mock_log(LogLevel::Warning, &error, Some("警告上下文"));
        let error_log = mock_log(LogLevel::Error, &error, Some("错误上下文"));
        
        // 验证日志格式
        assert!(debug_log.contains("DEBUG"));
        assert!(debug_log.contains("调试上下文"));
        assert!(info_log.contains("INFO"));
        assert!(info_log.contains("信息上下文"));
        assert!(warning_log.contains("WARNING"));
        assert!(warning_log.contains("警告上下文"));
        assert!(error_log.contains("ERROR"));
        assert!(error_log.contains("错误上下文"));
        
        // 测试没有上下文的情况
        let no_context_log = mock_log(LogLevel::Error, &error, None);
        assert!(no_context_log.contains("ERROR"));
        assert!(no_context_log.contains("Context: "));
    }
}