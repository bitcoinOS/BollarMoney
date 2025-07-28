// monitoring.rs - 系统监控和指标收集
// 这个模块实现系统性能监控、指标收集和告警功能

use crate::{Error, Result, types::*, secure_logging::*, ic_api};
use candid::{CandidType, Deserialize};
use ic_cdk_macros::{query, update};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

// 指标类型
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum MetricType {
    Counter,      // 计数器（只增不减）
    Gauge,        // 仪表（可增可减）
    Histogram,    // 直方图
    Timer,        // 计时器
}

// 指标数据点
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct MetricDataPoint {
    pub timestamp: u64,
    pub value: f64,
    pub labels: HashMap<String, String>,
}

// 指标定义
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct Metric {
    pub name: String,
    pub metric_type: MetricType,
    pub description: String,
    pub unit: String,
    pub data_points: VecDeque<MetricDataPoint>,
    pub max_data_points: usize,
}

impl Metric {
    pub fn new(name: String, metric_type: MetricType, description: String, unit: String) -> Self {
        Self {
            name,
            metric_type,
            description,
            unit,
            data_points: VecDeque::new(),
            max_data_points: 1000, // 默认保留1000个数据点
        }
    }
    
    // 添加数据点
    pub fn add_data_point(&mut self, value: f64, labels: Option<HashMap<String, String>>) {
        let data_point = MetricDataPoint {
            timestamp: ic_api::time(),
            value,
            labels: labels.unwrap_or_default(),
        };
        
        self.data_points.push_back(data_point);
        
        // 维护数据点数量限制
        while self.data_points.len() > self.max_data_points {
            self.data_points.pop_front();
        }
    }
    
    // 获取最新值
    pub fn latest_value(&self) -> Option<f64> {
        self.data_points.back().map(|dp| dp.value)
    }
    
    // 获取平均值
    pub fn average_value(&self, duration_ns: Option<u64>) -> Option<f64> {
        if self.data_points.is_empty() {
            return None;
        }
        
        let current_time = ic_api::time();
        let cutoff_time = duration_ns.map(|d| current_time.saturating_sub(d));
        
        let relevant_points: Vec<&MetricDataPoint> = self.data_points
            .iter()
            .filter(|dp| {
                cutoff_time.map_or(true, |ct| dp.timestamp >= ct)
            })
            .collect();
        
        if relevant_points.is_empty() {
            return None;
        }
        
        let sum: f64 = relevant_points.iter().map(|dp| dp.value).sum();
        Some(sum / relevant_points.len() as f64)
    }
}

// 告警规则
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct AlertRule {
    pub id: String,
    pub metric_name: String,
    pub condition: AlertCondition,
    pub threshold: f64,
    pub duration_seconds: u64,
    pub severity: AlertSeverity,
    pub message: String,
    pub enabled: bool,
}

// 告警条件
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum AlertCondition {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

// 告警严重程度
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info = 0,
    Warning = 1,
    Error = 2,
    Critical = 3,
}

// 告警事件
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct AlertEvent {
    pub id: String,
    pub rule_id: String,
    pub metric_name: String,
    pub current_value: f64,
    pub threshold: f64,
    pub severity: AlertSeverity,
    pub message: String,
    pub triggered_at: u64,
    pub resolved_at: Option<u64>,
    pub acknowledged: bool,
}

// 系统健康状态
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SystemHealth {
    pub overall_status: HealthStatus,
    pub components: HashMap<String, ComponentHealth>,
    pub last_updated: u64,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    pub message: String,
    pub last_check: u64,
    pub metrics: HashMap<String, f64>,
}

thread_local! {
    // 指标存储
    static METRICS: RefCell<HashMap<String, Metric>> = RefCell::new(HashMap::new());
    
    // 告警规则
    static ALERT_RULES: RefCell<HashMap<String, AlertRule>> = RefCell::new(HashMap::new());
    
    // 活跃告警
    static ACTIVE_ALERTS: RefCell<HashMap<String, AlertEvent>> = RefCell::new(HashMap::new());
    
    // 告警历史
    static ALERT_HISTORY: RefCell<VecDeque<AlertEvent>> = RefCell::new(VecDeque::new());
    
    // 系统健康状态
    static SYSTEM_HEALTH: RefCell<SystemHealth> = RefCell::new(SystemHealth {
        overall_status: HealthStatus::Unknown,
        components: HashMap::new(),
        last_updated: 0,
    });
}

// 监控管理器
pub struct MonitoringManager;

