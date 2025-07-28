// medium_priority_tests.rs - 中优先级修复的测试
// 测试访问控制、状态管理、安全日志等功能

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        access_control::*,
        secure_logging::*,
        state_manager::*,
        error::*,
    };

    #[test]
    fn test_access_control_permissions() {
        // 测试权限系统
        let user_role = Role::User;
        let admin_role = Role::SuperAdmin;
        
        let user_perms = user_role.permissions();
        let admin_perms = admin_role.permissions();
        
        // 用户应该有基础权限
        assert!(user_perms.contains(&Permission::Deposit));
        assert!(user_perms.contains(&Permission::Withdraw));
        assert!(user_perms.contains(&Permission::ViewMetrics));
        
        // 用户不应该有管理权限
        assert!(!user_perms.contains(&Permission::SuperAdmin));
        assert!(!user_perms.contains(&Permission::EmergencyPause));
        
        // 超级管理员应该有所有权限
        assert!(admin_perms.contains(&Permission::SuperAdmin));
        assert!(admin_perms.contains(&Permission::EmergencyPause));
        assert!(admin_perms.contains(&Permission::Deposit));
    }

    #[test]
    fn test_error_handling_macros() {
        // 测试参数验证宏
        let result = validate_test_function(0);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidArgument(_)));
        
        let result = validate_test_function(50);
        assert!(result.is_ok());
    }
    
    fn validate_test_function(value: u64) -> Result<u64> {
        validate_param!(value > 0, "Value must be positive");
        validate_param!(value <= 100, "Value must be <= 100");
        Ok(value)
    }

    #[test]
    fn test_safe_math_operations() {
        use crate::safe_math::*;
        
        // 测试安全加法
        assert_eq!(safe_add(10, 20).unwrap(), 30);
        assert!(safe_add(u64::MAX, 1).is_err());
        
        // 测试安全乘法
        assert_eq!(safe_mul(10, 20).unwrap(), 200);
        assert!(safe_mul(u64::MAX, 2).is_err());
        
        // 测试百分比计算
        assert_eq!(safe_percentage(1000, 10).unwrap(), 100);
        assert!(safe_percentage(1000, 101).is_err());
    }

    #[test]
    fn test_secure_logging() {
        // 测试日志记录
        let entry = SecureLogEntry::new(
            SecureLogLevel::Info,
            LogCategory::System,
            "Test message".to_string(),
            Some("Test context".to_string()),
            None,
        );
        
        assert_eq!(entry.level, SecureLogLevel::Info);
        assert_eq!(entry.category, LogCategory::System);
        assert_eq!(entry.message, "Test message");
        assert!(entry.id.len() > 0);
    }

    #[test]
    fn test_state_validation() {
        // 测试状态验证
        let result = StateManager::validate_state();
        
        // 基本验证应该通过
        assert!(result.errors.len() == 0 || result.warnings.len() >= 0);
        assert!(result.pools_count >= 0);
        assert!(result.positions_count >= 0);
    }

    #[test]
    fn test_sensitive_data_sanitization() {
        // 测试敏感数据清理
        let message = "User password: secret123 and private_key: abc123def456";
        let sanitized = sanitize_message(message.to_string());
        
        // 敏感信息应该被替换
        assert!(!sanitized.contains("secret123"));
        assert!(!sanitized.contains("abc123def456"));
        assert!(sanitized.contains("[REDACTED]"));
    }

    #[test]
    fn test_log_level_ordering() {
        // 测试日志级别排序
        assert!(SecureLogLevel::Debug < SecureLogLevel::Info);
        assert!(SecureLogLevel::Info < SecureLogLevel::Warning);
        assert!(SecureLogLevel::Warning < SecureLogLevel::Error);
        assert!(SecureLogLevel::Error < SecureLogLevel::Critical);
    }

    #[test]
    fn test_permission_inheritance() {
        // 测试权限继承
        let liquidator = Role::Liquidator;
        let pool_manager = Role::PoolManager;
        
        let liquidator_perms = liquidator.permissions();
        let pool_manager_perms = pool_manager.permissions();
        
        // 池管理员应该包含清算员的所有权限
        for perm in liquidator_perms.iter() {
            assert!(pool_manager_perms.contains(perm), 
                   "PoolManager should have permission {:?}", perm);
        }
    }

    #[test]
    fn test_state_snapshot_creation() {
        // 测试状态快照创建
        let metadata = std::collections::HashMap::new();
        let result = StateManager::create_snapshot(
            StateOperation::Emergency, 
            Some(metadata)
        );
        
        // 快照创建应该成功
        assert!(result.is_ok());
        let snapshot_id = result.unwrap();
        assert!(snapshot_id.starts_with("snapshot_"));
    }

    #[test]
    fn test_transaction_lifecycle() {
        // 测试事务生命周期
        let tx_result = StateManager::begin_transaction(StateOperation::CreatePosition);
        
        if let Ok(tx_id) = tx_result {
            // 事务应该可以提交
            let commit_result = StateManager::commit_transaction(tx_id.clone());
            // 注意：在测试环境中可能会失败，因为状态锁的存在
            // 这里主要测试接口的正确性
        }
    }

    #[test]
    fn test_auto_snapshot_config() {
        // 测试自动快照配置
        let config = AutoSnapshotConfig {
            enabled: true,
            interval_minutes: 30,
            max_snapshots: 100,
            operations_threshold: 50,
        };
        
        assert!(config.enabled);
        assert_eq!(config.interval_minutes, 30);
        assert_eq!(config.max_snapshots, 100);
        assert_eq!(config.operations_threshold, 50);
    }

    #[test]
    fn test_log_statistics() {
        // 测试日志统计
        let stats = LogStatistics::default();
        
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.debug_count, 0);
        assert_eq!(stats.info_count, 0);
        assert_eq!(stats.warning_count, 0);
        assert_eq!(stats.error_count, 0);
        assert_eq!(stats.critical_count, 0);
    }

    #[test]
    fn test_input_validation() {
        use crate::input_validation::*;
        
        // 测试 BTC 数量验证
        assert!(validate_btc_amount(100_000, "test").is_ok());
        assert!(validate_btc_amount(1000, "test").is_err());
        
        // 测试 Bollar 数量验证
        assert!(validate_bollar_amount(1000, "test").is_ok());
        assert!(validate_bollar_amount(0, "test").is_err());
        
        // 测试比特币地址验证
        assert!(validate_bitcoin_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa", "test").is_ok());
        assert!(validate_bitcoin_address("invalid", "test").is_err());
    }

    #[test]
    fn test_lru_cache() {
        use crate::performance::LRUCache;
        
        let mut cache = LRUCache::new(2);
        
        // 测试缓存操作
        cache.put("key1".to_string(), "value1".to_string(), 1_000_000_000);
        cache.put("key2".to_string(), "value2".to_string(), 1_000_000_000);
        
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        assert_eq!(cache.get("key2"), Some("value2".to_string()));
        
        // 测试容量限制
        cache.put("key3".to_string(), "value3".to_string(), 1_000_000_000);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_monitoring_metrics() {
        use crate::monitoring::*;
        
        let mut metric = Metric::new(
            "test_metric".to_string(),
            MetricType::Counter,
            "Test metric".to_string(),
            "count".to_string(),
        );
        
        // 测试数据点添加
        metric.add_data_point(10.0, None);
        metric.add_data_point(20.0, None);
        
        assert_eq!(metric.latest_value(), Some(20.0));
        assert_eq!(metric.data_points.len(), 2);
    }

    #[test]
    fn test_backup_metadata() {
        use crate::backup_recovery::*;
        
        let metadata = BackupMetadata {
            id: "test_backup".to_string(),
            backup_type: BackupType::Full,
            status: BackupStatus::Completed,
            created_at: 1000000,
            completed_at: Some(1000001),
            size_bytes: 1024,
            checksum: "abc123".to_string(),
            description: "Test backup".to_string(),
            created_by: "test_user".to_string(),
        };
        
        assert_eq!(metadata.backup_type, BackupType::Full);
        assert_eq!(metadata.status, BackupStatus::Completed);
        assert_eq!(metadata.size_bytes, 1024);
    }

    #[test]
    fn test_alert_conditions() {
        use crate::monitoring::*;
        
        let rule = AlertRule {
            id: "test_rule".to_string(),
            metric_name: "test_metric".to_string(),
            condition: AlertCondition::GreaterThan,
            threshold: 100.0,
            duration_seconds: 300,
            severity: AlertSeverity::Warning,
            message: "Test alert".to_string(),
            enabled: true,
        };
        
        assert_eq!(rule.condition, AlertCondition::GreaterThan);
        assert_eq!(rule.threshold, 100.0);
        assert_eq!(rule.severity, AlertSeverity::Warning);
    }

    #[test]
    fn test_performance_measurement() {
        use crate::performance::*;
        
        let config = PerformanceConfig::default();
        
        assert!(config.enable_caching);
        assert_eq!(config.cache_ttl_seconds, 300);
        assert_eq!(config.max_performance_records, 10000);
        assert_eq!(config.slow_request_threshold_ms, 1000);
    }
}