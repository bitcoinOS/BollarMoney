mod exchange;
mod lending;
mod pool;
mod oracle;
mod liquidation;
mod reorg;
mod types;
mod tests;
mod error;
mod error_tests;
mod exchange_tests;
mod stability;
mod auth;
mod integration_tests;
mod e2e_tests;
mod ic_api;

#[cfg(test)]
mod test_utils;

// 注释掉暂时未使用的类型导入
// use types::{
//     DepositOffer, RepayOffer, LiquidationOffer,
//     ProtocolMetrics, AuthResult
// };

use ic_stable_structures::{
    DefaultMemoryImpl, StableBTreeMap,
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
};
use types::{Pool, Position, TxRecord};
use std::cell::RefCell;
use std::collections::HashSet;
pub use error::{Error, LogLevel, Result};

// REE 池密钥名称
const SCHNORR_KEY_NAME: &str = "bollar_key_1";

// 使用 error.rs 中定义的 Error 类型

type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    // 内存管理器
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    // 资金池存储
    static POOLS: RefCell<StableBTreeMap<String, Pool, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))),
        )
    );

    // 用户头寸存储
    static POSITIONS: RefCell<StableBTreeMap<String, Position, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))),
        )
    );

    // 区块存储
    static BLOCKS: RefCell<StableBTreeMap<u32, types::NewBlockInfo, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2))),
        )
    );

    // 交易记录存储
    static TX_RECORDS: RefCell<StableBTreeMap<(types::Txid, bool), TxRecord, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3))),
        )
    );
    
    // 正在执行交易的池
    static EXECUTING_POOLS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

// 获取所有资金池
pub(crate) fn get_pools() -> Vec<Pool> {
    POOLS.with_borrow(|p| p.iter().map(|p| p.1.clone()).collect::<Vec<_>>())
}

// 获取指定资金池
pub(crate) fn get_pool(addr: &String) -> Option<Pool> {
    POOLS.with_borrow(|p| p.get(addr))
}

// 保存资金池
pub(crate) fn save_pool(pool: Pool) {
    POOLS.with_borrow_mut(|p| {
        p.insert(pool.addr.clone(), pool);
    });
}

// 获取所有头寸
pub(crate) fn get_positions() -> Vec<Position> {
    POSITIONS.with_borrow(|p| p.iter().map(|p| p.1.clone()).collect::<Vec<_>>())
}

// 获取指定头寸
pub(crate) fn get_position(position_id: &String) -> Option<Position> {
    POSITIONS.with_borrow(|p| p.get(position_id))
}

// 获取用户的所有头寸
pub(crate) fn get_user_positions(user: &String) -> Vec<Position> {
    POSITIONS.with_borrow(|p| {
        p.iter()
            .filter(|(_, pos)| pos.owner == *user)
            .map(|(_, pos)| pos.clone())
            .collect()
    })
}

// 保存头寸
pub(crate) fn save_position(position: Position) {
    POSITIONS.with_borrow_mut(|p| {
        p.insert(position.id.clone(), position);
    });
}

// 删除头寸
pub(crate) fn delete_position(position_id: &String) {
    POSITIONS.with_borrow_mut(|p| {
        p.remove(position_id);
    });
}

// 保存交易记录
#[allow(dead_code)]
pub(crate) fn save_tx_record(txid: types::Txid, confirmed: bool, record: TxRecord) {
    TX_RECORDS.with_borrow_mut(|t| {
        t.insert((txid, confirmed), record);
    });
}

// 获取交易记录
#[allow(dead_code)]
pub(crate) fn get_tx_record(txid: &types::Txid, confirmed: bool) -> Option<TxRecord> {
    TX_RECORDS.with_borrow(|t| t.get(&(*txid, confirmed)))
}

// 删除交易记录
#[allow(dead_code)]
pub(crate) fn delete_tx_record(txid: &types::Txid, confirmed: bool) {
    TX_RECORDS.with_borrow_mut(|t| {
        t.remove(&(*txid, confirmed));
    });
}

// 交易执行锁，防止同一个池的并发交易
#[must_use]
pub struct ExecuteTxGuard(String);

impl ExecuteTxGuard {
    pub fn new(pool_address: String) -> Option<Self> {
        EXECUTING_POOLS.with(|executing_pools| {
            if executing_pools.borrow().contains(&pool_address) {
                return None;
            }
            executing_pools.borrow_mut().insert(pool_address.clone());
            return Some(ExecuteTxGuard(pool_address));
        })
    }
}

impl Drop for ExecuteTxGuard {
    fn drop(&mut self) {
        EXECUTING_POOLS.with_borrow_mut(|executing_pools| {
            executing_pools.remove(&self.0);
        });
    }
}

// 导出 Candid 接口
// 暂时注释掉，等待修复
// ic_cdk::export_candid!();