impl MonitoringManager {
    // 初始化默认指标
    pub fn initialize_default_metrics() {
        let default_metrics = vec![
            ("transaction_count", MetricType::Counter, "Total number of transactions", "count"),
            ("active_positions", MetricType::Gauge, "Number of active positions", "count"),
            ("total_btc_locked", MetricType::Gauge, "Total BTC locked in protocol", "satoshis"),
            ("total_bollar_supply", MetricType::Gauge, "Total Bollar supply", "cents"),
            ("liquidation_count", MetricType::Counter, "Number of liquidations", "count"),
            ("average_health_factor", MetricType::Gauge, "Average health factor of positions", "ratio"),
            ("btc_price", MetricType::Gauge, "Current BTC price", "cents"),
            ("system_memory_usage", MetricType::Gauge, "System memory usage", "bytes"),
            ("api_response_time", MetricType::Timer, "API response time", "milliseconds"),
            ("error_rate", MetricType::Gauge, "Error rate", "percentage"),
        ];
        
        METRICS.with_borrow_mut(|metrics| {
            for (name, metric_type, description, unit) in default_metrics {
                let metric = Metric::new(
                    name.to_string(),
                    metric_type,
                    description.to_string(),
                    unit.to_string(),
                );
                metrics.insert(name.to_string(), metric);
            }
        });
        
        // 初始化默认告警规则
        Self::initialize_default_alert_rules();
    }
    
    // 初始化默认告警规则
    fn initialize_default_alert_rules() {
        let default_rules = vec![
            AlertRule {
                id: "high_liquidation_rate".to_string(),
                metric_name: "liquidation_count".to_string(),
                condition: AlertCondition::GreaterThan,
                threshold: 10.0,
                duration_seconds: 300, // 5分钟
                severity: AlertSeverity::Warning,
                message: "High liquidation rate detected".to_string(),
                enabled: true,
            },
            AlertRule {
                id: "low_average_health_factor".to_string(),
                metric_name: "average_health_factor".to_string(),
                condition: AlertCondition::LessThan,
                threshold: 120.0, // 120%
                duration_seconds: 600, // 10分钟
                severity: AlertSeverity::Warning,
                message: "Average health factor is low".to_string(),
                enabled: true,
            },
            AlertRule {
                id: "high_error_rate".to_string(),
                metric_name: "error_rate".to_string(),
                condition: AlertCondition::GreaterThan,
                threshold: 5.0, // 5%
                duration_seconds: 180, // 3分钟
                severity: AlertSeverity::Error,
                message: "High error rate detected".to_string(),
                enabled: true,
            },
            AlertRule {
                id: "extreme_btc_price_change".to_string(),
                metric_name: "btc_price".to_string(),
                condition: AlertCondition::GreaterThan,
                threshold: 10.0, // 10% 变化
                duration_seconds: 60, // 1分钟
                severity: AlertSeverity::Critical,
                message: "Extreme BTC price change detected".to_string(),
                enabled: true,
            },
        ];
        
        ALERT_RULES.with_borrow_mut(|rules| {
            for rule in default_rules {
                rules.insert(rule.id.clone(), rule);
            }
        });
    }
    
    // 记录指标
    pub fn record_metric(name: &str, value: f64, labels: Option<HashMap<String, String>>) {
        METRICS.with_borrow_mut(|metrics| {
            if let Some(metric) = metrics.get_mut(name) {
                metric.add_data_point(value, labels);
            }
        });
    }
    
    // 增加计数器
    pub fn increment_counter(name: &str, labels: Option<HashMap<String, String>>) {
        METRICS.with_borrow_mut(|metrics| {
            if let Some(metric) = metrics.get_mut(name) {
                let current_value = metric.latest_value().unwrap_or(0.0);
                metric.add_data_point(current_value + 1.0, labels);
            }
        });
    }
    
    // 检查告警规则
    pub fn check_alert_rules() {
        let current_time = ic_api::time();
        
        ALERT_RULES.with_borrow(|rules| {
            for rule in rules.values() {
                if !rule.enabled {
                    continue;
                }
                
                Self::evaluate_alert_rule(rule, current_time);
            }
        });
    }
    
    // 评估告警规则
    fn evaluate_alert_rule(rule: &AlertRule, current_time: u64) {
        let metric_value = METRICS.with_borrow(|metrics| {
            metrics.get(&rule.metric_name)
                .and_then(|m| m.latest_value())
        });
        
        let Some(value) = metric_value else {
            return;
        };
        
        let condition_met = match rule.condition {
            AlertCondition::GreaterThan => value > rule.threshold,
            AlertCondition::LessThan => value < rule.threshold,
            AlertCondition::Equal => (value - rule.threshold).abs() < f64::EPSILON,
            AlertCondition::NotEqual => (value - rule.threshold).abs() >= f64::EPSILON,
            AlertCondition::GreaterThanOrEqual => value >= rule.threshold,
            AlertCondition::LessThanOrEqual => value <= rule.threshold,
        };
        
        if condition_met {
            Self::trigger_alert(rule, value, current_time);
        } else {
            Self::resolve_alert(&rule.id, current_time);
        }
    }
    
