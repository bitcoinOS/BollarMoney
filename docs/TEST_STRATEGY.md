# Bollar Money Testing Strategy Document

## Testing Philosophy

### TDD Priority Principle
- Write test cases for each feature before implementing code
- Test-driven design ensures code testability
- User stories → Test cases → Implementation code

### Testing Pyramid
- **Unit Tests** (70%): Function-level logic testing
- **Integration Tests** (20%): Canister interaction testing
- **End-to-End Tests** (10%): Complete user flow testing

## Testing Environment

### Testing Networks
- **Local**: dfx local environment
- **Testnet**: ICP test network
- **Bitcoin Testnet**: BTC Testnet4

### Testing Tools
- **Rust Testing Framework**: cargo test
- **ICP Testing**: dfx + ic-test-state-machine
- **Frontend Testing**: Jest + React Testing Library
- **Integration Testing**: Playwright E2E testing

## Unit Testing Strategy

### Core Module Testing

#### 1. Price Calculation Module Tests
```rust
#[cfg(test)]
mod price_calculation_tests {
    use super::*;
    
    #[test]
    fn test_calculate_mintable_amount() {
        let btc_amount = 1_000_000; // 0.01 BTC in sats
        let btc_price = 65_000_000; // $65,000 in cents
        let max_ratio = 9000; // 90%
        
        let mintable = calculate_mintable_amount(btc_amount, btc_price, max_ratio);
        assert_eq!(mintable, 5_850_000); // 5850 cents = $58.50
    }
    
    #[test]
    fn test_calculate_collateral_ratio() {
        let collateral_value = 65_000_000; // $650 in cents
        let minted_amount = 50_000_000; // $500 in cents
        
        let ratio = calculate_collateral_ratio(collateral_value, minted_amount);
        assert_eq!(ratio, 13000); // 130% collateralization
    }
}
```

#### 2. Liquidation Judgment Tests
```rust
#[cfg(test)]
mod liquidation_tests {
    use super::*;
    
    #[test]
    fn test_should_liquidate_below_threshold() {
        let collateral_value = 80_000_000; // $800
        let minted_amount = 100_000_000; // $1000
        let liquidation_threshold = 8500; // 85%
        
        assert!(should_liquidate(collateral_value, minted_amount, liquidation_threshold));
    }
    
    #[test]
    fn test_should_not_liquidate_above_threshold() {
        let collateral_value = 100_000_000; // $1000
        let minted_amount = 100_000_000; // $1000
        let liquidation_threshold = 8500;
        
        assert!(!should_liquidate(collateral_value, minted_amount, liquidation_threshold));
    }
}
```

### Boundary Condition Testing

#### Numerical Boundary Tests
- Minimum BTC deposit amount (0.001 BTC)
- Maximum collateral ratio (90%) boundary
- Zero value handling
- Large number overflow protection

#### Asset Precision Tests
- Satoshi precision (8 decimal places)
- Bollar precision (2 decimal places)
- Price precision (cents, 2 decimal places)

## Integration Testing Strategy

### Oracle Integration Testing
```rust
#[test]
fn test_oracle_price_fetching() {
    // Mock ICP Oracle response
    let mock_oracle = setup_mock_oracle();
    let price = fetch_btc_price(&mock_oracle).await.unwrap();
    
    assert!(price > 0);
    assert!(price < 1_000_000_000); // Reasonable price range
}
```

### Runes Integration Testing
```rust
#[test]
fn test_runes_token_operations() {
    let token_id = create_bollar_token();
    assert!(token_id.is_valid());
    
    let mint_result = mint_bollar(token_id, 1_000_000);
    assert!(mint_result.is_ok());
}
```

## User Story Based Test Cases

### US-001: BTC Deposit Testing

#### Normal Flow Testing
```rust
#[test]
fn test_successful_btc_deposit() {
    // Given
    let user = create_test_user();
    let deposit_amount = 100_000; // 0.001 BTC
    let btc_price = 65_000_000;
    
    // When
    let result = deposit_collateral(user, deposit_amount, btc_price);
    
    // Then
    assert!(result.is_ok());
    assert_eq!(result.unwrap().max_mintable, 58_500_000); // $585
}
```

#### Boundary Testing
```rust
#[test]
fn test_minimum_deposit_amount() {
    let result = deposit_collateral(user, 99_999, price); // Below minimum
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::AmountTooSmall);
}
```

### US-002: Bollar Minting Testing

#### Normal Flow Testing
```rust
#[test]
fn test_successful_bollar_minting() {
    let collateral_id = create_collateral(100_000_000, 65_000_000); // 1 BTC
    let mint_amount = 50_000_000; // $500
    
    let result = mint_bollar(collateral_id, mint_amount);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().collateral_ratio, 13000); // 130%
}
```

