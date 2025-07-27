// e2e_tests.rs - 端到端测试
// 这个模块包含用户旅程和异常场景的端到端测试

#[cfg(test)]
mod tests {
    use crate::{
        lending::*,
    };
    use candid::Principal;

    // 测试用户 Principal
    #[allow(dead_code)]
    fn test_user() -> Principal {
        Principal::from_text("rdmx6-jaaaa-aaaah-qcaiq-cai").unwrap()
    }

    // 测试池地址
    fn test_pool_address() -> String {
        "bc1qtest123456789".to_string()
    }

    // 测试 PSBT 字符串
    fn test_psbt() -> String {
        "70736274ff01007d020000000100000000000000000000000000000000000000000000000000000000000000000000000000ffffffff01e8030000000000001976a914389ffce9cd9ae88dcc0631e88a821ffdbe9bfe2615".to_string()
    }

    // 模拟用户认证
    async fn authenticate_user() -> (String, String) {
        let address = "bc1qtest123456789".to_string();
        let signature = "test_signature".to_string();
        let message = "test_message".to_string();
        
        let auth_result = crate::auth::authenticate(address.clone(), signature, message).unwrap();
        let token = auth_result.token.unwrap();
        
        (address, token)
    }

    #[tokio::test]
    async fn test_user_journey() {
        // 测试完整用户旅程：认证 -> 抵押铸造 -> 还款赎回
        
        // 设置唯一的测试时间戳
        crate::test_utils::set_time(1_000_000_000_000_000_003);
        
        // 1. 用户认证
        let (_address, _token) = authenticate_user().await;
        assert!(crate::auth::is_authenticated());
        
        // 2. 设置 BTC 价格
        let btc_price = 3_000_000u64; // $30,000
        let _ = crate::oracle::mock_price_update(btc_price);
        
        // 3. 抵押 BTC 和铸造 Bollar
        let pool_address = test_pool_address();
        let btc_amount = 100_000_000u64; // 1 BTC
        
        // 初始化测试池
        crate::test_utils::init_test_pool(pool_address.clone());
        
        // 预抵押查询
        let deposit_offer = pre_deposit(pool_address.clone(), btc_amount).unwrap();
        let bollar_amount = deposit_offer.max_bollar_mint / 2; // 铸造一半的最大数量
        
        // 执行抵押和铸造
        let position_id = execute_deposit(
            pool_address.clone(),
            test_psbt(),
            bollar_amount
        ).await.unwrap();
        
        // 4. 查看用户头寸
        let caller = crate::ic_api::caller().to_string();
        let user_positions = crate::lending::get_user_positions(caller);
        assert!(!user_positions.is_empty(), "User should have positions");
        assert_eq!(user_positions[0].id, position_id);
        
        // 5. 还款 Bollar 和赎回 BTC
        let repay_amount = bollar_amount / 2; // 还款一半
        
        // 预还款查询
        let _repay_offer = pre_repay(position_id.clone(), repay_amount).unwrap();
        
        // 执行还款和赎回
        let _ = execute_repay(
            position_id.clone(),
            test_psbt()
        ).await.unwrap();
        
        // 6. 验证头寸更新
        let updated_position = crate::lending::get_position_details(position_id).unwrap();
        assert_eq!(updated_position.bollar_debt, bollar_amount - repay_amount);
        
        // 7. 查看协议指标
        let metrics = crate::lending::get_protocol_metrics();
        assert!(metrics.total_btc_locked > 0);
        assert!(metrics.total_bollar_supply > 0);
        
        // 8. 用户登出
        let _ = crate::auth::logout();
    }

