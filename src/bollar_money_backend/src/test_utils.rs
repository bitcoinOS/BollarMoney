//! 测试工具模块
//! 提供测试环境下的模拟函数和工具

#[cfg(test)]
pub mod mock {
    use candid::Principal;
    use std::cell::RefCell;
    // use std::collections::HashMap; // 暂时未使用

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
    #[allow(dead_code)]
    pub fn reset() {
        set_time(1_000_000_000_000_000_000);
        set_caller(Principal::anonymous());
        set_controllers(vec![Principal::anonymous()]);
        
        // 清理存储状态
        // 注意：由于 StableBTreeMap 的限制，我们不能直接清理存储
        // 在实际测试中，每个测试应该使用不同的键来避免冲突
    }

    /// 创建测试用的 Principal
    pub fn test_principal(id: u8) -> Principal {
        let mut bytes = [0u8; 29];
        bytes[0] = id;
        Principal::from_slice(&bytes)
    }

    /// 创建测试池
    pub fn create_test_pool(address: String) -> crate::types::Pool {
        use crate::types::{CoinMeta, Pool};
        use ree_types::Pubkey;
        
        // 创建 Bollar 代币元数据
        let meta = CoinMeta::bollar();
        
        // 创建公钥 - 使用有效的 33 字节压缩公钥格式
        let pubkey_bytes = vec![
            0x02, // 压缩公钥前缀
            0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce, 0x87, 0x0b, 0x07,
            0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81, 0x5b, 0x16, 0xf8, 0x17, 0x98
        ];
        let pubkey = Pubkey::from_raw(pubkey_bytes).unwrap();
        let tweaked = pubkey.clone();
        
        // 创建池
        Pool::new(
            meta,
            pubkey,
            tweaked,
            address,
            90, // 90% 抵押率
            95, // 95% 清算阈值
        )
    }

    /// 初始化测试池
    pub fn init_test_pool(address: String) {
        // 设置测试环境
        let test_principal = test_principal(1);
        set_caller(test_principal.clone());
        set_controllers(vec![test_principal]);
        
        let mut pool = create_test_pool(address);
        
        // 添加初始状态
        let initial_state = crate::types::PoolState {
            id: None,
            nonce: 0,
            utxo: Some(create_test_utxo(100_000_000, 50_000)), // 1 BTC, 50000 Bollar
            btc_price: 3_000_000, // $30,000
        };
        
        pool.commit(initial_state);
        crate::save_pool(pool);
    }

    /// 创建测试 UTXO
    fn create_test_utxo(sats: u64, rune_amount: u128) -> ree_types::Utxo {
        use ree_types::{CoinBalances, CoinBalance, CoinId};
        
        let mut coins = CoinBalances::new();
        coins.add_coin(&CoinBalance {
            id: CoinId::rune(72798, 1058),
            value: rune_amount,
        });
        
        // 使用 Utxo 的 try_from 方法
        let outpoint = "0000000000000000000000000000000000000000000000000000000000000000:0";
        ree_types::Utxo::try_from(outpoint, coins, sats).expect("Failed to create UTXO")
    }

    /// 模拟价格更新
    #[allow(dead_code)]
    pub fn mock_price_update(_price: u64) -> Result<(), String> {
        // 在测试环境中，我们直接设置价格
        // 实际实现中，这会通过 oracle 模块处理
        Ok(())
    }
}

#[cfg(test)]
pub use mock::*;