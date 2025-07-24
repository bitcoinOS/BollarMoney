# Bollar Money Technical Design Document

## Technical Overview

### Architecture Choice: ICP Native + Rust CDK
Choosing Internet Computer (ICP) as the underlying blockchain platform, using Rust CDK 0.18.x to develop the core protocol. This choice is based on the following reasons:

- **Chain Fusion Technology**: ICP natively supports Bitcoin integration without third-party bridges
- **High Performance**: ICP provides WebAssembly execution environment, superior to EVM
- **Cost Advantage**: Compared to Ethereum, ICP transaction costs are extremely low
- **Developer Friendly**: Rust language advantages in security and performance

### Technology Stack Decisions
- **Backend**: Rust + ic-cdk 0.18.x
- **Frontend**: React + TypeScript + Unisat Wallet SDK
- **Testing**: Rust unit tests + integration tests + E2E tests
- **Deployment**: dfx toolchain + GitHub Actions CI/CD

## System Architecture

### High-level Component Diagram
```
┌─────────────────────────────────────────┐
│           Frontend Layer                │
│  ┌─────────────┐   ┌──────────────┐    │
│  │ React App   │   │ Unisat SDK   │    │
│  │ Web UI      │   │ Wallet       │    │
│  └─────────────┘   └──────────────┘    │
└─────────────────────────────────────────┘
              │ JSON API / WebSocket
┌─────────────────────────────────────────┐
│        Bollar Canister                  │
│  ┌─────────────────────────────────────┐ │
│  │  Core Protocol Logic                │ │
│  │  ┌─────────┐ ┌─────────┐ ┌────────┐ │ │
│  │  │Collateral│ │  Price  │ │Liquid. │ │ │
│  │  │ Manager  │ │ Oracle  │ │ Engine │ │ │
│  │  └─────────┘ └─────────┘ └────────┘ │ │
│  └─────────────────────────────────────┘ │
│  ┌─────────────────────────────────────┐ │
│  │  Data Storage                       │ │
│  │  ┌─────────┐ ┌─────────┐ ┌────────┐ │ │
│  │  │ CDPs    │ │ Config  │ │ Stats  │ │ │
│  │  └─────────┘ └─────────┘ └────────┘ │ │
│  └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
              │
┌─────────────────────────────────────────┐
│         External Services               │
│  ┌─────────────┐   ┌──────────────┐    │
│  │ ICP Oracle  │   │ Bitcoin      │    │
│  │ Canister    │   │ Network      │    │
│  └─────────────┘   └──────────────┘    │
└─────────────────────────────────────────┘
```

### Core Component Interactions
1. **Frontend Layer**: React application handles user interaction and wallet connection
2. **Bollar Canister**: Core protocol logic, handles CDP management, price calculation and liquidation
3. **External Services**: ICP Oracle provides price data, Bitcoin network handles BTC transfers

## Data Design

### Storage Structure
```rust
// Main storage structure
struct BollarCanister {
    cdps: StableBTreeMap<u64, CDP>,           // CDP storage
    user_cdps: StableBTreeMap<Principal, Vec<u64>>, // User CDP index
    system_config: SystemConfig,              // System configuration
    price_history: Vec<PriceData>,           // Price history
    mint_stats: MintStats,                   // Minting statistics
    next_cdp_id: u64,                        // Next CDP ID
}
```

### Data Validation Rules
- **BTC Amount Validation**: Minimum 0.001 BTC, maximum 1000 BTC
- **Bollar Amount Validation**: Minimum $0.01, maximum limited by collateral ratio
- **Address Validation**: BTC address format validation (P2PKH, P2SH, Bech32)
- **Time Validation**: Price data no older than 5 minutes

### Data Consistency Guarantees
- **Atomic Operations**: Using ICP's transaction mechanism
- **State Synchronization**: Sync all CDP states when price updates
- **Data Integrity**: All calculation results are verifiable

## API Design

### Core Endpoint Specifications

#### 1. System Information Endpoint
```rust
#[query]
fn get_system_info() -> SystemInfo {
    SystemInfo {
        version: "1.0.0",
        max_collateral_ratio: 9000,
        liquidation_threshold: 8500,
        liquidation_penalty: 500,
        btc_price: get_current_price(),
        total_collateral: get_total_collateral(),
        total_minted: get_total_minted(),
    }
}
```

#### 2. CDP Management Endpoints
```rust
#[update]
fn create_cdp(btc_tx_hash: String, amount_satoshis: u64) -> ApiResponse<u64>;

#[update]
fn mint_bollar(cdp_id: u64, amount_cents: u64) -> ApiResponse<u64>;

#[update]
fn close_cdp(cdp_id: u64, bollar_amount: u64) -> ApiResponse<String>;
```

#### 3. Liquidation Endpoints
```rust
#[query]
fn get_liquidatable_cdps() -> Vec<LiquidationInfo>;

#[update]
fn liquidate_cdp(cdp_id: u64, bollar_amount: u64) -> ApiResponse<String>;
```

### Request/Response Format
All APIs use unified response format:
```json
{
  "status": "success|error",
  "data": {...},
  "error": {"code": "...", "message": "..."}
}
```

## Security Design

### Authentication Mechanism
- **Wallet Authentication**: Use Unisat wallet signature to verify user identity
- **Principal Binding**: Each CDP is bound to a unique ICP Principal
- **Access Control**: Users can only operate their own CDPs

