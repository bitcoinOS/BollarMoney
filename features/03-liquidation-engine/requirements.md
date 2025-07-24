# Liquidation Engine Feature Requirements

## Overview
Feature for automatically liquidating undercollateralized CDPs when collateral ratio drops below safety thresholds.

## Functional Requirements

### Liquidation Triggers
**WHEN** a CDP's collateral ratio falls below 85% **THEN** the system SHALL allow any user to trigger liquidation with 5% penalty to the liquidator and burn the minted Bollar.

**WHILE** a CDP has ratio < 85% **THEN** the system SHALL mark it as liquidatable and make it available for liquidation processing.

### Liquidation Process
**WHEN** liquidation is triggered **THEN** the system SHALL:
1. Calculate the 5% liquidation penalty
2. Burn all minted Bollar tokens from the CDP
3. Transfer the penalty amount to the liquidator
4. Return remaining BTC collateral to the original owner
5. Mark the CDP as liquidated

### Batch Processing
**WHERE** multiple CDPs are eligible for liquidation **THEN** the system SHALL process liquidations efficiently in batch operations.

### Liquidator Incentives
**WHERE** liquidation is triggered **THEN** the liquidator SHALL receive 5% of the CDP's BTC collateral as reward for maintaining system health.

## Non-Functional Requirements

### Performance
- Liquidation processing SHALL complete within 500ms per CDP
- System SHALL handle up to 100 concurrent liquidation triggers
- Batch liquidation SHALL process multiple CDPs efficiently

### Fairness
- Liquidation SHALL be available to any user on first-come-first-served basis
- System SHALL prevent front-running through proper transaction ordering
- Liquidation rewards SHALL be calculated transparently

### System Health
**WHILE** liquidation occurs **THEN** the system SHALL maintain overall system solvency
- Total Bollar supply SHALL decrease proportionally to liquidated debt
- System SHALL maintain sufficient BTC backing for remaining Bollar

## Error Handling
- **CDPNotFound**: When attempting to liquidate non-existent CDP
- **CDPAlreadyLiquidated**: When attempting to liquidate already liquidated CDP
- **CDPNotLiquidatable**: When CDP ratio is still above 85%
- **OraclePriceError**: When price data is unavailable for ratio calculation
- **InsufficientCollateral**: When CDP has no remaining collateral

## Testing Requirements
- Boundary testing at exactly 85% ratio
- Multiple liquidation scenarios with varying collateral amounts
- Concurrent liquidation attempts
- System solvency verification after liquidations
- Liquidator reward calculation accuracy
- Edge cases: zero collateral, maximum penalty scenarios