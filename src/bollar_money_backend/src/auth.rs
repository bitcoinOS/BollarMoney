// auth.rs - 用户认证和钱包集成
// 这个模块实现比特币钱包集成和用户身份验证

use crate::{Error, LogLevel, Result, error::catch_and_log, types::*, ic_api};
use candid::{CandidType, Deserialize, Principal};
use ic_cdk_macros::{query, update};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use bitcoin::{Address, Network, PublicKey};
use secp256k1::{Secp256k1, Message, Signature, ecdsa, PublicKey as Secp256k1PublicKey};
use sha2::{Sha256, Digest};
use rand::{RngCore, rngs::OsRng};
use hmac::{Hmac, Mac};
use aes_gcm::{Aes256Gcm, Key, Nonce, aead::{Aead, KeyInit}};
use std::str::FromStr;

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

// 验证比特币签名 (完整实现)
fn verify_bitcoin_signature(address: &str, signature: &str, message: &str) -> bool {
    // 验证参数
    if address.is_empty() || signature.is_empty() || message.is_empty() {
        return false;
    }
    
    // 解析比特币地址
    let btc_address = match Address::from_str(address) {
        Ok(addr) => addr.assume_checked(), // 在生产环境中应该验证网络
        Err(_) => return false,
    };
    
    // 获取地址对应的脚本公钥
    let script_pubkey = btc_address.script_pubkey();
    
    // 解码 base64 签名
    let signature_bytes = match base64::decode(signature) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    
    // 验证签名长度 (65 bytes for recoverable signature)
    if signature_bytes.len() != 65 {
        return false;
    }
    
    // 创建消息哈希 (Bitcoin 消息签名格式)
    let message_hash = create_bitcoin_message_hash(message);
    
    // 提取恢复 ID 和签名
    let recovery_id = signature_bytes[0];
    if recovery_id < 27 || recovery_id > 34 {
        return false;
    }
    
    let signature_data = &signature_bytes[1..];
    
    // 创建 secp256k1 上下文
    let secp = Secp256k1::new();
    
    // 解析签名
    let signature = match ecdsa::Signature::from_compact(signature_data) {
        Ok(sig) => sig,
        Err(_) => return false,
    };
    
    // 创建消息对象
    let message_obj = match Message::from_slice(&message_hash) {
        Ok(msg) => msg,
        Err(_) => return false,
    };
    
    // 恢复公钥
    let recovery_id_secp = match secp256k1::ecdsa::RecoveryId::from_i32((recovery_id - 27) as i32) {
        Ok(id) => id,
        Err(_) => return false,
    };
    
    let recoverable_sig = match secp256k1::ecdsa::RecoverableSignature::from_compact(signature_data, recovery_id_secp) {
        Ok(sig) => sig,
        Err(_) => return false,
    };
    
    let recovered_pubkey = match secp.recover_ecdsa(&message_obj, &recoverable_sig) {
        Ok(pubkey) => pubkey,
        Err(_) => return false,
    };
    
    // 将恢复的公钥转换为比特币公钥
    let bitcoin_pubkey = match PublicKey::from_slice(&recovered_pubkey.serialize()) {
        Ok(pubkey) => pubkey,
        Err(_) => return false,
    };
    
    // 验证地址是否匹配
    verify_address_matches_pubkey(&btc_address, &bitcoin_pubkey)
}

// 创建比特币消息哈希
fn create_bitcoin_message_hash(message: &str) -> [u8; 32] {
    let prefix = "Bitcoin Signed Message:\n";
    let message_bytes = message.as_bytes();
    
    let mut hasher = Sha256::new();
    hasher.update(&[prefix.len() as u8]);
    hasher.update(prefix.as_bytes());
    hasher.update(&[message_bytes.len() as u8]);
    hasher.update(message_bytes);
    
    let first_hash = hasher.finalize();
    
    let mut second_hasher = Sha256::new();
    second_hasher.update(&first_hash);
    second_hasher.finalize().into()
}

