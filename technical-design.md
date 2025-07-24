# Bollar Money Protocol - Comprehensive Technical Design

## Technical Overview

### Architecture Approach: ICP Native with Rust CDK

The Bollar Money protocol implements a Bitcoin-collateralized stablecoin system on Internet Computer (ICP) using Chain Fusion technology. The architecture leverages ICP's native Bitcoin integration, eliminating the need for external bridges while maintaining decentralization and security.

**Key Design Decisions:**
- **Single Canister MVP**: All protocol logic contained within one canister for simplicity and atomic operations
- **Rust CDK 0.18.x**: Provides memory safety, performance, and strong type system
- **Chain Fusion**: Direct Bitcoin network integration via ICP's Bitcoin API
- **Runes Protocol**: For Bollar token standard on Bitcoin network
- **StableBTreeMap**: Persistent storage for all critical state

### Technology Stack Justification

**Backend Stack:**
- **Language**: Rust 1.75+ with ic-cdk 0.18.x
- **Storage**: ICP Stable Structures (StableBTreeMap)
- **Bitcoin Integration**: ic-cdk bitcoin integration API
- **Cryptography**: ed25519 for signatures, SHA-256 for hashing

**Frontend Stack:**
- **Framework**: React 18.x with TypeScript 5.x
- **Wallet**: Unisat Wallet SDK for Bitcoin operations
- **State Management**: React Query + Zustand for canister interactions
- **UI Framework**: Tailwind CSS + Headless UI

**Infrastructure:**
- **Deployment**: dfx toolchain with GitHub Actions CI/CD
- **Testing**: Rust unit tests + Playwright E2E tests
- **Monitoring**: ICP dashboard + custom metrics

## System Architecture

### High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        User Layer                               │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌────────────────────┐   │
│  │   React     │    │   Unisat    │    │   Web3 Provider    │   │
│  │   Frontend  │◄───┤   Wallet    │◄───┤   (Bitcoin/ICP)    │   │
│  │   (Web UI)  │    │   SDK       │    │   Integration      │   │
│  └─────────────┘    └─────────────┘    └────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │ JSON-RPC / HTTPS
┌─────────────────────────────────────────────────────────────────┐
│                    Bollar Protocol Canister                     │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐    ┌─────────────────────┐             │
│  │   API Gateway       │    │   Security Layer    │             │
│  │   - Rate Limiting   │    │   - Authentication  │             │
│  │   - Request Router  │    │   - Authorization   │             │
│  └─────────────────────┘    └─────────────────────┘             │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                 Core Protocol Logic                      │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │   │
│  │  │   CDP       │  │   Price     │  │   Liquidation   │ │   │
│  │  │   Manager   │  │   Oracle    │  │   Engine        │ │   │
│  │  │   Module    │  │   Module    │  │   Module        │ │   │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘ │   │
│  └─────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                  Data Storage                            │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │   │
│  │  │   CDP       │  │   System    │  │   Audit         │ │   │
│  │  │   Registry  │  │   Config    │  │   Trail         │ │   │
│  │  │   (BTree)   │  │   (Stable)  │  │   (Append)      │ │   │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘ │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
              │                         │
              │ Bitcoin Network API     │ ICP Oracle API
┌─────────────┴───────────┐   ┌─────────┴─────────────┐
│    Bitcoin Network      │   │   ICP Price Oracle  │
│   (BTC Deposits)        │   │   (BTC/USD Price)   │
└─────────────────────────┘   └─────────────────────┘
```

### Data Flow Architecture

**CDP Creation Flow:**
1. User deposits BTC to protocol address via Bitcoin network
2. Bitcoin API confirms transaction inclusion
3. Protocol validates BTC amount (0.001-1000 BTC range)
4. CDP Manager creates new position with calculated collateral ratio
5. System updates total collateral and user CDP index

**Price Update Flow:**
1. Oracle Module fetches latest BTC/USD price from ICP Oracle
2. Validates price freshness (< 5 minutes old)
3. Updates all active CDP collateral ratios
4. Marks CDPs below 85% as liquidatable
5. Triggers liquidation events for eligible positions

**Liquidation Flow:**
1. Liquidator queries liquidatable CDPs
2. Liquidation Engine validates liquidation conditions
3. Calculates liquidation penalty (5% of collateral)
4. Transfers BTC to liquidator (collateral - penalty)
5. Burns corresponding Bollar tokens
6. Updates CDP state to liquidated

## Data Design

### Database Schema and Relationships

```rust
// Primary CDP data structure
#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct CDP {
    pub id: u64,
    pub owner: Principal,
    pub btc_tx_hash: String,          // Bitcoin transaction hash
    pub collateral_amount: u64,       // BTC amount in satoshis
    pub minted_bollar: u64,           // Bollar tokens (USD cents)
    pub collateral_ratio: u32,        // Current ratio (basis points)
    pub liquidation_price: u64,       // BTC price triggering liquidation
    pub created_at: u64,              // Block timestamp
    pub last_updated: u64,            // Last modification timestamp
    pub status: CDPStatus,            // Active | Liquidated | Closed
    pub version: u32,                 // Schema version for migrations
}

