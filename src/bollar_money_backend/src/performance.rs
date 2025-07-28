// performance.rs - 性能优化和资源管理
// 这个模块实现性能监控、缓存管理和资源优化

use crate::{Error, Result, types::*, secure_logging::*, ic_api};
use candid::{CandidType, Deserialize};
use ic_cdk_macros::{query, update};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

// 性能指标
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct PerformanceMetrics {
    pub avg_response_time_ms: f64,
    pub max_response_time_ms: f64,
    pub min_response_time_ms: f64,
    pub total_requests: u64,
    pub error_count: u64,
    pub cache_hit_rate: f64,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
}

// 请求性能数据
#[derive(Clone, Debug)]
pub struct RequestPerformance {
    pub endpoint: String,
    pub start_time: u64,
    pub end_time: u64,
    pub duration_ns: u64,
    pub success: bool,
    pub error_type: Option<String>,
}

// 缓存条目
#[derive(Clone, Debug)]
pub struct CacheEntry<T> {
    pub value: T,
    pub created_at: u64,
    pub last_accessed: u64,
    pub access_count: u64,
    pub ttl_ns: u64,
}

impl<T> CacheEntry<T> {
    pub fn new(value: T, ttl_ns: u64) -> Self {
        let now = ic_api::time();
        Self {
            value,
            created_at: now,
            last_accessed: now,
            access_count: 1,
            ttl_ns,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        let now = ic_api::time();
        now - self.created_at > self.ttl_ns
    }
    
    pub fn access(&mut self) -> &T {
        self.last_accessed = ic_api::time();
        self.access_count += 1;
        &self.value
    }
}

// LRU 缓存
pub struct LRUCache<T> {
    capacity: usize,
    data: HashMap<String, CacheEntry<T>>,
    access_order: VecDeque<String>,
}

impl<T: Clone> LRUCache<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            data: HashMap::new(),
            access_order: VecDeque::new(),
        }
    }
    
    pub fn get(&mut self, key: &str) -> Option<T> {
        if let Some(entry) = self.data.get_mut(key) {
            if entry.is_expired() {
                self.remove(key);
                return None;
            }
            
            // 更新访问顺序
            self.access_order.retain(|k| k != key);
            self.access_order.push_back(key.to_string());
            
            Some(entry.access().clone())
        } else {
            None
        }
    }
    
    pub fn put(&mut self, key: String, value: T, ttl_ns: u64) {
        // 如果已存在，先移除
        if self.data.contains_key(&key) {
            self.remove(&key);
        }
        
        // 如果达到容量限制，移除最少使用的条目
        while self.data.len() >= self.capacity {
            if let Some(lru_key) = self.access_order.pop_front() {
                self.data.remove(&lru_key);
            }
        }
        
        // 添加新条目
        let entry = CacheEntry::new(value, ttl_ns);
        self.data.insert(key.clone(), entry);
        self.access_order.push_back(key);
    }
    
    pub fn remove(&mut self, key: &str) {
        self.data.remove(key);
        self.access_order.retain(|k| k != key);
    }
    
    pub fn clear(&mut self) {
        self.data.clear();
        self.access_order.clear();
    }
    
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    pub fn hit_rate(&self) -> f64 {
        if self.data.is_empty() {
            return 0.0;
        }
        
        let total_accesses: u64 = self.data.values().map(|e| e.access_count).sum();
        let hits = self.data.len() as u64;
        
        if total_accesses == 0 {
            0.0
        } else {
            hits as f64 / total_accesses as f64
        }
    }
    
    // 清理过期条目
    pub fn cleanup_expired(&mut self) -> usize {
        let expired_keys: Vec<String> = self.data
            .iter()
            .filter(|(_, entry)| entry.is_expired())
            .map(|(key, _)| key.clone())
            .collect();
        
        let count = expired_keys.len();
        for key in expired_keys {
            self.remove(&key);
        }
        
        count
    }
}

