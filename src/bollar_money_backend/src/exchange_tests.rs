#[cfg(test)]
mod exchange_tests {
    use crate::types::{CoinMeta, Pool, PoolState, CoinId};
    use ree_types::{Pubkey, Utxo, CoinBalances, CoinBalance};
    
    // 创建测试池
    fn create_test_pool() -> Pool {
        // 创建 Bollar 代币元数据
        let meta = CoinMeta {
            id: CoinId::rune(72798, 1058),
            symbol: "BOLLAR".to_string(),
            min_amount: 1,
        };
        
        // 创建公钥 - 使用有效的 33 字节压缩公钥格式
        let pubkey_bytes = vec![
            0x02, // 压缩公钥前缀
            0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce, 0x87, 0x0b, 0x07,
            0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81, 0x5b, 0x16, 0xf8, 0x17, 0x98
        ];
        let pubkey = Pubkey::from_raw(pubkey_bytes).unwrap();
        let tweaked = pubkey.clone();
        
        // 创建池
        Pool {
            meta,
            pubkey,
            tweaked,
            addr: "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4".to_string(),
            states: vec![],
            collateral_ratio: 90,
            liquidation_threshold: 95,
        }
    }
    
    // 创建测试 UTXO
    fn create_test_utxo(sats: u64, rune_amount: u128) -> Utxo {
        let mut coins = CoinBalances::new();
        coins.add_coin(&CoinBalance {
            id: CoinId::rune(72798, 1058),
            value: rune_amount,
        });
        
        // 使用 Utxo 的 try_from 方法
        let outpoint = "0000000000000000000000000000000000000000000000000000000000000000:0";
        Utxo::try_from(outpoint, coins, sats).expect("Failed to create UTXO")
    }
    
    // 测试池状态管理
    #[test]
    fn test_pool_state_management() {
        let mut pool = create_test_pool();
        
        // 初始状态
        assert_eq!(pool.current_nonce(), 0);
        assert_eq!(pool.btc_balance(), 0);
        assert_eq!(pool.bollar_balance(), 0);
        
        // 添加状态
        let utxo = create_test_utxo(100000, 50000);
        let state = PoolState {
            id: None,
            nonce: 1,
            utxo: Some(utxo),
            btc_price: 3000000, // $30,000.00
        };
        
        pool.commit(state);
        
        // 验证状态
        assert_eq!(pool.current_nonce(), 1);
        assert_eq!(pool.btc_balance(), 100000);
        assert_eq!(pool.bollar_balance(), 50000);
        assert_eq!(pool.current_btc_price(), 3000000);
        
        // 测试计算最大 Bollar 铸造量
        let btc_amount = 100000; // 0.001 BTC
        let btc_price = 3000000; // $30,000.00
        let max_bollar = pool.calculate_max_bollar(btc_amount, btc_price);
        
        // 0.001 BTC @ $30,000.00 = $30.00
        // 90% 抵押率 = $27.00 Bollar
        // 由于 Bollar 是整数，所以应该是 27 Bollar
        assert_eq!(max_bollar, 27);
    }
    
    // 测试池地址生成
    #[test]
    fn test_pool_derivation_path() {
        let pool = create_test_pool();
        let path = pool.derivation_path();
        
        // 验证派生路径
        assert_eq!(path.len(), 1);
        assert_eq!(
            String::from_utf8(path[0].clone()).unwrap(),
            "rune72798:1058"
        );
    }
}