// System configuration
#[derive(CandidType, Serialize, Deserialize)]
pub struct SystemConfig {
    pub max_collateral_ratio: u32,    // 9000 basis points (90%)
    pub liquidation_threshold: u32,   // 8500 basis points (85%)
    pub liquidation_penalty: u32,     // 500 basis points (5%)
    pub min_collateral_amount: u64,   // 100000 satoshis (0.001 BTC)
    pub max_collateral_amount: u64,   // 100000000000 satoshis (1000 BTC)
    pub oracle_timeout: u64,          // 300 seconds (5 minutes)
    pub emergency_pause: bool,        // Emergency stop mechanism
    pub admin_principals: Vec<Principal>, // System administrators
}

// Price data with validation
#[derive(CandidType, Serialize, Deserialize)]
pub struct PriceData {
    pub btc_price: u64,               // BTC price in USD cents
    pub timestamp: u64,               // Unix timestamp
    pub source: String,               // Oracle source identifier
    pub confidence: u32,              // Confidence score (0-10000)
    pub signature: Vec<u8>,           // Oracle signature for verification
}

// User CDP index for efficient queries
#[derive(CandidType, Serialize, Deserialize)]
pub struct UserCDPIndex {
    pub user_principal: Principal,
    pub cdp_ids: Vec<u64>,
    pub total_collateral: u64,
    pub total_minted: u64,
    pub last_activity: u64,
}

// Audit trail for compliance
#[derive(CandidType, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_type: String,           // "CDP_CREATED", "MINTED", "LIQUIDATED"
    pub cdp_id: u64,
    pub user: Principal,
    pub amount: u64,                  // Relevant amount
    pub timestamp: u64,
    pub tx_hash: Option<String>,      // Bitcoin transaction if applicable
    pub details: String,              // JSON-encoded details
}
```

### Data Validation and Constraints

**Input Validation Rules:**
```rust
impl CDP {
    pub fn validate_collateral_amount(amount: u64) -> Result<(), ProtocolError> {
        if amount < SYSTEM_CONFIG.min_collateral_amount {
            return Err(ProtocolError::InvalidAmount(
                format!("Amount {} below minimum {} satoshis", 
                       amount, SYSTEM_CONFIG.min_collateral_amount)
            ));
        }
        if amount > SYSTEM_CONFIG.max_collateral_amount {
            return Err(ProtocolError::InvalidAmount(
                format!("Amount {} above maximum {} satoshis", 
                       amount, SYSTEM_CONFIG.max_collateral_amount)
            ));
        }
        Ok(())
    }

    pub fn validate_btc_address(address: &str) -> Result<(), ProtocolError> {
        let decoded = bitcoin::Address::from_str(address)
            .map_err(|_| ProtocolError::InvalidAddress("Invalid Bitcoin address".to_string()))?;
        
        match decoded.address_type() {
            Some(AddressType::P2pkh) | Some(AddressType::P2sh) | Some(AddressType::P2wpkh) => Ok(()),
            _ => Err(ProtocolError::InvalidAddress("Unsupported address type".to_string())),
        }
    }
}
```

**Data Integrity Constraints:**
- All BTC amounts stored as satoshis (u64) to prevent floating-point errors
- All USD values stored as cents (u64) for precise calculations
- Timestamps validated against block time to prevent future dates
- Principal ownership verified for all state-changing operations

### Migration and Versioning Strategy

```rust
// Schema versioning for future upgrades
const CURRENT_SCHEMA_VERSION: u32 = 1;

enum MigrationState {
    NotStarted,
    InProgress,
    Completed,
    Failed(String),
}

