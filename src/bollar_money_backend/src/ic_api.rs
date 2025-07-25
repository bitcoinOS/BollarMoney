//! IC API 抽象层
//! 在测试环境中使用模拟函数，在生产环境中使用真实的 IC API

use candid::Principal;

#[cfg(not(test))]
pub fn caller() -> Principal {
    ic_cdk::api::caller()
}

#[cfg(not(test))]
pub fn time() -> u64 {
    ic_cdk::api::time()
}

#[cfg(not(test))]
pub fn is_controller(principal: &Principal) -> bool {
    ic_cdk::api::is_controller(principal)
}

#[cfg(test)]
pub fn caller() -> Principal {
    crate::test_utils::mock::caller()
}

#[cfg(test)]
pub fn time() -> u64 {
    crate::test_utils::mock::time()
}

#[cfg(test)]
pub fn is_controller(principal: &Principal) -> bool {
    crate::test_utils::mock::is_controller(principal)
}