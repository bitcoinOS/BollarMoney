// pool.rs - 资金池管理
// 这个模块实现资金池的创建、更新和查询功能

use crate::{Error, LogLevel, Result, error::log_error, types::*};

impl Pool {
    // 创建新的资金池
    #[allow(dead_code)]
    pub fn new(
        meta: CoinMeta,
        pubkey: Pubkey,
        tweaked: Pubkey,
        addr: String,
        collateral_ratio: u8,
        liquidation_threshold: u8,
    ) -> Self {
        Self {
            states: vec![],
            meta,
            pubkey,
            tweaked,
            addr,
            collateral_ratio,
            liquidation_threshold,
        }
    }
    
    // 获取当前池状态
    pub fn current_state(&self) -> Option<&PoolState> {
        self.states.last()
    }
    
    // 获取当前 BTC 余额
    pub fn btc_balance(&self) -> u64 {
        self.current_state()
            .and_then(|s| s.utxo.as_ref())
            .map(|u| u.sats)
            .unwrap_or(0)
    }
    
    // 获取当前 Bollar 余额
    pub fn bollar_balance(&self) -> u128 {
        self.current_state()
            .and_then(|s| s.utxo.as_ref())
            .map(|u| u.coins.value_of(&self.meta.id))
            .unwrap_or(0)
    }
    
    // 获取当前 nonce
    pub fn current_nonce(&self) -> u64 {
        self.current_state().map(|s| s.nonce).unwrap_or(0)
    }
    
    // 获取当前 BTC 价格
    #[allow(dead_code)]
    pub fn current_btc_price(&self) -> u64 {
        self.current_state().map(|s| s.btc_price).unwrap_or(0)
    }
    
    // 更新抵押率
    pub fn update_collateral_ratio(&mut self, new_ratio: u8) {
        self.collateral_ratio = new_ratio;
    }
    
    // 更新清算阈值
    pub fn update_liquidation_threshold(&mut self, new_threshold: u8) {
        self.liquidation_threshold = new_threshold;
    }

    // 获取池的派生路径
    pub fn derivation_path(&self) -> Vec<Vec<u8>> {
        vec![format!("rune{}", self.meta.id.to_string()).as_bytes().to_vec()]
    }

    // 验证抵押交易
    pub fn validate_deposit(
        &self,
        _txid: Txid,
        _nonce: u64,
        _pool_utxo_spent: Vec<String>,
        _pool_utxo_received: Vec<Utxo>,
        _input_coins: Vec<InputCoin>,
        _output_coins: Vec<OutputCoin>,
        _bollar_mint_amount: u64,
    ) -> Result<(PoolState, Option<Utxo>)> {
        // 使用错误日志记录
        log_error(
            LogLevel::Warning,
            &Error::InvalidPool,
            Some(&format!("validate_deposit: 池验证失败, addr={}", self.addr))
        );
        
        // 待实现
        Err(Error::InvalidPool)
    }

    // 验证还款交易
    pub fn validate_repay(
        &self,
        _txid: Txid,
        _nonce: u64,
        _pool_utxo_spent: Vec<String>,
        _pool_utxo_received: Vec<Utxo>,
        _input_coins: Vec<InputCoin>,
        _output_coins: Vec<OutputCoin>,
        _position_id: String,
    ) -> Result<(PoolState, Utxo)> {
        // 使用 catch_and_log 包装操作
        crate::error::catch_and_log(
            || {
                // 检查池状态
                if self.states.is_empty() {
                    return Err(Error::EmptyPool);
                }
                
                // 检查 nonce
                let current_nonce = self.current_nonce();
                if _nonce != current_nonce {
                    return Err(Error::PoolStateExpired(current_nonce));
                }
                
                // 待实现
                Err(Error::InvalidPool)
            },
            LogLevel::Error,
            &format!("validate_repay: 池验证失败, addr={}", self.addr)
        )
    }

    // 计算可铸造的最大 Bollar 数量
    pub fn calculate_max_bollar(&self, btc_amount: u64, btc_price: u64) -> u64 {
        // 根据抵押率计算可铸造的最大 Bollar 数量
        // 例如，如果抵押率为 90%，那么可铸造的 Bollar 数量为 BTC 价值的 90%
        // btc_price 是以 USD cents 为单位，所以需要除以 100_000_000 (satoshis) 再除以 100 (cents to dollars)
        let btc_value_cents = (btc_amount as u128) * (btc_price as u128) / 100_000_000;
        let max_bollar = btc_value_cents * (self.collateral_ratio as u128) / 100;
        max_bollar.try_into().unwrap_or(0)
    }

    // 回滚池状态
    pub fn rollback(&mut self, txid: Txid) -> Result<()> {
        // 使用 catch_and_log 包装操作
        crate::error::catch_and_log(
            || {
                let idx = self
                    .states
                    .iter()
                    .position(|state| state.id == Some(txid))
                    .ok_or(Error::InvalidState("txid not found".to_string()))?;
                
                if idx == 0 {
                    self.states.clear();
                    return Ok(());
                }
                
                self.states.truncate(idx);
                Ok(())
            },
            LogLevel::Warning,
            &format!("rollback: 回滚池状态, addr={}, txid={}", self.addr, txid)
        )
    }

    // 确认交易，使其状态成为新的基础状态
    pub fn finalize(&mut self, txid: Txid) -> Result<()> {
        // 使用 catch_and_log 包装操作
        crate::error::catch_and_log(
            || {
                let idx = self
                    .states
                    .iter()
                    .position(|state| state.id == Some(txid))
                    .ok_or(Error::InvalidState("txid not found".to_string()))?;
                
                if idx == 0 {
                    return Ok(());
                }
                
                self.states.rotate_left(idx);
                self.states.truncate(self.states.len() - idx);
                Ok(())
            },
            LogLevel::Info,
            &format!("finalize: 确认交易, addr={}, txid={}", self.addr, txid)
        )
    }

    // 添加新的池状态
    pub fn commit(&mut self, state: PoolState) {
        self.states.push(state);
    }
}