struct SchemaMigrator {
    pub current_version: u32,
    pub target_version: u32,
    pub migration_log: Vec<MigrationEvent>,
}

// Migration plan for future schema changes
impl SchemaMigrator {
    pub fn migrate_v1_to_v2(&mut self) -> Result<(), ProtocolError> {
        // Example: Add new fields to CDP struct
        // 1. Backup existing CDPs
        // 2. Create new CDP structure with additional fields
        // 3. Migrate data with validation
        // 4. Verify migration completeness
        Ok(())
    }
}
```

## API Design

### RESTful Endpoint Specifications

#### System Information Endpoints

```rust
// GET /system/info
#[query]
fn get_system_info() -> SystemInfoResponse {
    SystemInfoResponse {
        version: env!("CARGO_PKG_VERSION"),
        max_collateral_ratio: SYSTEM_CONFIG.max_collateral_ratio,
        liquidation_threshold: SYSTEM_CONFIG.liquidation_threshold,
        liquidation_penalty: SYSTEM_CONFIG.liquidation_penalty,
        current_btc_price: get_current_price(),
        total_collateral_btc: get_total_collateral(),
        total_minted_bollar: get_total_minted(),
        active_cdp_count: get_active_cdp_count(),
        system_health_score: calculate_health_score(),
    }
}

// GET /system/health
#[query]
fn get_system_health() -> HealthReport {
    HealthReport {
        status: "healthy|degraded|critical",
        last_price_update: get_last_price_timestamp(),
        oracle_status: check_oracle_health(),
        liquidatable_cdps: count_liquidatable_cdps(),
        system_utilization: calculate_utilization_ratio(),
    }
}
```

#### CDP Management Endpoints

```rust
// POST /cdp/create
#[update]
fn create_cdp(params: CreateCDPParams) -> ApiResponse<CDPCreationResult> {
    let CreateCDPParams {
        btc_tx_hash,
        amount_satoshis,
        btc_address,
    } = params;

    // Validate inputs
    CDP::validate_collateral_amount(amount_satoshis)?;
    CDP::validate_btc_address(&btc_address)?;
    
    // Verify Bitcoin transaction
    verify_btc_deposit(&btc_tx_hash, amount_satoshis, &btc_address)?;
    
    // Create CDP with atomic operations
    let cdp_id = CDP_MANAGER.create(
        caller(),
        btc_tx_hash,
        amount_satoshis,
        get_current_price(),
    )?;
    
    Ok(CDPCreationResult { cdp_id, max_mintable: calculate_max_mintable(amount_satoshis) })
}

// POST /cdp/{cdp_id}/mint
#[update]
fn mint_bollar(cdp_id: u64, params: MintParams) -> ApiResponse<MintResult> {
    let MintParams { amount_cents } = params;
    
    // Verify ownership
    let mut cdp = CDP_REGISTRY.get(cdp_id)
        .ok_or(ProtocolError::CDPNotFound)?;
    
    if cdp.owner != caller() {
        return Err(ProtocolError::UnauthorizedAccess);
    }
    
    // Validate mint amount
    let current_price = get_current_price();
    let max_mintable = calculate_max_mintable(cdp.collateral_amount, current_price);
    
    if amount_cents > max_mintable {
        return Err(ProtocolError::InsufficientCollateral(
            format!("Requested {} exceeds max {} cents", amount_cents, max_mintable)
        ));
    }
    
    // Execute mint via Runes protocol
    let mint_tx = RUNES_PROTOCOL.mint_bollar(cdp.owner, amount_cents)?;
    
    // Update CDP state
    cdp.minted_bollar += amount_cents;
    cdp.last_updated = ic_cdk::api::time();
    CDP_REGISTRY.insert(cdp_id, cdp);
    
    Ok(MintResult { tx_hash: mint_tx, new_ratio: calculate_collateral_ratio(&cdp) })
}

