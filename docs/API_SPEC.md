# Bollar Money API Specification Document

## API Overview

### Basic Information
- **Protocol**: HTTPS/HTTP
- **Data Format**: JSON
- **Authentication**: Unisat Wallet + IC-siwb
- **Network**: Internet Computer

## Core API Endpoints

### 1. System Information

#### GET /system/info
Get basic system information

**Request Parameters**: None

**Response**:
```json
{
  "version": "1.0.0",
  "max_collateral_ratio": 9000,
  "liquidation_threshold": 8500,
  "liquidation_penalty": 500,
  "btc_price": 65000,
  "total_collateral": 100000000,
  "total_minted": 90000000,
  "system_health": "healthy"
}
```

### 2. Collateral Management

#### POST /collateral/deposit
Deposit BTC as collateral

**Request**:
```json
{
  "btc_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
  "amount": 1000000  // satoshis
}
```

**Response**:
```json
{
  "collateral_id": 123,
  "btc_amount": 1000000,
  "btc_price": 65000,
  "max_mintable": 58500000,
  "collateral_ratio": 9000
}
```

#### GET /collateral/{id}
Get collateral details

**Response**:
```json
{
  "id": 123,
  "owner": "principal-xyz",
  "amount": 1000000,
  "minted_bollar": 50000000,
  "collateral_ratio": 7800,
  "liquidation_price": 55000,
  "status": "active"
}
```

#### POST /collateral/{id}/mint
Mint Bollar against collateral

**Request**:
```json
{
  "amount": 50000000  // 0.5 Bollar
}
```

**Response**:
```json
{
  "tx_hash": "0xabc...",
  "new_collateral_ratio": 7800,
  "remaining_capacity": 8500000
}
```

#### POST /collateral/{id}/close
Close CDP and redeem collateral

**Request**:
```json
{
  "bollar_amount": 50000000
}
```

**Response**:
```json
{
  "success": true,
  "btc_returned": 1000000,
  "tx_hash": "0xdef..."
}
```

### 3. Price Information

#### GET /price/btc
Get current BTC price

**Response**:
```json
{
  "price": 65000,
  "timestamp": 1699123456,
  "source": "icp_oracle",
  "confidence": 0.99
}
```

### 4. Liquidation Functions

#### GET /liquidations/available
Get list of liquidatable CDPs

**Response**:
```json
{
  "liquidations": [
    {
      "collateral_id": 456,
      "owner": "principal-abc",
      "collateral_amount": 500000,
      "minted_amount": 40000000,
      "current_ratio": 8200,
      "liquidation_threshold": 8500,
      "reward": 25000
    }
  ]
}
```

#### POST /liquidations/{id}/execute
Execute liquidation

**Request**:
```json
{
  "bollar_amount": 40000000
}
```

**Response**:
```json
{
  "success": true,
  "collateral_received": 500000,
  "reward": 25000,
  "tx_hash": "0xghi..."
}
```

### 5. User Account

#### GET /account/{principal}/positions
Get all CDPs for a user

**Response**:
```json
{
  "total_collateral": 1500000,
  "total_minted": 120000000,
  "average_ratio": 8000,
  "positions": [
    {
      "id": 123,
      "amount": 1000000,
      "minted": 80000000,
      "ratio": 8125,
      "status": "active"
    }
  ]
}
```

## Error Handling

### Error Codes
- `400`: Parameter error
- `401`: Unauthorized
- `403`: Insufficient permissions
- `404`: Resource not found
- `409`: State conflict
- `422`: Business logic error
- `500`: Internal error

### Error Response Format
```json
{
  "error": {
    "code": "INSUFFICIENT_COLLATERAL",
    "message": "Insufficient collateral, cannot mint more Bollar",
    "details": {
      "current_ratio": 8200,
      "required_ratio": 9000
    }
  }
}
```

## Real-time Events

### WebSocket Events
- `price_update`: Price update
- `liquidation_alert`: Liquidation alert
- `collateral_change`: Collateral change

### Event Format
```json
{
  "type": "price_update",
  "data": {
    "asset": "BTC",
    "old_price": 64000,
    "new_price": 65000,
    "change": 0.0156
  },
  "timestamp": 1699123456
}
```

## Rate Limiting
- **Anonymous users**: 100 requests/minute
- **Authenticated users**: 1000 requests/minute
- **WebSocket**: 10 concurrent connections

## Version Control
- **Current version**: v1.0
- **Version identifier**: HTTP Header `X-API-Version: 1.0`
- **Backward compatibility**: Support minor version updates