    #[tokio::test]
    async fn test_price_volatility_scenario() {
        // 测试价格波动场景
        
        // 1. 用户认证
        let (_address, _) = authenticate_user().await;
        
        // 2. 设置初始 BTC 价格
        let initial_price = 3_000_000u64; // $30,000
        let _ = crate::oracle::mock_price_update(initial_price);
        
        // 3. 抵押 BTC 和铸造 Bollar
        let pool_address = test_pool_address();
        let btc_amount = 100_000_000u64; // 1 BTC
        
        // 初始化测试池
        crate::test_utils::init_test_pool(pool_address.clone());
        
        let deposit_offer = pre_deposit(pool_address.clone(), btc_amount).unwrap();
        let bollar_amount = deposit_offer.max_bollar_mint * 9 / 10; // 铸造90%的最大数量
        
        let position_id = execute_deposit(
            pool_address.clone(),
            test_psbt(),
            bollar_amount
        ).await.unwrap();
        
        // 4. 模拟价格下跌
        let crashed_price = 2_500_000u64; // $25,000 (价格下跌约17%)
        let _ = crate::oracle::mock_price_update(crashed_price);
        
        // 5. 检查头寸健康因子
        let position = crate::lending::get_position_details(position_id.clone()).unwrap();
        println!("Health factor after price drop: {}", position.health_factor);
        
        // 6. 模拟价格回升
        let recovered_price = 3_500_000u64; // $35,000 (价格回升)
        let _ = crate::oracle::mock_price_update(recovered_price);
        
        // 7. 再次检查头寸健康因子
        let position = crate::lending::get_position_details(position_id).unwrap();
        println!("Health factor after price recovery: {}", position.health_factor);
        
        // 8. 验证系统健康状态
        let health = crate::stability::get_system_health();
        assert!(health.system_collateral_ratio > 0);
    }

    #[tokio::test]
    async fn test_liquidation_scenario() {
        // 测试清算场景
        
        // 设置唯一的测试时间戳
        crate::test_utils::set_time(1_000_000_000_000_000_004);
        
        // 1. 设置初始 BTC 价格
        let initial_price = 3_000_000u64; // $30,000
        let _ = crate::oracle::mock_price_update(initial_price);
        
        // 2. 创建一个高风险头寸
        let pool_address = test_pool_address();
        let btc_amount = 100_000_000u64; // 1 BTC
        
        // 初始化测试池
        crate::test_utils::init_test_pool(pool_address.clone());
        
        let deposit_offer = pre_deposit(pool_address.clone(), btc_amount).unwrap();
        let bollar_amount = deposit_offer.max_bollar_mint; // 铸造最大数量
        
        let position_id = execute_deposit(
            pool_address.clone(),
            test_psbt(),
            bollar_amount
        ).await.unwrap();
        
        // 3. 模拟价格大幅下跌
        let crashed_price = 1_500_000u64; // $15,000 (价格下跌50%)
        let _ = crate::oracle::mock_price_update(crashed_price);
        
        // 4. 检查可清算头寸
        let liquidatable_positions = crate::liquidation::get_liquidatable_positions();
        assert!(!liquidatable_positions.is_empty(), "Should have liquidatable positions");
        
        // 5. 执行清算
        let _liquidation_offer = crate::liquidation::pre_liquidate(
            position_id.clone(),
            bollar_amount
        ).unwrap();
        
        let _ = crate::liquidation::execute_liquidate(
            position_id.clone(),
            test_psbt()
        ).await.unwrap();
        
        // 6. 验证头寸已被清算
        let position_result = crate::lending::get_position_details(position_id);
        assert!(position_result.is_err(), "Position should be liquidated");
    }

    #[tokio::test]
    async fn test_error_recovery_scenario() {
        // 测试错误恢复场景
        
        // 1. 设置 BTC 价格
        let btc_price = 3_000_000u64; // $30,000
        let _ = crate::oracle::mock_price_update(btc_price);
        
        // 2. 尝试使用无效参数
        let invalid_pool = "invalid_pool".to_string();
        let result = pre_deposit(invalid_pool, 100_000_000);
        assert!(result.is_err(), "Should reject invalid pool address");
        
        // 3. 使用有效参数重试
        let pool_address = test_pool_address();
        let btc_amount = 100_000_000u64; // 1 BTC
        
        // 初始化测试池
        crate::test_utils::init_test_pool(pool_address.clone());
        
        let deposit_offer = pre_deposit(pool_address.clone(), btc_amount);
        assert!(deposit_offer.is_ok(), "Should succeed with valid parameters");
        
        // 4. 尝试铸造超过最大数量的 Bollar
        let offer = deposit_offer.unwrap();
        let excessive_amount = offer.max_bollar_mint * 2; // 两倍的最大数量
        
        let result = execute_deposit(
            pool_address.clone(),
            test_psbt(),
            excessive_amount
        ).await;
        assert!(result.is_err(), "Should reject excessive mint amount");
        
        // 5. 使用有效金额重试
        let valid_amount = offer.max_bollar_mint / 2;
        let result = execute_deposit(
            pool_address.clone(),
            test_psbt(),
            valid_amount
        ).await;
        assert!(result.is_ok(), "Should succeed with valid amount");
    }