// POST /cdp/{cdp_id}/close
#[update]
fn close_cdp(cdp_id: u64, params: CloseParams) -> ApiResponse<CloseResult> {
    let CloseParams { bollar_amount } = params;
    
    let mut cdp = CDP_REGISTRY.get(cdp_id)
        .ok_or(ProtocolError::CDPNotFound)?;
    
    if cdp.owner != caller() {
        return Err(ProtocolError::UnauthorizedAccess);
    }
    
    if cdp.status != CDPStatus::Active {
        return Err(ProtocolError::CDPAlreadyLiquidated);
    }
    
    // Verify sufficient Bollar to close
    if bollar_amount < cdp.minted_bollar {
        return Err(ProtocolError::InsufficientCollateral(
            "Insufficient Bollar to close CDP".to_string()
        ));
    }
    
    // Burn Bollar via Runes
    let burn_tx = RUNES_PROTOCOL.burn_bollar(cdp.owner, cdp.minted_bollar)?;
    
    // Release BTC collateral
    let release_tx = release_btc_collateral(&cdp.btc_tx_hash, &cdp.owner)?;
    
    // Update CDP status
    cdp.status = CDPStatus::Closed;
    CDP_REGISTRY.insert(cdp_id, cdp);
    
    Ok(CloseResult { 
        btc_release_tx: release_tx,
        bollar_burn_tx: burn_tx,
        released_amount: cdp.collateral_amount
    })
}
```

#### Liquidation Endpoints

```rust
// GET /liquidations/available
#[query]
fn get_liquidatable_cdps(params: PaginationParams) -> ApiResponse<LiquidationList> {
    let liquidatable = CDP_REGISTRY
        .iter()
        .filter(|(_, cdp)| is_liquidatable(cdp))
        .map(|(id, cdp)| LiquidationInfo {
            cdp_id: id,
            owner: cdp.owner,
            collateral_amount: cdp.collateral_amount,
            minted_bollar: cdp.minted_bollar,
            current_ratio: cdp.collateral_ratio,
            liquidation_price: cdp.liquidation_price,
            potential_reward: calculate_liquidation_reward(cdp),
        })
        .collect();
    
    Ok(LiquidationList { cdps: liquidatable, total_count: liquidatable.len() as u64 })
}

// POST /liquidations/{cdp_id}/execute
#[update]
fn liquidate_cdp(cdp_id: u64, params: LiquidationParams) -> ApiResponse<LiquidationResult> {
    let LiquidationParams { bollar_amount } = params;
    
    let mut cdp = CDP_REGISTRY.get(cdp_id)
        .ok_or(ProtocolError::CDPNotFound)?;
    
    if !is_liquidatable(&cdp) {
        return Err(ProtocolError::InvalidOperation("CDP not eligible for liquidation".to_string()));
    }
    
    // Calculate liquidation reward
    let reward = calculate_liquidation_reward(&cdp);
    let liquidator_amount = cdp.collateral_amount - reward;
    
    // Verify liquidator has sufficient Bollar
    let liquidator_balance = RUNES_PROTOCOL.balance_of(caller())?;
    if liquidator_balance < bollar_amount {
        return Err(ProtocolError::InsufficientCollateral(
            "Insufficient Bollar for liquidation".to_string()
        ));
    }
    
    // Execute liquidation
    let burn_tx = RUNES_PROTOCOL.burn_bollar(caller(), cdp.minted_bollar)?;
    let transfer_tx = transfer_btc_to_liquidator(&cdp.btc_tx_hash, caller(), liquidator_amount)?;
    
    // Update CDP
    cdp.status = CDPStatus::Liquidated;
    CDP_REGISTRY.insert(cdp_id, cdp);
    
    // Record liquidation event
    record_audit_event(AuditEvent::liquidation(cdp_id, caller(), reward));
    
    Ok(LiquidationResult {
        bollar_burned: cdp.minted_bollar,
        btc_transferred: liquidator_amount,
        liquidation_reward: reward,
    })
}
```

### Request/Response Formats

**Standard Response Wrapper:**
```rust
#[derive(CandidType, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub status: ResponseStatus,
    pub data: Option<T>,
    pub error: Option<ErrorDetails>,
    pub timestamp: u64,
    pub request_id: String, // For tracking/debugging
}

