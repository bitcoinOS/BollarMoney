//! 测试工具模块
//! 提供测试环境下的模拟函数和工具

#[cfg(test)]
pub mod mock {
    use candid::Principal;
    use std::cell::RefCell;
    use std::collections::HashMap;

    // 模拟的时间戳
    thread_local! {
        static MOCK_TIME: RefCell<u64> = RefCell::new(1_000_000_000_000_000_000); // 默认时间戳
        static MOCK_CALLER: RefCell<Principal> = RefCell::new(Principal::anonymous());
        static MOCK_CONTROLLERS: RefCell<Vec<Principal>> = RefCell::new(vec![Principal::anonymous()]);
    }

    /// 设置模拟时间
    pub fn set_time(time: u64) {
        MOCK_TIME.with(|t| *t.borrow_mut() = time);
    }

    /// 获取模拟时间
    pub fn time() -> u64 {
        MOCK_TIME.with(|t| *t.borrow())
    }

    /// 设置模拟调用者
    pub fn set_caller(caller: Principal) {
        MOCK_CALLER.with(|c| *c.borrow_mut() = caller);
    }

    /// 获取模拟调用者
    pub fn caller() -> Principal {
        MOCK_CALLER.with(|c| *c.borrow())
    }

    /// 设置模拟控制者列表
    pub fn set_controllers(controllers: Vec<Principal>) {
        MOCK_CONTROLLERS.with(|c| *c.borrow_mut() = controllers);
    }

    /// 检查是否为控制者
    pub fn is_controller(principal: &Principal) -> bool {
        MOCK_CONTROLLERS.with(|controllers| {
            controllers.borrow().contains(principal)
        })
    }

    /// 重置所有模拟状态
    pub fn reset() {
        set_time(1_000_000_000_000_000_000);
        set_caller(Principal::anonymous());
        set_controllers(vec![Principal::anonymous()]);
    }

    /// 创建测试用的 Principal
    pub fn test_principal(id: u8) -> Principal {
        let mut bytes = [0u8; 29];
        bytes[0] = id;
        Principal::from_slice(&bytes)
    }
}

#[cfg(test)]
pub use mock::*;