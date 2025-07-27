use candid::{CandidType, Deserialize};
use ic_stable_structures::{Storable, storable::Bound};
use serde::Serialize;
use std::borrow::Cow;

// 重新导出 REE 类型，方便使用
pub use ree_types::{
    CoinId, InputCoin, OutputCoin, Pubkey, Txid, Utxo,
    exchange_interfaces::{
        NewBlockInfo,
    },
};

// 每个交易的最小 BTC 数量 (satoshis)
pub const MIN_BTC_VALUE: u64 = 10000;

// 代币元数据
#[derive(Clone, CandidType, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoinMeta {
    pub id: CoinId,
    pub symbol: String,
    pub min_amount: u128,
}

impl CoinMeta {
    #[allow(dead_code)]
    pub fn btc() -> Self {
        Self {
            id: CoinId::btc(),
            symbol: "BTC".to_string(),
            min_amount: 546, // 比特币的 dust limit
        }
    }
    
    #[allow(dead_code)]
    pub fn bollar() -> Self {
        Self {
            id: CoinId::rune(72798, 1058), // 示例 Rune ID，实际使用时需要替换
            symbol: "BOLLAR".to_string(),
            min_amount: 1,
        }
    }
}

// 资金池
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct Pool {
    pub states: Vec<PoolState>,  // 池状态历史
    pub meta: CoinMeta,          // 代币元数据
    pub pubkey: Pubkey,          // 池公钥
    pub tweaked: Pubkey,         // 调整后的公钥
    pub addr: String,            // 池地址
    pub collateral_ratio: u8,    // 抵押率 (例如 75 表示 75%)
    pub liquidation_threshold: u8, // 清算阈值 (例如 80 表示 80%)
}





// 池状态
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Default)]
pub struct PoolState {
    pub id: Option<Txid>,        // 创建此状态的交易 ID
    pub nonce: u64,              // 防重放攻击的计数器
    pub utxo: Option<Utxo>,      // 池的 UTXO
    pub btc_price: u64,          // BTC 价格 (USD cents)
}

// 用户抵押头寸
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Position {
    pub id: String,              // 头寸唯一标识符
    pub owner: String,           // 用户地址
    pub btc_collateral: u64,     // BTC 抵押数量 (satoshis)
    pub bollar_debt: u64,        // 借出的 Bollar 数量
    pub created_at: u64,         // 创建时间戳
    pub last_updated_at: u64,    // 最后更新时间戳
    pub health_factor: u64,      // 健康因子 (抵押价值/债务价值 * 100)
}

impl Position {
    // 创建新头寸
    pub fn new(
        id: String,
        owner: String,
        btc_collateral: u64,
        bollar_debt: u64,
        btc_price: u64,
    ) -> Self {
        let now = crate::ic_api::time();
        let health_factor = calculate_health_factor(btc_collateral, bollar_debt, btc_price);
        
        Self {
            id,
            owner,
            btc_collateral,
            bollar_debt,
            created_at: now,
            last_updated_at: now,
            health_factor,
        }
    }
    
    // 更新头寸
    pub fn update(
        &mut self,
        btc_collateral: u64,
        bollar_debt: u64,
        btc_price: u64,
    ) {
        self.btc_collateral = btc_collateral;
        self.bollar_debt = bollar_debt;
        self.last_updated_at = crate::ic_api::time();
        self.health_factor = calculate_health_factor(btc_collateral, bollar_debt, btc_price);
    }
    
    // 检查头寸是否可清算
    pub fn is_liquidatable(&self, liquidation_threshold: u8) -> bool {
        self.health_factor < (liquidation_threshold as u64)
    }
}

// 交易记录
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Default)]
pub struct TxRecord {
    pub pools: Vec<String>,      // 受影响的池地址
    pub timestamp: u64,          // 交易时间戳
    pub action: String,          // 交易类型 (deposit, repay, liquidate)
    pub user: String,            // 执行交易的用户
}

// 抵押预处理结果
#[derive(Eq, PartialEq, CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DepositOffer {
    pub pool_utxo: Option<Utxo>, // 池的当前 UTXO (首次抵押时为 None)
    pub nonce: u64,              // 交易 nonce
    pub btc_price: u64,          // BTC 当前价格
    pub max_bollar_mint: u64,    // 可铸造的最大 Bollar 数量
}

// 还款预处理结果
#[derive(Eq, PartialEq, CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct RepayOffer {
    pub pool_utxo: Utxo,         // 池的当前 UTXO
    pub nonce: u64,              // 交易 nonce
    pub btc_return: u64,         // 可赎回的 BTC 数量
}

// 清算预处理结果
#[derive(Eq, PartialEq, CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct LiquidationOffer {
    pub position_id: String,     // 头寸 ID
    pub owner: String,           // 头寸所有者
    pub btc_collateral: u64,     // BTC 抵押数量
    pub bollar_debt: u64,        // Bollar 债务数量
    pub health_factor: u64,      // 健康因子
    pub liquidation_bonus: u64,  // 清算奖励 (额外 BTC)
}

// 认证结果
#[derive(CandidType, Deserialize, Serialize)]
pub struct AuthResult {
    pub success: bool,
    pub message: String,
    pub token: Option<String>,
}

// 协议指标
#[derive(CandidType, Deserialize, Serialize)]
pub struct ProtocolMetrics {
    pub total_btc_locked: u64,
    pub total_bollar_supply: u64,
    pub btc_price: u64,
    pub collateral_ratio: u8,
    pub liquidation_threshold: u8,
    pub positions_count: u64,
    pub liquidatable_positions_count: u64,
}

// 计算头寸健康因子
pub fn calculate_health_factor(
    btc_collateral: u64,
    bollar_debt: u64,
    btc_price: u64,
) -> u64 {
    if bollar_debt == 0 {
        return u64::MAX; // 无债务，健康因子无限大
    }
    
    // 计算抵押品价值 (USD cents)
    let collateral_value = (btc_collateral as u128) * (btc_price as u128) / 100_000_000;
    
    // 计算健康因子 (抵押价值/债务价值 * 100)
    // bollar_debt 已经是以 cents 为单位，所以直接使用
    let health_factor = collateral_value * 100 / (bollar_debt as u128);
    
    health_factor.try_into().unwrap_or(0)
}

// 为数据结构实现 Storable trait，以便在稳定存储中使用
impl Storable for PoolState {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        let mut bytes = vec![];
        let _ = ciborium::ser::into_writer(self, &mut bytes);
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        ciborium::de::from_reader(bytes.as_ref()).expect("Failed to decode PoolState")
    }
}

impl Storable for Pool {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        let mut bytes = vec![];
        let _ = ciborium::ser::into_writer(self, &mut bytes);
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        ciborium::de::from_reader(bytes.as_ref()).expect("Failed to decode Pool")
    }
}

impl Storable for Position {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        let mut bytes = vec![];
        let _ = ciborium::ser::into_writer(self, &mut bytes);
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        ciborium::de::from_reader(bytes.as_ref()).expect("Failed to decode Position")
    }
}

impl Storable for TxRecord {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        let mut bytes = vec![];
        let _ = ciborium::ser::into_writer(self, &mut bytes);
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        ciborium::de::from_reader(bytes.as_ref()).expect("Failed to decode TxRecord")
    }
}