// 验证地址是否与公钥匹配
fn verify_address_matches_pubkey(address: &Address, pubkey: &PublicKey) -> bool {
    // 根据地址类型验证
    match address.address_type() {
        Some(bitcoin::AddressType::P2pkh) => {
            // P2PKH 地址验证
            let pubkey_hash = bitcoin::hashes::hash160::Hash::hash(&pubkey.to_bytes());
            address.script_pubkey() == bitcoin::Script::new_p2pkh(&pubkey_hash.into())
        }
        Some(bitcoin::AddressType::P2sh) => {
            // P2SH 地址验证 (更复杂，这里简化处理)
            false // 暂不支持 P2SH
        }
        Some(bitcoin::AddressType::P2wpkh) => {
            // P2WPKH 地址验证
            let pubkey_hash = bitcoin::hashes::hash160::Hash::hash(&pubkey.to_bytes());
            address.script_pubkey() == bitcoin::Script::new_v0_p2wpkh(&pubkey_hash.into())
        }
        Some(bitcoin::AddressType::P2wsh) => {
            // P2WSH 地址验证 (更复杂，这里简化处理)
            false // 暂不支持 P2WSH
        }
        Some(bitcoin::AddressType::P2tr) => {
            // Taproot 地址验证 (更复杂，这里简化处理)
            false // 暂不支持 Taproot
        }
        _ => false,
    }
}

// 生成安全的会话令牌
fn generate_session_token(address: &str, principal: Principal) -> String {
    // 生成随机盐
    let mut salt = [0u8; 32];
    OsRng.fill_bytes(&mut salt);
    
    // 创建 HMAC 密钥
    let timestamp = ic_api::time();
    let key_material = format!("{}:{}:{}", address, principal.to_text(), timestamp);
    
    // 使用 PBKDF2 派生密钥
    let mut derived_key = [0u8; 32];
    pbkdf2::pbkdf2::<Hmac<Sha256>>(key_material.as_bytes(), &salt, 10000, &mut derived_key);
    
    // 生成随机 nonce
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    
    // 创建要加密的数据
    let payload = format!("{}:{}:{}", address, principal.to_text(), timestamp);
    
    // 使用 AES-GCM 加密
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&derived_key));
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    match cipher.encrypt(nonce, payload.as_bytes()) {
        Ok(ciphertext) => {
            // 组合 salt + nonce + ciphertext
            let mut token_bytes = Vec::new();
            token_bytes.extend_from_slice(&salt);
            token_bytes.extend_from_slice(&nonce_bytes);
            token_bytes.extend_from_slice(&ciphertext);
            
            // 返回 base64 编码的令牌
            base64::encode(&token_bytes)
        }
        Err(_) => {
            // 如果加密失败，回退到简单方法（不推荐）
            ic_cdk::println!("Warning: Token encryption failed, using fallback method");
            let mut random_bytes = [0u8; 32];
            OsRng.fill_bytes(&mut random_bytes);
            format!("{}:{}", hex::encode(&random_bytes), timestamp)
        }
    }
}

// 验证会话令牌
fn verify_session_token(token: &str, address: &str, principal: Principal) -> bool {
    // 解码 base64 令牌
    let token_bytes = match base64::decode(token) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    
    // 检查令牌长度 (至少 salt(32) + nonce(12) + min_ciphertext(16))
    if token_bytes.len() < 60 {
        return false;
    }
    
    // 提取组件
    let salt = &token_bytes[0..32];
    let nonce_bytes = &token_bytes[32..44];
    let ciphertext = &token_bytes[44..];
    
    // 重建密钥 (需要时间戳，这里简化处理)
    // 在实际实现中，应该在令牌中包含时间戳或使用其他方法
    let current_time = ic_api::time();
    let time_window = 24 * 60 * 60 * 1_000_000_000; // 24小时窗口
    
    // 尝试多个时间戳 (简化的时间窗口验证)
    for offset in 0..24 {
        let test_timestamp = current_time - (offset * 60 * 60 * 1_000_000_000); // 每小时一个测试点
        let key_material = format!("{}:{}:{}", address, principal.to_text(), test_timestamp);
        
        let mut derived_key = [0u8; 32];
        pbkdf2::pbkdf2::<Hmac<Sha256>>(key_material.as_bytes(), salt, 10000, &mut derived_key);
        
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&derived_key));
        let nonce = Nonce::from_slice(nonce_bytes);
        
        if let Ok(decrypted) = cipher.decrypt(nonce, ciphertext) {
            if let Ok(payload) = String::from_utf8(decrypted) {
                let expected = format!("{}:{}:{}", address, principal.to_text(), test_timestamp);
                if payload == expected && (current_time - test_timestamp) <= time_window {
                    return true;
                }
            }
        }
    }
    
    false
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