#### Boundary Testing
```rust
#[test]
fn test_exceeding_max_collateral_ratio() {
    let collateral_id = create_collateral(100_000_000, 65_000_000);
    let mint_amount = 60_000_000; // Would exceed 90% ratio
    
    let result = mint_bollar(collateral_id, mint_amount);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::InsufficientCollateral);
}
```

### US-005: Liquidation Testing

#### Liquidation Trigger Testing
```rust
#[test]
fn test_liquidation_trigger() {
    let collateral_id = create_collateral(100_000_000, 65_000_000); // 1 BTC, $6500
    mint_bollar(collateral_id, 5_000_000_00).unwrap(); // Mint $5000
    
    // Simulate price drop to $4300
    mock_oracle.set_price(4_300_000_00);
    update_all_ratios();
    
    let liquidation_list = get_liquidatable_positions();
    assert!(liquidation_list.contains(&collateral_id));
}
```

## End-to-End Testing Strategy

### Playwright Testing Scenarios

#### E2E Test Cases
1. **Complete User Journey Test**: Alice's entire usage flow
2. **Wallet Connection Test**: Unisat wallet integration
3. **Transaction Flow Test**: Deposit → Mint → Redeem

#### Test Script Example
```typescript
// frontend/e2e/user-journey.spec.ts
test('complete user journey', async ({ page }) => {
  await page.goto('/');
  await connectWallet(page, 'unisat');
  
  // Deposit BTC
  await depositBTC(page, 0.01);
  await expect(page.locator('text=CDP created')).toBeVisible();
  
  // Mint Bollar
  await mintBollar(page, 500);
  await expect(page.locator('text=500 BOLLAR minted')).toBeVisible();
  
  // Close CDP
  await closeCDP(page);
  await expect(page.locator('text=0.01 BTC returned')).toBeVisible();
});
```

## Performance Testing

### Load Testing Scenarios
- Concurrent CDP creation (100 simultaneous creations)
- Batch price update testing
- Memory usage monitoring

### Stress Testing Metrics
- Canister call latency < 500ms
- Memory usage < 4GB
- CPU usage peak monitoring

## Security Testing

### Attack Scenario Testing
1. **Reentrancy Attack Test**: Multiple call validation protection mechanism
2. **Overflow Testing**: Large numerical input handling
3. **Permission Bypass Testing**: Non-owner CDP operation validation
4. **Oracle Manipulation Testing**: Malicious price input validation

### Fuzz Testing
```bash
# Use cargo-fuzz for fuzz testing
cargo fuzz run deposit_fuzzer
cargo fuzz run mint_fuzzer
cargo fuzz run liquidate_fuzzer
```

## Test Data Management

### Test Data Sets
- BTC price data: Historical price fluctuation testing
- Boundary value data sets: Minimum/maximum value testing
- Abnormal input data: Format errors, malicious data testing

### Mock Data Services
- Mock Oracle service
- Mock Bitcoin network
- Mock Unisat wallet connection

## Test Coverage Targets

### Coverage Requirements
- **Statement Coverage**: ≥90%
- **Branch Coverage**: ≥85%
- **Function Coverage**: ≥95%
- **Line Coverage**: ≥90%

### Coverage Report
```bash
# Generate coverage report
cargo tarpaulin --out html --output-dir coverage
```

## Continuous Integration Testing

### GitHub Actions Workflow
```yaml
name: Test Suite
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run unit tests
        run: cargo test
      - name: Run integration tests
        run: cargo test --test integration_test
      - name: Run E2E tests
        run: npm run test:e2e
```

## Testing Priorities

### P0 (Must Cover)
- Core calculation logic (collateral ratio, liquidation price)
- Basic CRUD operations
- Boundary condition handling
- Error handling mechanisms

### P1 (Important Coverage)
- Complex business logic
- Concurrent scenarios
- Performance bottlenecks
- Security vulnerability points

### P2 (Optional Enhancement)
- Extreme boundary conditions
- Long-term running stability
- Memory leak detection
- Performance benchmark testing

## Testing Milestones

### Week 1: Basic Testing Framework
- Unit testing framework setup
- Mock service creation
- Basic test case writing

### Week 2: Core Logic Testing
- Price calculation testing
- Collateral ratio calculation testing
- Liquidation logic testing

### Week 3: Integration Testing
- Oracle integration testing
- Runes integration testing
- End-to-end scenario testing

### Week 4: Complete Testing Suite
- All user story testing
- Performance testing
- Security testing
- Coverage verification