thread_local! {
    // 性能数据存储
    static PERFORMANCE_DATA: RefCell<VecDeque<RequestPerformance>> = RefCell::new(VecDeque::new());
    
    // 缓存系统
    static POSITION_CACHE: RefCell<LRUCache<Position>> = RefCell::new(LRUCache::new(1000));
    static POOL_CACHE: RefCell<LRUCache<Pool>> = RefCell::new(LRUCache::new(100));
    static METRICS_CACHE: RefCell<LRUCache<ProtocolMetrics>> = RefCell::new(LRUCache::new(10));
    
    // 性能配置
    static PERFORMANCE_CONFIG: RefCell<PerformanceConfig> = RefCell::new(PerformanceConfig::default());
}

// 性能配置
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct PerformanceConfig {
    pub enable_caching: bool,
    pub cache_ttl_seconds: u64,
    pub max_performance_records: usize,
    pub enable_performance_logging: bool,
    pub slow_request_threshold_ms: u64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            cache_ttl_seconds: 300, // 5分钟
            max_performance_records: 10000,
            enable_performance_logging: true,
            slow_request_threshold_ms: 1000, // 1秒
        }
    }
}

// 性能管理器
pub struct PerformanceManager;

impl PerformanceManager {
    // 开始性能测量
    pub fn start_measurement(endpoint: &str) -> PerformanceMeasurement {
        PerformanceMeasurement {
            endpoint: endpoint.to_string(),
            start_time: ic_api::time(),
        }
    }
    
    // 记录请求性能
    pub fn record_request_performance(
        endpoint: String,
        start_time: u64,
        success: bool,
        error_type: Option<String>,
    ) {
        let end_time = ic_api::time();
        let duration_ns = end_time.saturating_sub(start_time);
        
        let performance = RequestPerformance {
            endpoint: endpoint.clone(),
            start_time,
            end_time,
            duration_ns,
            success,
            error_type,
        };
        
        PERFORMANCE_DATA.with_borrow_mut(|data| {
            data.push_back(performance.clone());
            
            // 维护数据大小限制
            let max_records = PERFORMANCE_CONFIG.with_borrow(|config| config.max_performance_records);
            while data.len() > max_records {
                data.pop_front();
            }
        });
        
        // 检查是否为慢请求
        let config = PERFORMANCE_CONFIG.with_borrow(|config| config.clone());
        if config.enable_performance_logging {
            let duration_ms = duration_ns / 1_000_000;
            if duration_ms > config.slow_request_threshold_ms {
                secure_log_warning!(
                    LogCategory::System,
                    format!("Slow request detected: {} took {}ms", endpoint, duration_ms),
                    format!("Success: {}", success)
                );
            }
        }
    }
    
    // 获取性能指标
    pub fn get_performance_metrics() -> PerformanceMetrics {
        PERFORMANCE_DATA.with_borrow(|data| {
            if data.is_empty() {
                return PerformanceMetrics {
                    avg_response_time_ms: 0.0,
                    max_response_time_ms: 0.0,
                    min_response_time_ms: 0.0,
                    total_requests: 0,
                    error_count: 0,
                    cache_hit_rate: 0.0,
                    memory_usage_bytes: 0,
                    cpu_usage_percent: 0.0,
                };
            }
            
            let durations_ms: Vec<f64> = data.iter()
                .map(|p| p.duration_ns as f64 / 1_000_000.0)
                .collect();
            
            let avg_response_time_ms = durations_ms.iter().sum::<f64>() / durations_ms.len() as f64;
            let max_response_time_ms = durations_ms.iter().fold(0.0, |a, &b| a.max(b));
            let min_response_time_ms = durations_ms.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            
            let total_requests = data.len() as u64;
            let error_count = data.iter().filter(|p| !p.success).count() as u64;
            
            // 计算缓存命中率
            let cache_hit_rate = POSITION_CACHE.with_borrow(|cache| cache.hit_rate());
            
            PerformanceMetrics {
                avg_response_time_ms,
                max_response_time_ms,
                min_response_time_ms,
                total_requests,
                error_count,
                cache_hit_rate,
                memory_usage_bytes: Self::estimate_memory_usage(),
                cpu_usage_percent: 0.0, // IC 环境中难以准确测量
            }
        })
    }
    
