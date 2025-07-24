# Bollar Money User Stories Document

## User Role Definitions

### Primary User Roles
1. **Depositor**: Deposits BTC to obtain Bollar
2. **Liquidator**: Liquidates unhealthy CDPs for profit
3. **Observer**: Monitors system status and statistics

## User Stories List

### US-001: As a depositor, I want to deposit BTC as collateral

#### Story Description
As a BTC holder, I want to deposit BTC into the protocol as collateral to obtain Bollar stablecoins for other DeFi operations.

#### Preconditions
- User has Unisat wallet
- Wallet has sufficient BTC
- User has connected wallet to DApp

#### Main Flow
1. User connects Unisat wallet
2. User enters amount of BTC to deposit
3. System displays current BTC price and mintable Bollar amount
4. User confirms transaction
5. BTC transfers from user wallet to protocol address
6. System mints corresponding Bollar amount to user after confirmation

#### Boundary Conditions
- BTC minimum deposit amount: 0.001 BTC
- Maximum collateral ratio: 90%
- Price slippage tolerance: 1%

#### Acceptance Criteria
- After successful BTC deposit, user can see corresponding CDP in account
- System correctly calculates mintable Bollar amount
- BTC transfer confirmation time within reasonable range

### US-002: As a depositor, I want to mint Bollar against collateral

#### Story Description
As a CDP owner, I want to mint Bollar stablecoins against deposited BTC collateral.

#### Preconditions
- User has active CDP
- CDP collateral ratio above 90%

#### Main Flow
1. User selects CDP and clicks "Mint Bollar"
2. User enters amount of Bollar to mint
3. System validates post-mint collateral ratio not below 90%
4. User confirms transaction
5. System mints Bollar and sends to user wallet

#### Boundary Conditions
- Minimum mint amount: 0.01 Bollar
- Post-mint collateral ratio must be ≥90%

#### Acceptance Criteria
- After minting, user wallet receives correct Bollar amount
- CDP collateral ratio updates in real-time
- Transaction hash is queryable

### US-003: As a depositor, I want to close CDP and redeem BTC

#### Story Description
As a CDP owner, I want to return all borrowed Bollar and redeem my BTC collateral.

#### Preconditions
- User has active CDP
- CDP has sufficient Bollar debt

#### Main Flow
1. User selects CDP to close
2. System displays total Bollar amount to return (including interest if any)
3. User confirms return amount
4. User wallet authorizes Bollar transfer to protocol
5. After system confirms Bollar receipt, releases BTC collateral
6. BTC transfers back to user wallet

#### Boundary Conditions
- Must return all Bollar debt
- BTC transfer gas fee reservation

#### Acceptance Criteria
- BTC successfully returns to user wallet
- CDP status changes to closed
- All related balances update correctly

### US-004: As a liquidator, I want to view liquidatable CDP list

#### Story Description
As a liquidator, I want to see which CDPs have collateral ratios below liquidation threshold to profit from them.

#### Preconditions
- User has connected wallet
- Protocol has CDPs with collateral ratios below threshold

#### Main Flow
1. User visits liquidation page
2. System displays all CDPs with collateral ratio <85%
3. Each CDP shows: collateral amount, debt amount, current collateral ratio, liquidation reward
4. User can select any CDP for liquidation

#### Boundary Conditions
- Liquidation reward: 5% of collateral
- Liquidation threshold: 85% collateral ratio

#### Acceptance Criteria
- List updates liquidatable CDPs in real-time
- Liquidation reward calculation is accurate
- Interface display is clear and understandable

### US-005: As a liquidator, I want to execute liquidation for profit

#### Story Description
As a liquidator, I want to execute liquidation when CDP collateral ratio is too low, obtaining collateral at discount.

#### Preconditions
- CDP collateral ratio below 85%
- Liquidator wallet has sufficient Bollar

#### Main Flow
1. Liquidator selects CDP to liquidate
2. System displays Bollar amount to pay
3. Liquidator confirms liquidation operation
4. System deducts Bollar from liquidator wallet
5. Liquidator receives collateral BTC (with 5% reward)
6. CDP status updates to liquidated

#### Boundary Conditions
- Liquidator must pay all debt
- Liquidation reward distributed immediately

#### Acceptance Criteria
- Liquidator correctly receives collateral + reward
- CDP status updates correctly
- System total collateral ratio remains healthy

### US-006: As an observer, I want to view overall system status

#### Story Description
As an observer, I want to view the overall operation status and statistics of the Bollar Money protocol.

#### Display Data
- Total collateral BTC amount
- Total minted Bollar amount
- Current BTC price
- System collateral ratio
- Active CDP count
- Liquidation statistics

#### Acceptance Criteria
- Data updates in real-time
- Interface responds quickly
- Data display is clear

## Boundary User Stories

### US-007: Price Anomaly Handling

#### Story Description
When Oracle prices show abnormal fluctuations, the system should pause related operations to protect user funds.

#### Scenarios
- BTC price fluctuation exceeds 10%/hour
- Oracle service unavailable
- Price data delay exceeds 5 minutes

#### Handling Methods
- Pause new minting operations
- Allow CDP closing and BTC redemption
- Send alert notifications to users

### US-008: Minimum Collateral Protection

#### Story Description
Prevent users from creating CDPs that are too small, avoiding uneconomical gas fees.

#### Rules
- Minimum BTC collateral: 0.001 BTC
- Minimum Bollar mint: 0.01 Bollar

## User Journey Map

### Typical User Journey: Alice's First Use

#### Stage 1: Exploration (0-5 minutes)
1. Alice visits Bollar Money website
2. Connects Unisat wallet
3. Views system information and current BTC price
4. Learns about collateral ratios and liquidation rules

#### Stage 2: First Deposit (5-15 minutes)
1. Alice decides to deposit 0.1 BTC as collateral
2. System shows can mint ~9,000 Bollar (at current price)
3. Alice confirms transaction and sends BTC
4. Waits for BTC network confirmation
5. Alice mints 7,000 Bollar (conservative choice)

#### Stage 3: Monitoring (Ongoing)
1. Alice periodically checks her CDP status
2. Monitors BTC price changes impact on collateral ratio
3. BTC price rises, collateral ratio improves

#### Stage 4: Exit (Anytime)
1. Alice decides to close CDP to redeem all BTC
2. Returns 7,000 Bollar to protocol
3. Receives 0.1 BTC back to wallet

## Non-functional Requirements

### Performance Requirements
- Page load time < 3 seconds
- API response time < 500ms
- BTC transfer confirmation wait time display clear

### Security Requirements
- All transactions require explicit user confirmation
- Sensitive operations display risk warnings
- Private keys never leave user wallet

### User Experience Requirements
- Interface simple and intuitive
- Important information prominently displayed
- Error messages clear and understandable
- Good mobile adaptation

## Testing Scenarios

### Scenario 1: Normal Collateral Flow Testing
- Deposit 0.01 BTC, verify minting calculation
- Check collateral ratio updates
- Verify BTC receipt confirmation

### Scenario 2: Boundary Condition Testing
- Deposit minimum BTC amount
- Attempt minting beyond 90% collateral ratio
- Verify system rejects invalid operations

### Scenario 3: Liquidation Scenario Testing
- Simulate BTC price drop triggering liquidation
- Verify liquidator profit mechanism
- Check CDP status changes

### Scenario 4: Frontend Integration Testing
- Unisat wallet connection testing
- Transaction signature flow testing
- Error handling testing