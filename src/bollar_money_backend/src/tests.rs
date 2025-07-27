#[cfg(test)]
mod tests {
    use crate::types::{Pool, Position, PoolState, CoinMeta, calculate_health_factor};
    use ree_types::{Pubkey, CoinId};
    
    // 测试 Pool 结构
    #[test]
    fn test_pool_creation() {
        let meta = CoinMeta {
            id: CoinId::rune(72798, 1058),
            symbol: "BOLLAR".to_string(),
            min_amount: 1,
        };
        
        // 创建一个简单的 Pubkey
        let pubkey_bytes = vec![0x02, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce, 0x87, 0x0b, 0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81, 0x5b, 0x16, 0xf8, 0x17, 0x98];
        let pubkey = Pubkey::from_raw(pubkey_bytes).unwrap();
        let tweaked = pubkey.clone();
        let addr = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4".to_string();
        
        let pool = Pool::new(
            meta.clone(),
            pubkey.clone(),
            tweaked.clone(),
            addr.clone(),
            75, // 75% 抵押率
            80, // 80% 清算阈值
        );
        
        assert_eq!(pool.meta.symbol, "BOLLAR");
        assert_eq!(pool.addr, addr);
        assert_eq!(pool.collateral_ratio, 75);
        assert_eq!(pool.liquidation_threshold, 80);
        assert!(pool.states.is_empty());
    }
    
    // 测试 Pool 状态管理
    #[test]
    fn test_pool_state_management() {
        let meta = CoinMeta::bollar();
        // 创建一个简单的 Pubkey
        let pubkey_bytes = vec![0x02, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce, 0x87, 0x0b, 0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81, 0x5b, 0x16, 0xf8, 0x17, 0x98];
        let pubkey = Pubkey::from_raw(pubkey_bytes).unwrap();
        let tweaked = pubkey.clone();
        let addr = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4".to_string();
        
        let mut pool = Pool::new(
            meta.clone(),
            pubkey.clone(),
            tweaked.clone(),
            addr.clone(),
            75,
            80,
        );
        
        // 初始状态
        assert_eq!(pool.current_nonce(), 0);
        assert_eq!(pool.btc_balance(), 0);
        
        // 添加状态
        let state1 = PoolState {
            id: None,
            nonce: 1,
            utxo: None,
            btc_price: 3000000, // $30,000.00
        };
        
        pool.commit(state1);
        
        assert_eq!(pool.current_nonce(), 1);
        assert_eq!(pool.current_btc_price(), 3000000);
        
        // 更新抵押率
        pool.update_collateral_ratio(80);
        assert_eq!(pool.collateral_ratio, 80);
        
        // 更新清算阈值
        pool.update_liquidation_threshold(85);
        assert_eq!(pool.liquidation_threshold, 85);
    }
    
    // 测试 Position 结构
    #[test]
    fn test_position_creation() {
        let position = Position::new(
            "position1".to_string(),
            "user1".to_string(),
            100000, // 0.001 BTC
            75000,  // 75000 Bollar
            3000000, // $30,000.00
        );
        
        assert_eq!(position.id, "position1");
        assert_eq!(position.owner, "user1");
        assert_eq!(position.btc_collateral, 100000);
        assert_eq!(position.bollar_debt, 75000);
        
        // 验证健康因子计算
        let expected_health_factor = calculate_health_factor(100000, 75000, 3000000);
        assert_eq!(position.health_factor, expected_health_factor);
    }
    
    // 测试健康因子计算
    #[test]
    fn test_health_factor_calculation() {
        // 0.001 BTC @ $30,000.00 = $30.00 = 3000 cents
        // 债务 2250 cents
        // 健康因子 = (3000 / 2250) * 100 = 133.33...
        let health_factor = calculate_health_factor(100000, 2250, 3000000);
        assert_eq!(health_factor, 133);
        
        // 无债务情况
        let health_factor = calculate_health_factor(100000, 0, 3000000);
        assert_eq!(health_factor, u64::MAX);
        
        // 价格下跌 50%，健康因子应该减半
        let health_factor = calculate_health_factor(100000, 2250, 1500000);
        assert_eq!(health_factor, 66);
    }
    
    // 测试头寸可清算状态
    #[test]
    fn test_position_liquidatable() {
        // 创建一个健康因子为 133 的头寸
        let mut position = Position::new(
            "position1".to_string(),
            "user1".to_string(),
            100000, // 0.001 BTC
            2250, // 2250 cents Bollar
            3000000, // $30,000.00
        );
        
        // 清算阈值为 120，头寸不可清算
        assert!(!position.is_liquidatable(120));
        
        // 清算阈值为 140，头寸可清算
        assert!(position.is_liquidatable(140));
        
        // 价格下跌，更新头寸
        position.update(100000, 2250, 1500000);
        
        // 健康因子应该减半
        assert_eq!(position.health_factor, 66);
        
        // 清算阈值为 70，头寸可清算
        assert!(position.is_liquidatable(70));
    }
}