// auth.rs - 用户认证和钱包集成
// 这个模块实现比特币钱包集成和用户身份验证

use crate::{Error, LogLevel, Result, error::{log_error, catch_and_log}, types::*, ic_api};
use candid::{CandidType, Deserialize, Principal};
use ic_cdk_macros::{query, update};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;

// 会话有效期 (纳秒)
const SESSION_VALIDITY_PERIOD_NS: u64 = 24 * 60 * 60 * 1_000_000_000; // 24小时

// 用户会话数据
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct UserSession {
    pub user_address: String,        // 用户比特币地址
    pub principal: Principal,        // 用户 Principal
    pub created_at: u64,            // 创建时间戳 (纳秒)
    pub last_accessed: u64,         // 最后访问时间戳 (纳秒)
    pub token: String,              // 会话令牌
}

// 认证请求
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct AuthRequest {
    pub address: String,            // 比特币地址
    pub signature: String,          // 签名
    pub message: String,            // 签名的消息
}

thread_local! {
    // 用户会话存储 (Principal -> UserSession)
    static USER_SESSIONS: RefCell<HashMap<Principal, UserSession>> = RefCell::new(HashMap::new());
    
    // 地址到 Principal 的映射
    static ADDRESS_TO_PRINCIPAL: RefCell<HashMap<String, Principal>> = RefCell::new(HashMap::new());
}

#[update]
// 用户认证
pub fn authenticate(address: String, signature: String, message: String) -> Result<AuthResult> {
    // 使用 catch_and_log 包装操作
    catch_and_log(
        || {
            // 验证参数
            if address.is_empty() || signature.is_empty() || message.is_empty() {
                return Err(Error::InvalidArgument("认证参数不能为空".to_string()));
            }
            
            // 获取调用者 Principal
            let caller = ic_api::caller();
            
            // 验证签名 (简化实现，实际应该验证比特币签名)
            if !verify_bitcoin_signature(&address, &signature, &message) {
                return Err(Error::AuthenticationFailed);
            }
            
            // 生成会话令牌
            let token = generate_session_token(&address, caller);
            
            // 创建用户会话
            let now = ic_api::time();
            let session = UserSession {
                user_address: address.clone(),
                principal: caller,
                created_at: now,
                last_accessed: now,
                token: token.clone(),
            };
            
            // 存储会话
            USER_SESSIONS.with_borrow_mut(|sessions| {
                sessions.insert(caller, session);
            });
            
            // 存储地址映射
            ADDRESS_TO_PRINCIPAL.with_borrow_mut(|mapping| {
                mapping.insert(address.clone(), caller);
            });
            
            // 记录认证成功
            ic_cdk::println!("User authenticated: address={}, principal={}", address, caller);
            
            Ok(AuthResult {
                success: true,
                message: "认证成功".to_string(),
                token: Some(token),
            })
        },
        LogLevel::Error,
        &format!("authenticate: 用户认证失败, address={}", address)
    )
}

#[query]
// 获取当前用户会话
pub fn get_current_session() -> Result<UserSession> {
    // 使用 catch_and_log 包装操作
    catch_and_log(
        || {
            let caller = ic_api::caller();
            
            // 获取用户会话
            let session = USER_SESSIONS.with_borrow(|sessions| {
                sessions.get(&caller).cloned()
            }).ok_or(Error::AuthenticationFailed)?;
            
            // 检查会话是否过期
            let now = ic_api::time();
            if now - session.created_at > SESSION_VALIDITY_PERIOD_NS {
                return Err(Error::AuthenticationFailed);
            }
            
            Ok(session)
        },
        LogLevel::Debug,
        "get_current_session: 获取当前用户会话失败"
    )
}

#[update]
// 刷新用户会话
pub fn refresh_session() -> Result<AuthResult> {
    // 使用 catch_and_log 包装操作
    catch_and_log(
        || {
            let caller = ic_api::caller();
            
            // 获取现有会话
            let mut session = USER_SESSIONS.with_borrow(|sessions| {
                sessions.get(&caller).cloned()
            }).ok_or(Error::AuthenticationFailed)?;
            
            // 检查会话是否过期
            let now = ic_api::time();
            if now - session.created_at > SESSION_VALIDITY_PERIOD_NS {
                return Err(Error::AuthenticationFailed);
            }
            
            // 更新最后访问时间
            session.last_accessed = now;
            
            // 生成新的令牌
            session.token = generate_session_token(&session.user_address, caller);
            
            // 更新会话
            USER_SESSIONS.with_borrow_mut(|sessions| {
                sessions.insert(caller, session.clone());
            });
            
            Ok(AuthResult {
                success: true,
                message: "会话刷新成功".to_string(),
                token: Some(session.token),
            })
        },
        LogLevel::Error,
        "refresh_session: 刷新用户会话失败"
    )
}

