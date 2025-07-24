# Bollar Minting Feature Requirements

## Overview
Feature for minting Bollar stablecoins against BTC collateral using Runes protocol.

## Functional Requirements

### Bollar Minting
**WHEN** a user requests to mint Bollar against their CDP collateral **THEN** the system SHALL verify the requested amount maintains ≥90% collateral ratio after minting and mint tokens using Runes protocol.

**WHERE** minting is requested **THEN** the system SHALL calculate the maximum mintable amount as: (collateral_value * 0.9) - already_minted_amount.

### Bollar Burning
**WHEN** a user provides sufficient Bollar tokens to fully close their CDP **THEN** the system SHALL burn the Bollar tokens and release the BTC collateral to the original owner.

**WHERE** partial burning is requested **THEN** the system SHALL allow burning any amount up to the total minted Bollar, maintaining ≥90% collateral ratio.

### Runes Protocol Integration
**WHERE** Bollar tokens are minted **THEN** the system SHALL use the Runes protocol to create fungible tokens with proper metadata (name: "Bollar", symbol: "BOLL", decimals: 8).

**WHEN** Bollar tokens are burned **THEN** the system SHALL properly destroy Runes tokens and update the total supply.

### Balance Tracking
**WHILE** Bollar is minted **THEN** the system SHALL maintain accurate tracking of per-user balances and total system supply.

## Non-Functional Requirements

### Performance
- Minting operations SHALL complete within 500ms
- Burning operations SHALL complete within 500ms
- Balance queries SHALL complete within 100ms

### Accuracy
- All calculations SHALL maintain 8 decimal places for Bollar amounts
- System SHALL ensure mathematical precision in collateral ratio calculations
- Runes protocol integration SHALL maintain token supply consistency

### Security
- System SHALL prevent minting beyond collateral limits
- All minting/burning SHALL be atomic operations
- System SHALL validate Runes protocol compatibility

## Error Handling
- **InsufficientCollateral**: When requested mint amount exceeds available collateral ratio
- **CDPNotFound**: When accessing a non-existent CDP
- **CDPAlreadyLiquidated**: When attempting to mint against liquidated CDPs
- **InvalidAmount**: When mint amount is zero or negative
- **UnauthorizedAccess**: When attempting to mint against another user's CDP

## Testing Requirements
- Boundary testing for minting at 90% collateral ratio
- Partial burning scenarios
- Runes protocol integration testing
- Concurrent minting/burning operations
- Mathematical accuracy validation for ratio calculations