    // 触发告警
    fn trigger_alert(rule: &AlertRule, current_value: f64, current_time: u64) {
        let alert_id = format!("{}_{}", rule.id, current_time);
        
        let alert_event = AlertEvent {
            id: alert_id.clone(),
            rule_id: rule.id.clone(),
            metric_name: rule.metric_name.clone(),
            current_value,
            threshold: rule.threshold,
            severity: rule.severity.clone(),
            message: rule.message.clone(),
            triggered_at: current_time,
            resolved_at: None,
            acknowledged: false,
        };
        
        // 检查是否已经有相同的活跃告警
        let already_active = ACTIVE_ALERTS.with_borrow(|alerts| {
            alerts.values().any(|alert| alert.rule_id == rule.id)
        });
        
        if !already_active {
            ACTIVE_ALERTS.with_borrow_mut(|alerts| {
                alerts.insert(alert_id.clone(), alert_event.clone());
            });
            
            // 记录告警日志
            let log_level = match rule.severity {
                AlertSeverity::Info => SecureLogLevel::Info,
                AlertSeverity::Warning => SecureLogLevel::Warning,
                AlertSeverity::Error => SecureLogLevel::Error,
                AlertSeverity::Critical => SecureLogLevel::Critical,
            };
            
            secure_log(
                log_level,
                LogCategory::Security,
                format!("Alert triggered: {}", rule.message),
                Some(format!("Metric: {}, Value: {}, Threshold: {}", 
                           rule.metric_name, current_value, rule.threshold)),
                None,
            );
        }
    }
    
    // 解决告警
    fn resolve_alert(rule_id: &str, current_time: u64) {
        ACTIVE_ALERTS.with_borrow_mut(|alerts| {
            if let Some(mut alert) = alerts.values().find(|a| a.rule_id == rule_id).cloned() {
                alert.resolved_at = Some(current_time);
                
                // 移动到历史记录
                ALERT_HISTORY.with_borrow_mut(|history| {
                    history.push_back(alert.clone());
                    
                    // 保持历史记录大小
                    while history.len() > 1000 {
                        history.pop_front();
                    }
                });
                
                // 从活跃告警中移除
                alerts.retain(|_, a| a.rule_id != rule_id);
                
                secure_log_info!(
                    LogCategory::Security,
                    format!("Alert resolved: {}", rule_id)
                );
            }
        });
    }
    
    // 更新系统健康状态
    pub fn update_system_health() {
        let current_time = ic_api::time();
        
        let mut components = HashMap::new();
        
        // 检查各个组件的健康状态
        components.insert("lending".to_string(), Self::check_lending_health());
        components.insert("liquidation".to_string(), Self::check_liquidation_health());
        components.insert("oracle".to_string(), Self::check_oracle_health());
        components.insert("emergency".to_string(), Self::check_emergency_health());
        
        // 计算整体健康状态
        let overall_status = components.values()
            .map(|c| &c.status)
            .max()
            .cloned()
            .unwrap_or(HealthStatus::Unknown);
        
        let system_health = SystemHealth {
            overall_status,
            components,
            last_updated: current_time,
        };
        
        SYSTEM_HEALTH.with_borrow_mut(|health| {
            *health = system_health;
        });
    }
    
    // 检查借贷模块健康状态
    fn check_lending_health() -> ComponentHealth {
        let positions = crate::get_positions();
        let mut metrics = HashMap::new();
        
        let total_positions = positions.len() as f64;
        metrics.insert("total_positions".to_string(), total_positions);
        
        if total_positions > 0.0 {
            let avg_health_factor = positions.iter()
                .map(|p| p.health_factor as f64)
                .sum::<f64>() / total_positions;
            metrics.insert("average_health_factor".to_string(), avg_health_factor);
            
            let status = if avg_health_factor < 110.0 {
                HealthStatus::Critical
            } else if avg_health_factor < 130.0 {
                HealthStatus::Warning
            } else {
                HealthStatus::Healthy
            };
            
            ComponentHealth {
                status,
                message: format!("Average health factor: {:.2}", avg_health_factor),
                last_check: ic_api::time(),
                metrics,
            }
        } else {
            ComponentHealth {
                status: HealthStatus::Healthy,
                message: "No active positions".to_string(),
                last_check: ic_api::time(),
                metrics,
            }
        }
    }
    
    // 检查清算模块健康状态
    fn check_liquidation_health() -> ComponentHealth {
        let positions = crate::get_positions();
        let mut metrics = HashMap::new();
        
        let liquidatable_count = positions.iter()
            .filter(|p| p.health_factor < 80) // 假设80%为清算阈值
            .count() as f64;
        
        metrics.insert("liquidatable_positions".to_string(), liquidatable_count);
        
        let status = if liquidatable_count > 10.0 {
            HealthStatus::Warning
        } else if liquidatable_count > 50.0 {
            HealthStatus::Critical
        } else {
            HealthStatus::Healthy
        };
        
        ComponentHealth {
            status,
            message: format!("{} positions need liquidation", liquidatable_count),
            last_check: ic_api::time(),
            metrics,
        }
    }
    