#[update]
// 用户登出
pub fn logout() -> Result<bool> {
    // 使用 catch_and_log 包装操作
    catch_and_log(
        || {
            let caller = ic_api::caller();
            
            // 获取用户会话以获取地址
            let session = USER_SESSIONS.with_borrow(|sessions| {
                sessions.get(&caller).cloned()
            });
            
            // 删除会话
            let removed = USER_SESSIONS.with_borrow_mut(|sessions| {
                sessions.remove(&caller).is_some()
            });
            
            // 删除地址映射
            if let Some(session) = session {
                ADDRESS_TO_PRINCIPAL.with_borrow_mut(|mapping| {
                    mapping.remove(&session.user_address);
                });
                
                ic_cdk::println!("User logged out: address={}, principal={}", session.user_address, caller);
            }
            
            Ok(removed)
        },
        LogLevel::Debug,
        "logout: 用户登出失败"
    )
}

#[query]
// 检查用户是否已认证
pub fn is_authenticated() -> bool {
    let caller = ic_api::caller();
    
    // 检查是否有有效会话
    USER_SESSIONS.with_borrow(|sessions| {
        if let Some(session) = sessions.get(&caller) {
            let now = ic_api::time();
            now - session.created_at <= SESSION_VALIDITY_PERIOD_NS
        } else {
            false
        }
    })
}

#[query]
// 根据地址获取 Principal
pub fn get_principal_by_address(address: String) -> Option<Principal> {
    ADDRESS_TO_PRINCIPAL.with_borrow(|mapping| {
        mapping.get(&address).cloned()
    })
}

#[query]
// 获取所有活跃会话 (仅管理员)
pub fn get_active_sessions() -> Result<Vec<UserSession>> {
    // 使用 catch_and_log 包装操作
    catch_and_log(
        || {
            // 验证调用者是否为控制者
            let caller = ic_api::caller();
            if !ic_api::is_controller(&caller) {
                return Err(Error::PermissionDenied("Not authorized".to_string()));
            }
            
            let now = ic_api::time();
            
            // 获取所有活跃会话
            let active_sessions = USER_SESSIONS.with_borrow(|sessions| {
                sessions.values()
                    .filter(|session| now - session.created_at <= SESSION_VALIDITY_PERIOD_NS)
                    .cloned()
                    .collect()
            });
            
            Ok(active_sessions)
        },
        LogLevel::Debug,
        "get_active_sessions: 获取活跃会话失败"
    )
}

// 验证比特币签名 (简化实现)
fn verify_bitcoin_signature(address: &str, signature: &str, message: &str) -> bool {
    // 在实际实现中，这里应该验证比特币签名
    // 这里简化处理，只检查基本格式
    !address.is_empty() && !signature.is_empty() && !message.is_empty() && signature.len() > 10
}

// 生成会话令牌
fn generate_session_token(address: &str, principal: Principal) -> String {
    // 在实际实现中，应该使用更安全的令牌生成方法
    let timestamp = ic_api::time();
    format!("{}:{}:{}", address, principal.to_text(), timestamp)
}

// 清理过期会话的定时任务
#[update]
pub fn cleanup_expired_sessions() -> u64 {
    let now = ic_api::time();
    let mut removed_count = 0;
    
    // 收集过期的会话
    let expired_principals: Vec<Principal> = USER_SESSIONS.with_borrow(|sessions| {
        sessions.iter()
            .filter(|(_, session)| now - session.created_at > SESSION_VALIDITY_PERIOD_NS)
            .map(|(principal, _)| *principal)
            .collect()
    });
    
    // 删除过期会话
    for principal in expired_principals {
        // 获取会话以获取地址
        let session = USER_SESSIONS.with_borrow(|sessions| {
            sessions.get(&principal).cloned()
        });
        
        // 删除会话
        USER_SESSIONS.with_borrow_mut(|sessions| {
            sessions.remove(&principal);
        });
        
        // 删除地址映射
        if let Some(session) = session {
            ADDRESS_TO_PRINCIPAL.with_borrow_mut(|mapping| {
                mapping.remove(&session.user_address);
            });
        }
        
        removed_count += 1;
    }
    
    if removed_count > 0 {
        ic_cdk::println!("Cleaned up {} expired sessions", removed_count);
    }
    
    removed_count
}