#[derive(CandidType, Serialize, Deserialize)]
pub enum ResponseStatus {
    Success,
    Error,
    Pending,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct ErrorDetails {
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
    pub retryable: bool,
}
```

### Authentication and Authorization

**Wallet-Based Authentication:**
```rust
// Verify caller identity through ICP Principal
fn require_authenticated() -> Result<Principal, ProtocolError> {
    let caller = ic_cdk::caller();
    if caller == Principal::anonymous() {
        return Err(ProtocolError::UnauthorizedAccess);
    }
    Ok(caller)
}

// Verify CDP ownership
fn require_cdp_owner(cdp_id: u64) -> Result<(Principal, CDP), ProtocolError> {
    let caller = require_authenticated()?;
    let cdp = CDP_REGISTRY.get(cdp_id)
        .ok_or(ProtocolError::CDPNotFound)?;
    
    if cdp.owner != caller {
        return Err(ProtocolError::UnauthorizedAccess);
    }
    Ok((caller, cdp))
}
```

**Role-Based Access Control:**
```rust
#[derive(CandidType, Serialize, Deserialize)]
pub enum UserRole {
    User,           // Standard user
    Liquidator,     // Can liquidate positions
    Admin,          // System administration
    Emergency,      // Emergency operations
}

impl AccessControl {
    pub fn check_role(principal: &Principal, role: UserRole) -> bool {
        match role {
            UserRole::User => true, // All authenticated users
            UserRole::Liquidator => LIQUIDATOR_WHITELIST.contains(principal),
            UserRole::Admin => SYSTEM_CONFIG.admin_principals.contains(principal),
            UserRole::Emergency => EMERGENCY_ADMINS.contains(principal),
        }
    }
}
```

## Security Considerations

### Authentication Mechanisms

**Multi-Level Authentication:**
1. **ICP Principal Verification**: All operations require authenticated ICP principal
2. **Bitcoin Address Verification**: BTC deposits verified through transaction signatures
3. **Runes Token Verification**: Bollar token operations validated against Runes protocol
4. **Session Management**: Time-based session tokens for frontend interactions

**Authentication Flow:**
```rust
pub struct AuthManager {
    pub session_timeout: Duration,
    pub failed_attempts: HashMap<Principal, u32>,
    pub lockout_duration: Duration,
}

impl AuthManager {
    pub fn authenticate_user(&mut self, principal: Principal, signature: &[u8]) -> Result<AuthToken, AuthError> {
        // Verify signature against challenge
        let challenge = self.get_challenge(&principal)?;
        self.verify_signature(&principal, signature, &challenge)?;
        
        // Check for lockout
        if self.is_locked_out(&principal) {
            return Err(AuthError::AccountLocked);
        }
        
        // Generate session token
        let token = AuthToken::new(principal, self.session_timeout);
        self.store_session(token.clone())?;
        
        Ok(token)
    }
}
```

### Data Protection and Encryption

**Sensitive Data Handling:**
- **BTC Private Keys**: Never stored in canister, handled by Bitcoin network
- **User Data**: All user-specific data encrypted with user's public key
- **Transaction Data**: Sensitive transaction details encrypted at rest
- **Price Data**: Oracle responses signed and verified for integrity

**Encryption Implementation:**
```rust
use ic_crypto_utils_threshold_sig_der::parse_threshold_sig_key;

pub struct EncryptionManager {
    pub master_key: Vec<u8>,
    pub user_keys: HashMap<Principal, Vec<u8>>,
}

impl EncryptionManager {
    pub fn encrypt_user_data(&self, user: &Principal, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let user_key = self.user_keys.get(user)
            .ok_or(CryptoError::KeyNotFound)?;
        
        // Use AES-256-GCM for symmetric encryption
        let cipher = Aes256Gcm::new_from_slice(user_key)
            .map_err(|_| CryptoError::InvalidKey)?;
        
        let nonce = generate_nonce();
        let ciphertext = cipher.encrypt(&nonce, data)
            .map_err(|_| CryptoError::EncryptionFailed)?;
        
        Ok([nonce.as_slice(), &ciphertext].concat())
    }
}
```

### Input Validation and Sanitization

**Comprehensive Validation Pipeline:**
```rust
pub struct ValidationPipeline {
    pub validators: Vec<Box<dyn Validator>>,
}

impl ValidationPipeline {
    pub fn validate_create_cdp(&self, params: &CreateCDPParams) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        
        // BTC amount validation
        if params.amount_satoshis < MIN_COLLATERAL_AMOUNT {
            errors.push(ValidationError::AmountTooSmall(params.amount_satoshis));
        }
        
        // BTC address validation
        if !self.validate_btc_address(&params.btc_address) {
            errors.push(ValidationError::InvalidAddress(params.btc_address.clone()));
        }
        