    // 检查预言机健康状态
    fn check_oracle_health() -> ComponentHealth {
        let btc_price = crate::oracle::get_btc_price();
        let mut metrics = HashMap::new();
        
        metrics.insert("btc_price".to_string(), btc_price as f64);
        
        let status = if btc_price == 0 {
            HealthStatus::Critical
        } else if !crate::oracle::is_price_valid() {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };
        
        ComponentHealth {
            status,
            message: format!("BTC price: ${}.{}", btc_price / 100, btc_price % 100),
            last_check: ic_api::time(),
            metrics,
        }
    }
    
    // 检查紧急控制健康状态
    fn check_emergency_health() -> ComponentHealth {
        let emergency_state = crate::emergency::get_emergency_state();
        let mut metrics = HashMap::new();
        
        let status = match emergency_state.state {
            crate::emergency::EmergencyState::Normal => HealthStatus::Healthy,
            crate::emergency::EmergencyState::MaintenanceMode => HealthStatus::Warning,
            _ => HealthStatus::Critical,
        };
        
        ComponentHealth {
            status,
            message: format!("Emergency state: {:?}", emergency_state.state),
            last_check: ic_api::time(),
            metrics,
        }
    }
}

#[query]
// 获取指标数据
pub fn get_metric(name: String, limit: Option<usize>) -> Option<Metric> {
    METRICS.with_borrow(|metrics| {
        metrics.get(&name).map(|metric| {
            let mut result = metric.clone();
            let limit = limit.unwrap_or(100).min(1000);
            
            // 限制返回的数据点数量
            if result.data_points.len() > limit {
                let start_index = result.data_points.len() - limit;
                result.data_points = result.data_points.iter()
                    .skip(start_index)
                    .cloned()
                    .collect();
            }
            
            result
        })
    })
}

#[query]
// 获取所有指标名称
pub fn get_metric_names() -> Vec<String> {
    METRICS.with_borrow(|metrics| {
        metrics.keys().cloned().collect()
    })
}

#[query]
// 获取系统健康状态
pub fn get_system_health() -> SystemHealth {
    SYSTEM_HEALTH.with_borrow(|health| health.clone())
}

#[query]
// 获取活跃告警
pub fn get_active_alerts() -> Vec<AlertEvent> {
    ACTIVE_ALERTS.with_borrow(|alerts| {
        alerts.values().cloned().collect()
    })
}

#[query]
// 获取告警历史
pub fn get_alert_history(limit: Option<usize>) -> Vec<AlertEvent> {
    ALERT_HISTORY.with_borrow(|history| {
        let limit = limit.unwrap_or(100).min(1000);
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    })
}

#[update]
// 确认告警
pub fn acknowledge_alert(alert_id: String) -> Result<()> {
    let caller = ic_api::caller();
    crate::access_control::check_permission(caller, crate::access_control::Permission::SystemAdmin)?;
    
    ACTIVE_ALERTS.with_borrow_mut(|alerts| {
        if let Some(alert) = alerts.get_mut(&alert_id) {
            alert.acknowledged = true;
            secure_log_info!(
                LogCategory::Security,
                format!("Alert acknowledged: {}", alert_id),
                format!("By: {}", caller)
            );
            Ok(())
        } else {
            Err(Error::InvalidArgument("Alert not found".to_string()))
        }
    })
}

// 收集协议指标
pub fn collect_protocol_metrics() {
    let metrics = crate::lending::get_protocol_metrics();
    
    MonitoringManager::record_metric("active_positions", metrics.positions_count as f64, None);
    MonitoringManager::record_metric("total_btc_locked", metrics.total_btc_locked as f64, None);
    MonitoringManager::record_metric("total_bollar_supply", metrics.total_bollar_supply as f64, None);
    MonitoringManager::record_metric("btc_price", metrics.btc_price as f64, None);
    MonitoringManager::record_metric("liquidatable_positions_count", metrics.liquidatable_positions_count as f64, None);
    
    // 计算平均健康因子
    let positions = crate::get_positions();
    if !positions.is_empty() {
        let avg_health_factor = positions.iter()
            .map(|p| p.health_factor as f64)
            .sum::<f64>() / positions.len() as f64;
        MonitoringManager::record_metric("average_health_factor", avg_health_factor, None);
    }
}

// 心跳函数中的监控任务
#[ic_cdk_macros::heartbeat]
async fn monitoring_heartbeat() {
    collect_protocol_metrics();
    MonitoringManager::check_alert_rules();
    MonitoringManager::update_system_health();
}