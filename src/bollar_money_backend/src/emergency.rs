// emergency.rs - 紧急控制机制
// 这个模块实现紧急暂停、恢复和其他紧急控制功能

use crate::{Error, LogLevel, Result, error::log_error, ic_api};
use candid::{CandidType, Deserialize};
use ic_cdk_macros::{query, update};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;

// 紧急状态类型
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum EmergencyState {
    Normal,                    // 正常运行
    Paused,                   // 全面暂停
    DepositPaused,            // 仅暂停存款
    WithdrawPaused,           // 仅暂停提款
    LiquidationPaused,        // 仅暂停清算
    MaintenanceMode,          // 维护模式
}

// 紧急控制结构
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct EmergencyControls {
    pub state: EmergencyState,
    pub reason: String,
    pub timestamp: u64,
    pub operator: String,
    pub auto_resume_time: Option<u64>,
}

impl Default for EmergencyControls {
    fn default() -> Self {
        Self {
            state: EmergencyState::Normal,
            reason: "System initialized".to_string(),
            timestamp: ic_api::time(),
            operator: "system".to_string(),
            auto_resume_time: None,
        }
    }
}

thread_local! {
    // 紧急控制状态
    static EMERGENCY_CONTROLS: RefCell<EmergencyControls> = RefCell::new(EmergencyControls::default());
    
    // 操作权限映射
    static EMERGENCY_OPERATORS: RefCell<HashMap<String, bool>> = RefCell::new(HashMap::new());
}

#[query]
// 获取当前紧急状态
pub fn get_emergency_state() -> EmergencyControls {
    EMERGENCY_CONTROLS.with_borrow(|controls| controls.clone())
}

#[query]
// 检查系统是否正常运行
pub fn is_system_operational() -> bool {
    EMERGENCY_CONTROLS.with_borrow(|controls| {
        controls.state == EmergencyState::Normal
    })
}

#[query]
// 检查特定操作是否被允许
pub fn is_operation_allowed(operation: String) -> bool {
    EMERGENCY_CONTROLS.with_borrow(|controls| {
        match controls.state {
            EmergencyState::Normal => true,
            EmergencyState::Paused => false,
            EmergencyState::DepositPaused => operation != "deposit",
            EmergencyState::WithdrawPaused => operation != "withdraw" && operation != "repay",
            EmergencyState::LiquidationPaused => operation != "liquidate",
            EmergencyState::MaintenanceMode => false,
        }
    })
}

#[update]
// 紧急暂停系统
pub fn emergency_pause(reason: String) -> Result<()> {
    // 验证调用者权限
    let caller = ic_api::caller().to_string();
    if !is_emergency_operator(&caller) {
        return Err(Error::PermissionDenied("Not authorized for emergency operations".to_string()));
    }
    
    // 验证原因不为空
    if reason.trim().is_empty() {
        return Err(Error::InvalidArgument("Emergency reason cannot be empty".to_string()));
    }
    
    // 设置紧急状态
    EMERGENCY_CONTROLS.with_borrow_mut(|controls| {
        controls.state = EmergencyState::Paused;
        controls.reason = reason.clone();
        controls.timestamp = ic_api::time();
        controls.operator = caller.clone();
        controls.auto_resume_time = None;
    });
    
    // 记录紧急暂停事件
    log_error(
        LogLevel::Error,
        &Error::SystemError("Emergency pause activated".to_string()),
        Some(&format!("Operator: {}, Reason: {}", caller, reason))
    );
    
    ic_cdk::println!("EMERGENCY: System paused by {} - {}", caller, reason);
    
    Ok(())
}

#[update]
// 部分暂停系统
pub fn emergency_partial_pause(operation: String, reason: String) -> Result<()> {
    // 验证调用者权限
    let caller = ic_api::caller().to_string();
    if !is_emergency_operator(&caller) {
        return Err(Error::PermissionDenied("Not authorized for emergency operations".to_string()));
    }
    
    // 验证参数
    if reason.trim().is_empty() {
        return Err(Error::InvalidArgument("Emergency reason cannot be empty".to_string()));
    }
    
    // 确定暂停状态
    let new_state = match operation.as_str() {
        "deposit" => EmergencyState::DepositPaused,
        "withdraw" => EmergencyState::WithdrawPaused,
        "liquidate" => EmergencyState::LiquidationPaused,
        _ => return Err(Error::InvalidArgument("Invalid operation type".to_string())),
    };
    
    // 设置紧急状态
    EMERGENCY_CONTROLS.with_borrow_mut(|controls| {
        controls.state = new_state;
        controls.reason = reason.clone();
        controls.timestamp = ic_api::time();
        controls.operator = caller.clone();
        controls.auto_resume_time = None;
    });
    
    // 记录部分暂停事件
    ic_cdk::println!("EMERGENCY: {} paused by {} - {}", operation, caller, reason);
    
    Ok(())
}