        // Transaction hash validation
        if !self.validate_tx_hash(&params.btc_tx_hash) {
            errors.push(ValidationError::InvalidHash(params.btc_tx_hash.clone()));
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
```

**SQL Injection Prevention:**
- No SQL databases used (ICP Stable Structures)
- All inputs strictly typed and validated
- No string concatenation for query construction
- Parameterized queries for all data access

### Access Control and Permissions

**Permission Matrix:**
| Operation | User | Liquidator | Admin | Emergency |
|-----------|------|------------|-------|-----------|
| Create CDP | ✅ | ❌ | ❌ | ✅ |
| Mint Bollar | ✅ | ❌ | ❌ | ✅ |
| Close CDP | ✅ | ❌ | ❌ | ✅ |
| Liquidate CDP | ❌ | ✅ | ✅ | ✅ |
| View Any CDP | ❌ | ✅ | ✅ | ✅ |
| Modify System | ❌ | ❌ | ✅ | ✅ |
| Emergency Pause | ❌ | ❌ | ❌ | ✅ |

**Dynamic Permission System:**
```rust
pub struct PermissionManager {
    pub user_permissions: HashMap<Principal, HashSet<Permission>>,
    pub role_permissions: HashMap<UserRole, HashSet<Permission>>,
    pub emergency_mode: bool,
}

impl PermissionManager {
    pub fn check_permission(&self, principal: &Principal, permission: Permission) -> bool {
        if self.emergency_mode {
            return self.check_emergency_permission(principal, permission);
        }
        
        let user_perms = self.user_permissions.get(principal);
        let role_perms = self.get_role_permissions(principal);
        
        user_perms.map_or(false, |perms| perms.contains(&permission)) ||
        role_perms.contains(&permission)
    }
}
```

## Performance & Scalability

### Performance Targets and Bottlenecks

**Performance Benchmarks:**
- **Query Latency**: < 100ms for all read operations
- **Update Latency**: < 500ms for all state-changing operations
- **Throughput**: 100 concurrent operations supported
- **Memory Usage**: < 4GB peak usage
- **Storage**: < 100MB for 10,000 active CDPs

**Identified Bottlenecks:**
1. **Price Oracle Updates**: Real-time price calculation across all CDPs
2. **Bitcoin Transaction Verification**: Network latency for BTC confirmations
3. **Runes Protocol Operations**: Bitcoin network transaction processing
4. **CDP State Synchronization**: Atomic updates across multiple data structures

### Caching Strategies

**Multi-Level Caching:**
```rust
pub struct CacheManager {
    pub price_cache: TimedCache<u64>,           // BTC price with 30s TTL
    pub liquidation_cache: TimedCache<Vec<u64>>, // Liquidatable CDPs with 10s TTL
    pub user_cache: TimedCache<UserCDPIndex>,   // User data with 60s TTL
    pub system_cache: TimedCache<SystemStats>,  // System stats with 300s TTL
}

impl CacheManager {
    pub fn get_price(&mut self) -> Option<u64> {
        self.price_cache.get(&"btc_price".to_string())
    }
    
    pub fn invalidate_price(&mut self) {
        self.price_cache.invalidate(&"btc_price".to_string());
    }
    
    pub fn warm_cache(&mut self) {
        // Pre-populate critical data
        self.price_cache.set("btc_price".to_string(), fetch_btc_price(), Duration::from_secs(30));
        self.liquidation_cache.set("liquidatable".to_string(), find_liquidatable_cdps(), Duration::from_secs(10));
    }
}
```

**Smart Caching Logic:**
- **Price Cache**: 30-second TTL with background refresh
- **Liquidation Cache**: 10-second TTL for real-time liquidation data
- **User Cache**: 60-second TTL with invalidation on user actions
- **System Cache**: 5-minute TTL for dashboard metrics

### Database Optimization

**Storage Optimization:**
```rust
// Compact storage for CDP data
#[derive(CandidType, Serialize, Deserialize)]
pub struct CompactCDP {
    pub id: u64,
    pub owner: [u8; 29], // Compressed Principal
    pub amount: u64,     // BTC satoshis
    pub minted: u64,     // Bollar cents
    pub ratio: u16,      // Collateral ratio (basis points)
    pub flags: u8,       // Status flags
}

impl CompactCDP {
    pub fn from_cdp(cdp: CDP) -> Self {
        CompactCDP {
            id: cdp.id,
            owner: compress_principal(&cdp.owner),
            amount: cdp.collateral_amount,
            minted: cdp.minted_bollar,
            ratio: cdp.collateral_ratio as u16,
            flags: pack_status_flags(&cdp.status),
        }
    }
}
```

**Index Optimization:**
- **Principal Index**: O(log n) lookup for user CDPs
- **Status Index**: O(log n) lookup for liquidatable CDPs
- **Time Index**: O(log n) lookup for recent CDPs
- **Amount Index**: O(log n) lookup for large CDPs

### Scaling Considerations

**Horizontal Scaling Strategy:**
```rust
// Future multi-canister architecture
pub struct ShardingManager {
    pub shard_count: u32,
    pub shard_mapping: HashMap<Principal, u32>,
    pub load_balancer: LoadBalancer,
}

impl ShardingManager {
    pub fn get_shard_for_user(&self, principal: &Principal) -> u32 {
        let hash = calculate_hash(principal);
        hash % self.shard_count
    }
    
    pub fn rebalance_shards(&mut self) {
        // Dynamic rebalancing based on load
        let load_distribution = self.calculate_load_distribution();
        self.shard_mapping = self.generate_new_mapping(load_distribution);
    }
}
```

**Scaling Phases:**
1. **Phase 1**: Single canister (MVP)
2. **Phase 2**: Vertical scaling with optimized storage
3. **Phase 3**: Horizontal sharding by user principal
4. **Phase 4**: Functional separation (CDP, Oracle, Liquidation)
5. **Phase 5**: Multi-region deployment

## Implementation Approach

### Development Phases and Priorities

**Phase 1: Core Protocol (Weeks 1-4)**
- Week 1: CDP creation and basic validation
- Week 2: Bollar minting with Runes integration
- Week 3: Price oracle integration and ratio calculations
- Week 4: CDP closure and BTC release

**Phase 2: Security & Liquidation (Weeks 5-6)**
- Week 5: Liquidation engine and penalty calculations
- Week 6: Security audits and penetration testing

**Phase 3: Frontend Integration (Weeks 7-8)**
- Week 7: React frontend with wallet integration
- Week 8: UI/UX polish and mobile optimization

**Phase 4: Testing & Deployment (Weeks 9-10)**
- Week 9: Comprehensive testing and bug fixes
- Week 10: Mainnet deployment and monitoring

### Testing Strategy Alignment

**Testing Pyramid:**
```
            E2E Tests (10%)
               /      \
         Integration (30%)
            /         \
      Unit Tests (60%)
```

**Test Coverage Requirements:**
- **Unit Tests**: 90% statement coverage, 85% branch coverage
- **Integration Tests**: All external service interactions
- **E2E Tests**: Complete user workflows from deposit to withdrawal

**Test Categories:**
```rust
#[cfg(test)]
mod tests {
    mod unit_tests {
        mod collateral_calculation_tests;
        mod price_validation_tests;
        mod liquidation_logic_tests;
        mod input_validation_tests;
    }
    
    mod integration_tests {
        mod oracle_integration_tests;
        mod runes_integration_tests;
        mod bitcoin_api_tests;
    }
    
    mod e2e_tests {
        mod complete_user_flow_tests;
        mod liquidation_scenario_tests;
        mod concurrent_operation_tests;
    }
}
```

### Deployment and Rollout Plan

**Deployment Environments:**
1. **Local Development**: dfx local replica
2. **Testnet**: ICP testnet with test BTC
3. **Staging**: Production-like environment
4. **Mainnet**: Production deployment with monitoring

**Deployment Pipeline:**
```yaml
# GitHub Actions workflow
name: Deploy Bollar Protocol

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run tests
        run: |
          cargo test --all-features
          npm test --prefix frontend/
  
  deploy-testnet:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to testnet
        run: dfx deploy --network ic-testnet
  
  deploy-mainnet:
    needs: [test, deploy-testnet]
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - name: Deploy to mainnet
        run: dfx deploy --network ic
```

**Monitoring and Alerting:**
- **Health Checks**: System availability every 30 seconds
- **Performance Metrics**: Response time, error rate, throughput
- **Business Metrics**: Total collateral, minted Bollar, liquidation count
- **Security Alerts**: Unusual activity, failed authentication attempts

**Rollback Strategy:**
- **Feature Flags**: Enable/disable features without redeployment
- **Blue-Green Deployment**: Zero-downtime upgrades
- **Emergency Pause**: Global emergency stop mechanism
- **Data Snapshots**: Regular state backups for recovery

This comprehensive technical design provides a complete blueprint for implementing the Bollar Money protocol, addressing all requirements while maintaining security, performance, and scalability. The design serves as the foundation for the structured development approach using the `/spec:tasks` methodology.