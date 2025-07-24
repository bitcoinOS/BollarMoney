# CDP Creation Feature Requirements

## Overview
Feature for creating and managing Collateralized Debt Positions (CDPs) where users deposit BTC as collateral.

## Functional Requirements

### CDP Creation
**WHEN** a user deposits BTC collateral between 0.001-1000 BTC **THEN** the system SHALL create a CDP with unique ID and calculate maximum mintable amount using 90% collateral ratio.

**WHERE** BTC addresses are provided **THEN** the system SHALL validate P2PKH, P2SH, or Bech32 address formats before processing deposits.

**WHERE** BTC amounts are provided **THEN** the system SHALL validate minimum 0.001 BTC and maximum 1000 BTC limits and reject invalid amounts.

### CDP State Management
**WHILE** a CDP exists **THEN** the system SHALL continuously monitor the collateral ratio based on current BTC price and mark CDPs with ratio < 85% as liquidatable.

**WHEN** a user requests CDP information **THEN** the system SHALL provide current collateral amount, minted Bollar, and current collateral ratio.

### Access Control
**WHERE** a CDP exists **THEN** only the original owner SHALL be able to view or modify the CDP state.

## Non-Functional Requirements

### Performance
- CDP creation queries SHALL complete within 100ms
- System SHALL support 100 concurrent CDP operations

### Security
- System SHALL prevent unauthorized access to user CDPs
- All BTC deposits SHALL be validated for authenticity
- System SHALL maintain atomic state consistency during CDP creation

## Error Handling
- **InvalidAmount**: When BTC amount is below 0.001 or above 1000 BTC
- **InvalidAddress**: When BTC address format is invalid
- **CDPAlreadyExists**: When user already has an active CDP
- **OraclePriceError**: When BTC price data is unavailable

## Testing Requirements
- Boundary testing for BTC amount limits (0.001, 1000 BTC)
- Invalid address format validation
- Concurrent CDP creation scenarios
- Access control verification