    // 估算内存使用量
    fn estimate_memory_usage() -> u64 {
        let positions_count = crate::get_positions().len();
        let pools_count = crate::get_pools().len();
        
        // 粗略估算
        let position_size = std::mem::size_of::<Position>();
        let pool_size = std::mem::size_of::<Pool>();
        
        let positions_memory = positions_count * position_size;
        let pools_memory = pools_count * pool_size;
        
        // 加上缓存和其他数据结构的估算
        let cache_memory = POSITION_CACHE.with_borrow(|cache| cache.len() * position_size);
        let performance_memory = PERFORMANCE_DATA.with_borrow(|data| {
            data.len() * std::mem::size_of::<RequestPerformance>()
        });
        
        (positions_memory + pools_memory + cache_memory + performance_memory) as u64
    }
    
    // 清理缓存
    pub fn cleanup_caches() -> CacheCleanupResult {
        let position_expired = POSITION_CACHE.with_borrow_mut(|cache| cache.cleanup_expired());
        let pool_expired = POOL_CACHE.with_borrow_mut(|cache| cache.cleanup_expired());
        let metrics_expired = METRICS_CACHE.with_borrow_mut(|cache| cache.cleanup_expired());
        
        CacheCleanupResult {
            position_cache_expired: position_expired,
            pool_cache_expired: pool_expired,
            metrics_cache_expired: metrics_expired,
        }
    }
    
    // 预热缓存
    pub fn warm_up_cache() {
        let config = PERFORMANCE_CONFIG.with_borrow(|config| config.clone());
        if !config.enable_caching {
            return;
        }
        
        let ttl_ns = config.cache_ttl_seconds * 1_000_000_000;
        
        // 预热位置缓存
        let positions = crate::get_positions();
        POSITION_CACHE.with_borrow_mut(|cache| {
            for position in positions {
                cache.put(position.id.clone(), position, ttl_ns);
            }
        });
        
        // 预热池缓存
        let pools = crate::get_pools();
        POOL_CACHE.with_borrow_mut(|cache| {
            for pool in pools {
                cache.put(pool.addr.clone(), pool, ttl_ns);
            }
        });
        
        secure_log_info!(
            LogCategory::System,
            "Cache warmed up successfully"
        );
    }
}

// 性能测量辅助结构
pub struct PerformanceMeasurement {
    endpoint: String,
    start_time: u64,
}

impl PerformanceMeasurement {
    // 完成测量
    pub fn finish(self, success: bool, error_type: Option<String>) {
        PerformanceManager::record_request_performance(
            self.endpoint,
            self.start_time,
            success,
            error_type,
        );
    }
}

// 缓存清理结果
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CacheCleanupResult {
    pub position_cache_expired: usize,
    pub pool_cache_expired: usize,
    pub metrics_cache_expired: usize,
}

// 缓存辅助函数
pub fn get_cached_position(position_id: &str) -> Option<Position> {
    let config = PERFORMANCE_CONFIG.with_borrow(|config| config.clone());
    if !config.enable_caching {
        return None;
    }
    
    POSITION_CACHE.with_borrow_mut(|cache| cache.get(position_id))
}

pub fn cache_position(position: Position) {
    let config = PERFORMANCE_CONFIG.with_borrow(|config| config.clone());
    if !config.enable_caching {
        return;
    }
    
    let ttl_ns = config.cache_ttl_seconds * 1_000_000_000;
    POSITION_CACHE.with_borrow_mut(|cache| {
        cache.put(position.id.clone(), position, ttl_ns);
    });
}

pub fn get_cached_pool(pool_address: &str) -> Option<Pool> {
    let config = PERFORMANCE_CONFIG.with_borrow(|config| config.clone());
    if !config.enable_caching {
        return None;
    }
    
    POOL_CACHE.with_borrow_mut(|cache| cache.get(pool_address))
}