### Input Validation
```rust
fn validate_deposit_amount(amount: u64) -> Result<(), ProtocolError> {
    if amount < MIN_COLLATERAL_AMOUNT {
        return Err(ProtocolError::AmountTooSmall(amount, MIN_COLLATERAL_AMOUNT));
    }
    if amount > MAX_COLLATERAL_AMOUNT {
        return Err(ProtocolError::AmountTooLarge(amount, MAX_COLLATERAL_AMOUNT));
    }
    Ok(())
}
```

### Reentrancy Protection
- **State Locking**: Operations in progress use lock mechanism
- **Time Window**: Prevent rapid repeated operations
- **Transaction Integrity**: Failed operations automatically rollback

### Access Control Matrix
| Operation | Owner | Liquidator | Admin |
|------|--------|--------|--------|
| Create CDP | ✅ | ❌ | ❌ |
| Mint Bollar | ✅ | ❌ | ❌ |
| Close CDP | ✅ | ❌ | ❌ |
| Liquidate CDP | ❌ | ✅ | ❌ |
| System Configuration | ❌ | ❌ | ✅ |

## Performance Design

### Performance Targets
- **Query Latency**: < 100ms
- **Update Latency**: < 500ms
- **Concurrent Processing**: Support 100 concurrent operations
- **Memory Usage**: < 4GB peak

### Optimization Strategies

#### 1. Caching Mechanism
```rust
struct Cache {
    btc_price: (u64, u64),  // (price, timestamp)
    liquidation_list: (Vec<u64>, u64), // (cdp_ids, timestamp)
    system_stats: (SystemHealth, u64), // (stats, timestamp)
}
```

#### 2. Batch Operations
- **Batch Liquidation**: Support processing multiple CDPs at once
- **Batch State Updates**: Update CDP states in batches when price changes

#### 3. Data Pagination
- **CDP List Pagination**: Support paginated CDP queries by user
- **Price History Pagination**: Support time range queries

### Memory Optimization
- **Data Structure Compression**: Use compact data structures
- **Garbage Collection**: Regular cleanup of historical data
- **Memory Pool**: Reuse temporary objects

## Scalability Design

### Single Canister to Multi-Canister Evolution

#### Phase 1: MVP Single Canister
- All functions in one canister
- Suitable for MVP rapid validation

#### Phase 2: Vertical Separation
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   CDP Manager   │    │   Price Oracle  │    │   Statistics    │
│   Canister      │    │   Canister      │    │   Canister      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

#### Phase 3: Horizontal Expansion
- **User Sharding**: Store CDPs by Principal sharding
- **Function Separation**: Liquidation engine as independent canister

### Interface Compatibility
- **Version Control**: API versioning design
- **Backward Compatibility**: Support older client versions
- **Progressive Migration**: Smooth upgrade strategy

## Error Handling

### Error Classification
```rust
enum ProtocolError {
    // Input errors
    InvalidAmount,
    InvalidAddress,
    
    // Business logic errors
    InsufficientCollateral,
    CDPNotFound,
    CDPAlreadyLiquidated,
    
    // System errors
    OraclePriceError,
    RunesOperationFailed,
    
    // Permission errors
    UnauthorizedAccess,
}
```

### Error Recovery Mechanisms
- **Auto Retry**: Auto retry on network errors
- **Degraded Service**: Use cached price when Oracle fails
- **Manual Intervention**: System administrator manual recovery mechanism

## Test Strategy Integration

### Unit Test Design
- **Mathematical Calculation Tests**: Collateral ratio calculation accuracy
- **Boundary Condition Tests**: Extreme value handling
- **Error Path Tests**: Coverage of all error scenarios

### Integration Test Design
- **Oracle Integration**: Price acquisition and validation
- **Runes Integration**: Token operation tests
- **Wallet Integration**: Unisat connection tests

### End-to-End Test Design
- **Complete User Flow**: Deposit → Mint → Redeem
- **Liquidation Scenarios**: Price triggered liquidation tests
- **Concurrency Tests**: Multiple users operating simultaneously

## Deployment and Operations

### Deployment Stages
1. **Testnet Deployment**: Use ICP testnet
2. **Beta Testing**: Invite core users to test
3. **Mainnet Launch**: Gradually open features

### Monitoring Metrics
- **Business Metrics**: Total collateral amount, total minted amount, liquidation count
- **Performance Metrics**: Response time, error rate, memory usage
- **Security Metrics**: Attack attempts, abnormal operations

### Upgrade Strategy
- **Blue-Green Deployment**: Zero-downtime upgrade
- **Canary Release**: Gradual user migration
- **Rollback Mechanism**: Quick rollback to stable version

## Technical Risk Assessment

### High Priority Risks
1. **Oracle Single Point Failure**: Price data source issues
2. **Liquidation Delay**: Untimely price updates causing bad debt
3. **Reentrancy Attack**: Smart contract reentrancy vulnerabilities

### Risk Mitigation Measures
1. **Multi-Oracle Sources**: At least 3 independent price sources
2. **Price Buffer Period**: Pause operations during price anomalies
3. **Security Audit**: Third-party security audit
4. **Insurance Fund**: System risk reserve fund

This technical design provides a complete technical implementation blueprint for Bollar Money, supporting smooth evolution from MVP to production environment.