#[update]
// 恢复系统正常运行
pub fn emergency_resume(reason: String) -> Result<()> {
    // 验证调用者权限
    let caller = ic_api::caller().to_string();
    if !is_emergency_operator(&caller) {
        return Err(Error::PermissionDenied("Not authorized for emergency operations".to_string()));
    }
    
    // 验证原因不为空
    if reason.trim().is_empty() {
        return Err(Error::InvalidArgument("Resume reason cannot be empty".to_string()));
    }
    
    // 恢复正常状态
    EMERGENCY_CONTROLS.with_borrow_mut(|controls| {
        controls.state = EmergencyState::Normal;
        controls.reason = reason.clone();
        controls.timestamp = ic_api::time();
        controls.operator = caller.clone();
        controls.auto_resume_time = None;
    });
    
    // 记录恢复事件
    ic_cdk::println!("EMERGENCY: System resumed by {} - {}", caller, reason);
    
    Ok(())
}

#[update]
// 设置维护模式
pub fn set_maintenance_mode(duration_hours: u64, reason: String) -> Result<()> {
    // 验证调用者权限
    let caller = ic_api::caller().to_string();
    if !is_emergency_operator(&caller) {
        return Err(Error::PermissionDenied("Not authorized for emergency operations".to_string()));
    }
    
    // 验证参数
    if reason.trim().is_empty() {
        return Err(Error::InvalidArgument("Maintenance reason cannot be empty".to_string()));
    }
    
    if duration_hours == 0 || duration_hours > 72 {
        return Err(Error::InvalidArgument("Maintenance duration must be 1-72 hours".to_string()));
    }
    
    // 计算自动恢复时间
    let auto_resume_time = ic_api::time() + (duration_hours * 60 * 60 * 1_000_000_000);
    
    // 设置维护模式
    EMERGENCY_CONTROLS.with_borrow_mut(|controls| {
        controls.state = EmergencyState::MaintenanceMode;
        controls.reason = reason.clone();
        controls.timestamp = ic_api::time();
        controls.operator = caller.clone();
        controls.auto_resume_time = Some(auto_resume_time);
    });
    
    // 记录维护模式事件
    ic_cdk::println!("MAINTENANCE: Mode activated by {} for {} hours - {}", 
                     caller, duration_hours, reason);
    
    Ok(())
}

#[update]
// 添加紧急操作员
pub fn add_emergency_operator(operator: String) -> Result<()> {
    // 验证调用者是否为控制者
    let caller = ic_api::caller();
    if !ic_api::is_controller(&caller) {
        return Err(Error::PermissionDenied("Only controllers can add emergency operators".to_string()));
    }
    
    // 验证操作员地址
    if operator.trim().is_empty() {
        return Err(Error::InvalidArgument("Operator address cannot be empty".to_string()));
    }
    
    // 添加操作员
    EMERGENCY_OPERATORS.with_borrow_mut(|operators| {
        operators.insert(operator.clone(), true);
    });
    
    ic_cdk::println!("Emergency operator added: {}", operator);
    
    Ok(())
}

#[update]
// 移除紧急操作员
pub fn remove_emergency_operator(operator: String) -> Result<()> {
    // 验证调用者是否为控制者
    let caller = ic_api::caller();
    if !ic_api::is_controller(&caller) {
        return Err(Error::PermissionDenied("Only controllers can remove emergency operators".to_string()));
    }
    
    // 移除操作员
    let removed = EMERGENCY_OPERATORS.with_borrow_mut(|operators| {
        operators.remove(&operator).is_some()
    });
    
    if removed {
        ic_cdk::println!("Emergency operator removed: {}", operator);
    }
    
    Ok(())
}

#[query]
// 获取所有紧急操作员
pub fn get_emergency_operators() -> Vec<String> {
    // 验证调用者是否为控制者
    let caller = ic_api::caller();
    if !ic_api::is_controller(&caller) {
        return vec![];
    }
    
    EMERGENCY_OPERATORS.with_borrow(|operators| {
        operators.keys().cloned().collect()
    })
}

// 检查是否为紧急操作员
fn is_emergency_operator(operator: &str) -> bool {
    // 控制者总是紧急操作员
    let caller_principal = match operator.parse() {
        Ok(principal) => principal,
        Err(_) => return false,
    };
    
    if ic_api::is_controller(&caller_principal) {
        return true;
    }
    
    // 检查是否在操作员列表中
    EMERGENCY_OPERATORS.with_borrow(|operators| {
        operators.get(operator).copied().unwrap_or(false)
    })
}

// 自动检查和恢复维护模式
pub fn check_auto_resume() {
    EMERGENCY_CONTROLS.with_borrow_mut(|controls| {
        if controls.state == EmergencyState::MaintenanceMode {
            if let Some(resume_time) = controls.auto_resume_time {
                let current_time = ic_api::time();
                if current_time >= resume_time {
                    controls.state = EmergencyState::Normal;
                    controls.reason = "Auto-resumed from maintenance mode".to_string();
                    controls.timestamp = current_time;
                    controls.operator = "system".to_string();
                    controls.auto_resume_time = None;
                    
                    ic_cdk::println!("MAINTENANCE: Auto-resumed from maintenance mode");
                }
            }
        }
    });
}

// 操作前检查宏
#[macro_export]
macro_rules! check_emergency_state {
    ($operation:expr) => {
        if !$crate::emergency::is_operation_allowed($operation.to_string()) {
            let state = $crate::emergency::get_emergency_state();
            return Err($crate::Error::SystemError(format!(
                "Operation '{}' is not allowed in current state: {:?} - {}",
                $operation, state.state, state.reason
            )));
        }
    };
}

// 心跳函数中调用的检查
#[ic_cdk_macros::heartbeat]
async fn emergency_heartbeat() {
    check_auto_resume();
}