pub fn cache_pool(pool: Pool) {
    let config = PERFORMANCE_CONFIG.with_borrow(|config| config.clone());
    if !config.enable_caching {
        return;
    }
    
    let ttl_ns = config.cache_ttl_seconds * 1_000_000_000;
    POOL_CACHE.with_borrow_mut(|cache| {
        cache.put(pool.addr.clone(), pool, ttl_ns);
    });
}

// 性能监控宏
#[macro_export]
macro_rules! measure_performance {
    ($endpoint:expr, $operation:expr) => {{
        let measurement = $crate::performance::PerformanceManager::start_measurement($endpoint);
        let result = $operation;
        let success = result.is_ok();
        let error_type = if let Err(ref e) = result {
            Some(format!("{:?}", e))
        } else {
            None
        };
        measurement.finish(success, error_type);
        result
    }};
}

#[query]
// 获取性能指标
pub fn get_performance_metrics() -> PerformanceMetrics {
    // 检查权限
    let caller = ic_api::caller();
    if !crate::access_control::has_permission(caller, crate::access_control::Permission::ViewMetrics) {
        return PerformanceMetrics {
            avg_response_time_ms: 0.0,
            max_response_time_ms: 0.0,
            min_response_time_ms: 0.0,
            total_requests: 0,
            error_count: 0,
            cache_hit_rate: 0.0,
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
        };
    }
    
    PerformanceManager::get_performance_metrics()
}

#[update]
// 更新性能配置
pub fn update_performance_config(config: PerformanceConfig) -> Result<()> {
    // 检查权限
    let caller = ic_api::caller();
    crate::access_control::check_permission(caller, crate::access_control::Permission::SystemAdmin)?;
    
    // 验证配置
    validate_param!(config.cache_ttl_seconds > 0 && config.cache_ttl_seconds <= 3600, 
                   "Cache TTL must be between 1 and 3600 seconds");
    validate_param!(config.max_performance_records > 0 && config.max_performance_records <= 100000, 
                   "Max performance records must be between 1 and 100000");
    
    PERFORMANCE_CONFIG.with_borrow_mut(|current_config| {
        *current_config = config.clone();
    });
    
    secure_log_info!(
        LogCategory::System,
        "Performance configuration updated",
        format!("Updated by: {}", caller)
    );
    
    Ok(())
}

#[update]
// 清理缓存
pub fn cleanup_caches() -> Result<CacheCleanupResult> {
    // 检查权限
    let caller = ic_api::caller();
    crate::access_control::check_permission(caller, crate::access_control::Permission::SystemAdmin)?;
    
    let result = PerformanceManager::cleanup_caches();
    
    secure_log_info!(
        LogCategory::System,
        format!("Cache cleanup completed: {} positions, {} pools, {} metrics expired", 
                result.position_cache_expired, result.pool_cache_expired, result.metrics_cache_expired),
        format!("Initiated by: {}", caller)
    );
    
    Ok(result)
}

#[update]
// 预热缓存
pub fn warm_up_cache() -> Result<()> {
    // 检查权限
    let caller = ic_api::caller();
    crate::access_control::check_permission(caller, crate::access_control::Permission::SystemAdmin)?;
    
    PerformanceManager::warm_up_cache();
    
    Ok(())
}

// 心跳函数中的性能管理任务
#[ic_cdk_macros::heartbeat]
async fn performance_heartbeat() {
    // 定期清理过期缓存
    PerformanceManager::cleanup_caches();
    
    // 记录性能指标到监控系统
    let metrics = PerformanceManager::get_performance_metrics();
    crate::monitoring::MonitoringManager::record_metric("avg_response_time", metrics.avg_response_time_ms, None);
    crate::monitoring::MonitoringManager::record_metric("error_rate", 
        (metrics.error_count as f64 / metrics.total_requests.max(1) as f64) * 100.0, None);
    crate::monitoring::MonitoringManager::record_metric("cache_hit_rate", metrics.cache_hit_rate * 100.0, None);
    crate::monitoring::MonitoringManager::record_metric("memory_usage", metrics.memory_usage_bytes as f64, None);
}