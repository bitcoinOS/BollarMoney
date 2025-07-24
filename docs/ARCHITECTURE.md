# Bollar Money Technical Architecture Document

## Project Overview
Bollar Money is a Bitcoin over-collateralized stablecoin protocol based on the Internet Computer (ICP) network, integrated with the Bitcoin network through Chain Fusion technology, using the Runes standard to issue Bollar stablecoins.

## System Architecture

### Overall Architecture
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Bitcoin       │    │   ICP Network   │    │    Frontend     │
│   Network       │◄───┤   Canister      │◄───┤   React App     │
│   (BTC Collateral)│    │   (Rust CDK)    │    │   (Unisat)      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                        │                        │
         │                        │                        │
    ┌────────┐              ┌────────┐              ┌────────┐
    │  BTC   │              │Oracle  │              │Web     │
    │Collateral│              │Price   │              │Interface│
    └────────┘              └────────┘              └────────┘
```

### Core Components

#### 1. Canister Architecture (Single Canister Design)
- **Location**: ICP Network
- **Language**: Rust + CDK 0.18.x
- **Functions**: Token issuance, collateral management, lending logic, liquidation mechanism

#### 2. Oracle Integration
- **Type**: ICP Network Native Oracle canister
- **Functions**: BTC/USD price acquisition and validation
- **Update Frequency**: Per block update

#### 3. Runes Integration
- **Standard**: Runes (REE environment)
- **Functions**: Bollar token issuance and management
- **Network**: Bitcoin Network

## Data Model

### Collateral
```rust
pub struct Collateral {
    pub id: u64,
    pub owner: Principal,
    pub amount: u64,          // BTC amount in satoshis
    pub minted_bollar: u64,   // Amount of Bollar minted against this collateral
    pub collateral_ratio: u32, // Current collateralization ratio (basis points)
    pub created_at: u64,
    pub last_updated: u64,
}
```

### CDP (Collateralized Debt Position)
```rust
pub struct CDP {
    pub id: u64,
    pub owner: Principal,
    pub collateral_amount: u64,
    pub minted_amount: u64,
    pub liquidation_price: u64,
    pub is_liquidated: bool,
}
```

### System Configuration
```rust
pub struct SystemConfig {
    pub max_collateral_ratio: u32,  // 90% (9000 basis points)
    pub liquidation_threshold: u32, // 85% (8500 basis points)
    pub liquidation_penalty: u32,   // 5% (500 basis points)
}
```

## Business Processes

### 1. Collateral Process
1. User connects Unisat wallet
2. User sends BTC to designated address
3. System confirms BTC receipt
4. Calculate mintable Bollar amount based on current BTC price
5. Mint Bollar to user

### 2. Price Update Process
1. Oracle canister periodically fetches BTC price
2. System recalculates collateralization ratios for all CDPs based on new price
3. Mark CDPs requiring liquidation

### 3. Liquidation Process
1. Anyone can call the liquidation function
2. Check if CDP collateral ratio is below liquidation threshold
3. Liquidator pays Bollar to receive collateral (with reward)
4. Update CDP status to liquidated

## Error Handling Mechanisms

### Exception Cases
- Oracle price fetch failure: use last valid price
- BTC transfer failure: rollback operation
- Collateral ratio calculation exception: reject operation
- Liquidation conditions not met: reject liquidation

### Security Mechanisms
- Input validation: all parameter boundary checks
- Reentrancy protection: prevent reentrancy attacks
- Access control: role-based access control
- Emergency pause: global pause mechanism

## Performance Considerations

### Optimization Strategies
- State storage optimization: use efficient data structures
- Batch operations: support batch liquidation
- Caching mechanism: price data caching
- Gas optimization: minimize computational complexity

### Scalability Design
Although MVP is single canister, interfaces are reserved for multi-canister expansion.