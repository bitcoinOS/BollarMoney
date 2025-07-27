# Bollar Money - Bitcoin-Backed Stablecoin Protocol

<div align="center">
  <img src="./images/bollar-logo-word.png" alt="Bollar Money Logo" width="300"/>
  
  **A Bitcoin-collateralized stablecoin protocol built on Internet Computer (ICP)**
  
  [![ICP](https://img.shields.io/badge/Built%20on-ICP-blue.svg)](https://internetcomputer.org)
  [![Bitcoin](https://img.shields.io/badge/Collateral-BTC-orange.svg)](https://bitcoin.org)
  [![License](https://img.shields.io/badge/License-MIT-green.svg)](./LICENSE)
</div>

## 🎯 Project Overview

Bollar Money is a revolutionary decentralized finance protocol built on the Internet Computer (ICP) blockchain, featuring innovative Chain Fusion technology for seamless interaction with the Bitcoin network. The protocol enables users to use native Bitcoin (BTC) and Runes protocol assets (such as Rune tokens) as collateral to mint the decentralized dollar stablecoin Bollar (Bitcoin Dollar).

Bollar is not only Bitcoin's first native stablecoin, but also aims to become the "first currency" of the entire blockchain network, providing high liquidity and high reliability for decentralized finance.

### Core Features as "First Currency"

✅ **1. Reshaping Bitcoin Finance Layer (Bitcoin Fi)**
- **Native Asset Activation**: Users can transform BTC and Runes assets into yield-bearing collateral without cross-chain bridges, unlocking Bitcoin's DeFi potential and solving the long-standing pain point of lacking native stablecoins in the Bitcoin ecosystem.
- **Seamless Cross-chain Interaction**: Based on ICP's Chain Fusion technology, enabling secure interoperability between Bitcoin network and smart contracts while avoiding centralized custody risks.

✅ **2. Universal Cross-chain Stablecoin Standard**
- **Cross-chain Liquidity Hub**: Bollar can expand to Ethereum, Solana and other chains through ICP's inter-canister communication capabilities, becoming a stablecoin bridge connecting multi-chain ecosystems.
- **Compliant and Censorship-resistant**: Fully on-chain minting/liquidation mechanism providing transparent and verifiable dollar peg, avoiding centralized black-box risks of traditional stablecoins.

✅ **3. Empowering Runes Ecosystem Assets**
- **Innovative Collateral Categories**: First to support Runes protocol assets (such as tokenized meme coins, NFTs) as collateral, solving liquidity challenges for emerging assets and driving innovation in Bitcoin Layer 2 finance.
- **Risk Hedging Tool**: Holders can collateralize volatile Runes assets into Bollar to hedge market volatility and lock in gains.

✅ **4. "Base Currency" Status in Blockchain Networks**

| Scenario | Bollar's Role |
|----------|---------------|
| Bitcoin Miners | Collateralize mining rewards into Bollar for operational costs, avoiding BTC volatility risk |
| Ordinals Ecosystem Developers | Use Bollar as DEX trading pair benchmark, unifying pricing standards |
| Cross-chain Applications | As universal collateral for multi-chain lending protocols, reducing liquidation risk |

### Technical Advantages: Why ICP?
- **Chain Fusion**: ICP nodes directly read Bitcoin state, enabling on-chain verification of BTC collateral without relying on cross-chain bridge oracles.

### Vision: Bollar's "First Currency" Path
- **Short-term**: Become the basic settlement unit for Bitcoin ecosystem DeFi (lending, DEX pricing).
- **Medium-term**: Establish Runes asset collateral standards, driving explosive growth of the Rune economy.
- **Long-term**: Through ICP's multi-chain expansion, become the reserve stablecoin for the entire blockchain network, challenging USDT/USDC monopoly.

Bollar's essence is "Bitcoin's dollarization" - preserving Bitcoin's decentralized spirit while empowering it with modern financial liquidity engines. This is not just a technological upgrade, but a key leap for Bitcoin from "digital gold" to "financial infrastructure".

### Summary
Bollar Money leverages Bitcoin native asset collateral + ICP's high-performance Chain Fusion to create the first stablecoin protocol truly serving the Bitcoin ecosystem. Bollar (Bitcoin Dollar), with its secure, efficient, and cross-chain compatible features, anchors Bitcoin's trillion-dollar value and is positioned to become the "first currency" standard in the blockchain world, providing an immutable value foundation for the decentralized economy.

### Core Features

- **Bitcoin Collateral**: Use native BTC and Runes assets as collateral
- **Dollar Stablecoin**: Mint Bollar stablecoins pegged 1:1 to USD
- **Decentralized**: Automated protocol based on smart contracts
- **High Collateral Ratio**: Up to 95% collateral ratio for capital efficiency
- **Instant Liquidation**: Automatic liquidation when collateral ratio falls below threshold
- **Runes Standard**: Based on Bitcoin's Runes protocol token standard

## 🏗️ System Architecture

### Technology Stack

- **Blockchain Platform**: Internet Computer (ICP) + Bitcoin
- **Smart Contracts**: Rust + ICP CDK + REE (Runes Exchange Environment)
- **Frontend**: React + TypeScript
- **Wallet Integration**: Unisat (Bitcoin wallet)
- **Build Tools**: Webpack + DFX

### Core Components

```
bollar-cc/
├── src/
│   ├── bollar_money_backend/    # Rust smart contracts
│   │   ├── src/
│   │   │   ├── lib.rs          # Main entry point
│   │   │   ├── types.rs        # Data structure definitions
│   │   │   ├── lending.rs      # Lending logic
│   │   │   ├── liquidation.rs  # Liquidation engine
│   │   │   ├── oracle.rs       # Price oracle
│   │   │   └── exchange.rs     # Transaction processing
│   │   └── bollar_money_backend.did  # Candid interface
│   └── bollar_money_frontend/   # React frontend application
│       ├── src/
│       │   ├── App.jsx         # Main application component
│       │   ├── components/     # UI components
│       │   ├── contexts/       # React Context
│       │   └── services/       # API services
├── dfx.json                    # ICP deployment configuration
└── package.json               # Frontend dependency management
```

## 🚀 Quick Start

### Requirements

- **Node.js** >= 18.0.0
- **Rust** >= 1.70.0
- **DFX** (ICP SDK) >= 0.15.0
- **Git**

### Installation Steps

1. **Clone the project**
   ```bash
   git clone https://github.com/bifipal/bollar-money.git
   cd bollar-money
   ```

2. **Install dependencies**
   ```bash
   # Install frontend dependencies
   npm install
   
   # Build Rust contracts
   cargo build --release
   ```

3. **Local development**
   ```bash
   # Start ICP local network
   dfx start --background --clean
   
   # Deploy contracts to local network
   dfx deploy
   
   # Start frontend development server
   npm start
   ```

4. **Access the application**
   - Frontend: http://localhost:8080
   - Candid UI: http://localhost:4943?canisterId=<backend_canister_id>

## 📋 Usage Guide

### 1. Connect Wallet

When first using the application, you need to connect your Bitcoin wallet (Unisat supported):

```javascript
// Example code
import { useWallet } from './contexts/WalletContext';

const { connect, disconnect, wallet } = useWallet();

// Connect wallet
await connect();

// Check connection status
console.log(wallet.isConnected); // true
console.log(wallet.address); // "bc1q..."
```

### 2. Create Collateral Position

Users can mint Bollar stablecoins by collateralizing BTC:

```javascript
// 1. Get pre-deposit information
const depositOffer = await api.pre_deposit(poolAddress, btcAmount);

// 2. Execute deposit and minting
const txHash = await api.execute_deposit(
  poolAddress,
  signedPsbt,
  bollarAmount
);
```

### 3. Repayment and Redemption

Users can repay Bollar and redeem collateralized BTC at any time:

```javascript
// 1. Get pre-repayment information
const repayOffer = await api.pre_repay(positionId, bollarAmount);

// 2. Execute repayment and redemption
const txHash = await api.execute_repay(positionId, signedPsbt);
```

### 4. Liquidation Mechanism

When a collateral position's health factor falls below the liquidation threshold, anyone can liquidate the position:

```javascript
// Get liquidatable positions
const liquidatable = await api.get_liquidatable_positions();

// Execute liquidation
const txHash = await api.execute_liquidate(positionId, signedPsbt);
```

## 🔧 API Reference

### Core Interfaces

#### User Authentication
```candid
authenticate : (address : text, signature : text, message : text) -> (AuthResult)
```

#### Collateral and Minting
```candid
pre_deposit : (pool_address : text, btc_amount : nat64) -> (DepositOffer)
execute_deposit : (pool_address : text, signed_psbt : text, bollar_amount : nat64) -> (variant { Ok : text; Err : Error })
```

#### Repayment and Redemption
```candid
pre_repay : (position_id : text, bollar_amount : nat64) -> (RepayOffer)
execute_repay : (position_id : text, signed_psbt : text) -> (variant { Ok : text; Err : Error })
```

#### Liquidation
```candid
get_liquidatable_positions : () -> (vec LiquidationOffer)
pre_liquidate : (position_id : text, bollar_repay_amount : nat64) -> (LiquidationOffer)
execute_liquidate : (position_id : text, signed_psbt : text) -> (variant { Ok : text; Err : Error })
```

#### Query Interfaces
```candid
get_user_positions : (user : text) -> (vec Position)
get_pool_info : (pool_address : text) -> (variant { Ok : record { collateral_ratio : nat8; liquidation_threshold : nat8; btc_locked : nat64; bollar_supply : nat64 }; Err : Error }) query
get_btc_price : () -> (nat64)
get_protocol_metrics : () -> (ProtocolMetrics)
```

## 📊 Data Structures

### Pool
```rust
pub struct Pool {
    pub states: Vec<PoolState>,    // Pool state history
    pub meta: CoinMeta,           // Token metadata
    pub pubkey: Pubkey,           // Pool public key
    pub tweaked: Pubkey,          // Tweaked public key
    pub addr: String,             // Pool address
    pub collateral_ratio: u8,     // Collateral ratio (75%)
    pub liquidation_threshold: u8, // Liquidation threshold (80%)
}
```

### Position
```rust
pub struct Position {
    pub id: String,               // Position unique identifier
    pub owner: String,            // User address
    pub btc_collateral: u64,      // BTC collateral amount (satoshis)
    pub bollar_debt: u64,         // Borrowed Bollar amount
    pub created_at: u64,          // Creation timestamp
    pub last_updated_at: u64,     // Last update timestamp
    pub health_factor: u64,       // Health factor
}
```

### ProtocolMetrics
```rust
pub struct ProtocolMetrics {
    pub total_btc_locked: u64,    // Total locked BTC
    pub total_bollar_supply: u64, // Total Bollar supply
    pub btc_price: u64,           // BTC price (USD cents)
    pub collateral_ratio: u8,     // Collateral ratio
    pub liquidation_threshold: u8, // Liquidation threshold
    pub positions_count: u64,     // Number of positions
    pub liquidatable_positions_count: u64, // Number of liquidatable positions
}
```

## 🔐 Security Mechanisms

### 1. Collateral Ratio Management
- **Minimum Collateral Ratio**: 75%
- **Liquidation Threshold**: 80%
- **Health Factor**: Collateral Value/Debt Value * 100

### 2. Price Oracle
- **Data Source**: ICP native Bitcoin price oracle
- **Update Frequency**: Every 30 seconds
- **Price Precision**: Precise to cents

### 3. Liquidation Protection
- **Liquidation Reward**: 5% additional BTC reward to liquidators
- **Reentrancy Protection**: Transaction execution lock prevents concurrent issues
- **Minimum Collateral**: 0.001 BTC minimum collateral amount

### 4. Access Control
```rust
// Permission check example
pub fn only_owner(position: &Position, caller: &Principal) -> Result<()> {
    if position.owner != caller.to_string() {
        return Err(Error::Unauthorized);
    }
    Ok(())
}
```

## 🧪 Testing

### Unit Tests
```bash
# Run all tests
cargo test

# Run specific module tests
cargo test lending_tests
cargo test liquidation_tests

# Generate test coverage report
cargo tarpaulin --out html --output-dir coverage
```

### Integration Tests
```bash
# Run end-to-end tests
npm run test:e2e

# Test specific scenarios
npm run test:integration
```

### Test Coverage
- **Target**: 90% statement coverage
- **Target**: 85% branch coverage
- **Tools**: cargo-tarpaulin + Playwright

## 🚀 Deployment

### Local Deployment
```bash
# Start local network
dfx start --clean

# Deploy contracts
dfx deploy

# Generate Candid interface
dfx generate bollar_money_backend
```

### Testnet Deployment
```bash
# Deploy to ICP testnet
dfx deploy --network ic

# Use deployment script
./deploy-testnet.sh
```

### Mainnet Deployment
```bash
# Deploy to ICP mainnet
./src/bollar_money_backend/deploy/mainnet.sh
```

## 📈 Monitoring Metrics

### Core Metrics
- **Total Value Locked (TVL)**: Total BTC locked in the protocol
- **Bollar Supply**: Total Bollar in circulation
- **Active Positions**: Current active user positions
- **Liquidation Events**: Liquidations in the last 24 hours
- **Collateral Ratio Distribution**: Position distribution across different collateral ratio ranges

### Health Indicators
- **System Collateral Ratio**: Total collateral value/total debt value
- **Liquidation Threshold**: Proportion of positions near liquidation line
- **Price Sensitivity**: Impact of BTC price volatility on the system

## 🤝 Contribution Guide

### Development Process
1. **Fork the project**
2. **Create feature branch**: `git checkout -b feature/amazing-feature`
3. **Commit changes**: `git commit -m 'Add amazing feature'`
4. **Push branch**: `git push origin feature/amazing-feature`
5. **Create Pull Request**

### Code Standards
- **Rust**: Follow rustfmt + clippy
- **JavaScript**: Follow ESLint + Prettier
- **Commit messages**: Use Conventional Commits

### Testing Requirements
- **New features**: Must include unit tests
- **Bug fixes**: Must include regression tests
- **Integration tests**: Must include end-to-end tests

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- **Internet Computer 团队**: 提供强大的区块链平台
- **Octopus Network**: REE 类型定义支持
- **Unisat**: 比特币钱包集成
- **开源社区**: 所有贡献者和支持者

## 📞 联系方式

- **项目主页**: https://github.com/bifipal/bollar-money
- **技术文档**: https://docs.bollar.money
- **社区讨论**: https://discord.gg/bollar
- **问题反馈**: https://github.com/bifipal/bollar-money/issues

---

<div align="center">
  <p><strong>⚡ 用比特币抵押，铸造美元稳定币 ⚡</strong></p>
  <p>构建在 Internet Computer 上的下一代 DeFi 协议</p>
</div>