    #[tokio::test]
    async fn test_system_parameters_update() {
        // 测试系统参数更新
        
        // 初始化测试池
        let pool_address = test_pool_address();
        crate::test_utils::init_test_pool(pool_address.clone());
        
        // 1. 获取当前系统参数
        let _initial_params = crate::stability::get_system_parameters().unwrap();
        
        // 2. 更新抵押率
        let new_collateral_ratio = 70u8;
        let result = crate::stability::update_collateral_ratio(new_collateral_ratio);
        assert!(result.is_ok(), "Should be able to update collateral ratio");
        
        // 3. 更新清算阈值
        let new_liquidation_threshold = 85u8;
        let result = crate::stability::update_liquidation_threshold(new_liquidation_threshold);
        assert!(result.is_ok(), "Should be able to update liquidation threshold");
        
        // 4. 验证参数已更新
        let updated_params = crate::stability::get_system_parameters().unwrap();
        assert_eq!(updated_params.collateral_ratio, new_collateral_ratio);
        assert_eq!(updated_params.liquidation_threshold, new_liquidation_threshold);
        
        // 5. 创建头寸测试新参数
        let btc_amount = 100_000_000u64; // 1 BTC
        let btc_price = 3_000_000u64; // $30,000
        
        let _ = crate::oracle::mock_price_update(btc_price);
        
        let deposit_offer = pre_deposit(pool_address.clone(), btc_amount).unwrap();
        
        // 验证最大铸造数量反映了新的抵押率
        let expected_max = (btc_amount as u128) * (btc_price as u128) * (new_collateral_ratio as u128) / 100 / 100_000_000;
        assert_eq!(deposit_offer.max_bollar_mint, expected_max as u64);
    }

    #[tokio::test]
    async fn test_performance_under_load() {
        // 测试高负载下的性能
        
        // 设置唯一的测试时间戳
        crate::test_utils::set_time(1_000_000_000_000_000_005);
        
        // 1. 设置 BTC 价格
        let btc_price = 3_000_000u64; // $30,000
        let _ = crate::oracle::mock_price_update(btc_price);
        
        // 2. 创建多个头寸
        let pool_address = test_pool_address();
        let position_count = 10; // 创建10个头寸
        let mut position_ids = Vec::with_capacity(position_count);
        
        // 初始化测试池
        crate::test_utils::init_test_pool(pool_address.clone());
        
        for i in 0..position_count {
            // 设置不同的调用者以避免头寸 ID 冲突
            let test_principal = crate::test_utils::test_principal((i + 1) as u8);
            crate::test_utils::set_caller(test_principal);
            
            let btc_amount = 10_000_000u64 + (i as u64 * 1_000_000); // 0.1 BTC 到 0.19 BTC
            
            let deposit_offer = pre_deposit(pool_address.clone(), btc_amount).unwrap();
            let bollar_amount = deposit_offer.max_bollar_mint / 2;
            
            let position_id = execute_deposit(
                pool_address.clone(),
                test_psbt(),
                bollar_amount
            ).await.unwrap();
            
            position_ids.push(position_id);
        }
        
        // 3. 批量查询头寸
        let start_time = std::time::Instant::now();
        
        for position_id in &position_ids {
            let _ = crate::lending::get_position_details(position_id.clone()).unwrap();
        }
        
        let query_duration = start_time.elapsed();
        println!("Time to query {} positions: {:?}", position_count, query_duration);
        
        // 4. 模拟价格变化并更新所有头寸
        let new_price = 3_200_000u64; // $32,000
        let _ = crate::oracle::mock_price_update(new_price);
        
        // 5. 获取系统指标
        let metrics = crate::lending::get_protocol_metrics();
        assert_eq!(metrics.positions_count, position_count as u64);
    }
}