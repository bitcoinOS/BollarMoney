# Bollar Money - Bitcoin-Collateralized Stablecoin Protocol

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![ICP](https://img.shields.io/badge/Built%20on-Internet%20Computer-blue)](https://internetcomputer.org/)
[![Bitcoin](https://img.shields.io/badge/Collateral-Bitcoin-orange)](https://bitcoin.org/)

<img src="images/bollar-logo-word.png" width="66%">

### What's `Bollar`  <img src="images/bollar-logo.png" width="6%">

## Overview

Bollar Money is a decentralized stablecoin protocol that enables users to mint USD-pegged stablecoins (Bollar) by depositing Bitcoin as collateral on the Internet Computer (ICP) blockchain. Built with Chain Fusion technology, the protocol combines the security of Bitcoin with the programmability of ICP to create a trustless, censorship-resistant stablecoin system.

### Key Features

- **Bitcoin-Backed Stability**: Mint Bollar stablecoins by locking BTC as collateral
- **Chain Fusion Integration**: Native Bitcoin integration without bridges or wrapped tokens
- **Decentralized Oracle**: Real-time BTC/USD price feeds from ICP's native oracle
- **Automated Liquidation**: Algorithmic liquidation of undercollateralized positions
- **Runes Standard**: Built on Bitcoin's Runes protocol for enhanced compatibility
- **Non-Custodial**: Users maintain full control of their collateral

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Bollar Protocol                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐    ┌─────────────────┐                │
│  │   Frontend      │    │  Bollar         │                │
│  │   (React/TS)    │◄───┤  Canister       │                │
│  │                 │    │  (Rust/ICP)     │                │
│  └─────────────────┘    └─────────────────┘                │
│         ▲                        ▲                          │
│         │                        │                          │
│         │                   ┌────┴────┐                     │
│         │                   │         │                     │
│  ┌──────┴──────┐    ┌──────┴──────┐ │                     │
│  │   Unisat    │    │   ICP       │ │   ┌─────────────┐   │
│  │   Wallet    │    │   Oracle    │ │   │   Bitcoin   │   │
│  │             │    │             │ │   │   Network   │   │
│  └─────────────┘    └─────────────┘ │   └─────────────┘   │
│                                     │                      │
└─────────────────────────────────────────────────────────────┘
```

## Tech Stack

### Backend (ICP Canister)
- **Language**: Rust
- **Framework**: ICP CDK 0.16.x
- **Target**: WebAssembly (wasm32-unknown-unknown)
- **Storage**: StableBTreeMap for persistent storage
- **Testing**: cargo-test, ic-test-state-machine

### Frontend
- **Framework**: React with TypeScript
- **Wallet**: Unisat integration for Bitcoin operations
- **Build**: Vite for development and production builds
- **Testing**: Jest for unit tests, Playwright for E2E

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install DFX (ICP SDK)
DFX_VERSION=0.15.0 sh -ci "$(curl -fsSL https://sdk.dfinity.org/install.sh)"

# Install Node.js (v18+)
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs
```

### Local Development

```bash
# Clone the repository
git clone https://github.com/bifipal/bollar-money.git
cd bollar-money

# Start local ICP network
dfx start --clean --background

# Deploy canisters locally
dfx deploy

# Install frontend dependencies
cd frontend
npm install

# Start frontend development server
npm run dev
```

### Building from Source

#### Backend (Rust Canister)
```bash
cd bollar-canister

# Build for local development
cargo build

# Build release for deployment
cargo build --release --target wasm32-unknown-unknown

# Run all tests
cargo test

# Generate Candid interface
cargo test --test candid_generation
```

#### Frontend (React)
```bash
cd frontend

# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build

# Run tests
npm test
npm run test:e2e
```

## Protocol Mechanics

### Creating a CDP

1. **Deposit BTC**: Send BTC to the protocol's Bitcoin address
2. **Create CDP**: Call `create_cdp()` with your Bitcoin transaction details
3. **Mint Bollar**: Use `mint_bollar()` to create stablecoins against your collateral

### Collateral Ratio Calculation

```
Collateral Ratio = (BTC Value in USD) / (Minted Bollar Amount)

Example:
- BTC Price: $50,000
- BTC Deposited: 1 BTC
- Collateral Value: $50,000
- Maximum Bollar Mint: $45,000 (90% LTV)
- Liquidation Threshold: 85% ($42,500)
```

### Liquidation Process

- **Trigger**: When collateral ratio drops below 85%
- **Penalty**: 5% liquidation fee
- **Process**: Anyone can liquidate undercollateralized positions
- **Reward**: Liquidators receive liquidation bonus

## API Reference

### Core Canister Methods

#### `create_cdp(btc_address: text, amount_satoshis: nat64) -> Result<CDPId, Error>`
Create a new collateralized debt position.

#### `mint_bollar(cdp_id: CDPId, amount: nat64) -> Result<(), Error>`
Mint Bollar stablecoins against existing collateral.

#### `close_cdp(cdp_id: CDPId) -> Result<(), Error>`
Close CDP and retrieve collateral after repaying Bollar.

#### `liquidate_cdp(cdp_id: CDPId) -> Result<(), Error>`
Liquidate an undercollateralized CDP.

#### `get_btc_price() -> Result<PriceData, Error>`
Get current BTC/USD price from oracle.

### Query Methods

#### `get_cdp(cdp_id: CDPId) -> Result<CDP, Error>`
Get detailed information about a specific CDP.

#### `get_system_health() -> Result<SystemHealth, Error>`
Get overall protocol statistics.

#### `get_user_cdps(user: Principal) -> Result<Vec<CDP>, Error>`
Get all CDPs owned by a specific user.

## Configuration

### Environment Variables

```bash
# Frontend
VITE_CANISTER_ID=ryjl3-tyaaa-aaaaa-aaaba-cai
VITE_NETWORK=local
VITE_BTC_NETWORK=testnet

# Canister
ORACLE_CANISTER_ID=ryjl3-tyaaa-aaaaa-aaaba-cai
MAX_COLLATERAL_RATIO=90
LIQUIDATION_THRESHOLD=85
LIQUIDATION_PENALTY=5
```

### Network Configuration

#### Local Development
```json
{
  "networks": {
    "local": {
      "bind": "127.0.0.1:8000",
      "type": "ephemeral"
    }
  }
}
```

#### ICP Testnet
```bash
dfx deploy --network ic
```

#### ICP Mainnet
```bash
dfx deploy --network ic --with-cycles 1000000000000
```

## Testing

### Running Tests

```bash
# Backend tests
cd bollar-canister
cargo test

# Frontend tests
cd frontend
npm test
npm run test:e2e

# Integration tests
cd bollar-canister
cargo test --test integration_tests
```

### Test Coverage

```bash
# Generate coverage report
cd bollar-canister
cargo tarpaulin --out html --output-dir coverage

# View coverage report
open coverage/index.html
```

### Manual Testing

```bash
# Check canister health
dfx canister call bollar_canister get_system_health

# Create test CDP
dfx canister call bollar_canister create_cdp '(record { btc_address="tb1qexample"; amount_satoshis=100000 })'

# Mint test Bollar
dfx canister call bollar_canister mint_bollar '(record { cdp_id=1; amount=5000 })'
```

## Security

### Audit Status
- **Internal Audit**: Completed
- **External Audit**: In progress (Trail of Bits)
- **Bug Bounty**: Immunefi program launching Q2 2024

### Security Features

- **Reentrancy Protection**: All state-changing functions protected
- **Input Validation**: Comprehensive bounds checking on all inputs
- **Oracle Security**: Multi-source price validation with time delays
- **Access Control**: Role-based permissions for critical functions
- **Emergency Pause**: Circuit breaker mechanism for emergency situations

### Security Best Practices

1. **Never share private keys or seed phrases**
2. **Verify contract addresses before transactions**
3. **Monitor collateral ratios regularly**
4. **Use hardware wallets for large positions**
5. **Test with small amounts first**

## Monitoring & Analytics

### Key Metrics
- **Total Value Locked (TVL)**: Total BTC locked in protocol
- **Bollar Supply**: Total Bollar stablecoins in circulation
- **Active CDPs**: Number of open positions
- **Liquidation Events**: Historical liquidation data
- **System Utilization**: Collateral ratio distribution

### Monitoring Commands

```bash
# Check system health
dfx canister call bollar_canister get_system_health

# Monitor CDP stats
dfx canister call bollar_canister get_total_stats

# Check oracle price
dfx canister call bollar_canister get_btc_price
```

## Deployment Checklist

### Pre-deployment
- [ ] All tests passing (`cargo test`, `npm test`)
- [ ] Security audit completed
- [ ] Performance benchmarks met
- [ ] Documentation updated
- [ ] Emergency procedures tested

### Mainnet Deployment
- [ ] Deploy canisters with sufficient cycles
- [ ] Configure production oracle endpoints
- [ ] Set up monitoring and alerts
- [ ] Deploy frontend to CDN
- [ ] Update DNS and SSL certificates
- [ ] Conduct final integration testing

## Contributing

### Development Workflow

1. **Fork the repository**
2. **Create feature branch**: `git checkout -b feature/amazing-feature`
3. **Make changes**: Follow existing code style
4. **Add tests**: Ensure comprehensive test coverage
5. **Run checks**: `cargo fmt`, `cargo clippy`, `npm run lint`
6. **Submit PR**: Include detailed description

### Code Style

#### Rust
```rust
// Use snake_case for functions and variables
fn calculate_collateral_ratio(collateral: u64, debt: u64) -> f64 {
    // Always include bounds checking
    if debt == 0 { return f64::MAX; }
    collateral as f64 / debt as f64
}
```

#### TypeScript
```typescript
// Use camelCase for variables and functions
interface CDP {
  id: bigint;
  owner: Principal;
  collateralAmount: bigint;
  mintedAmount: bigint;
}
```

### Pull Request Process

1. **Update documentation** for any API changes
2. **Add tests** for new functionality
3. **Ensure CI passes** all checks
4. **Request review** from maintainers
5. **Address feedback** promptly

## Troubleshooting

### Common Issues

#### Canister Deployment Fails
```bash
# Check canister status
dfx canister status bollar_canister

# Reinstall if necessary
dfx canister install bollar_canister --mode reinstall
```

#### BTC Transaction Issues
- Ensure testnet BTC for testing
- Verify transaction confirmations
- Check address format (Bech32 recommended)

#### Oracle Price Stale
```bash
# Manually trigger price update
dfx canister call bollar_canister update_price
```

### Getting Help

- **Discord**: [Bollar Money Community](https://discord.gg/bollar)
- **GitHub Issues**: [Report bugs here](https://github.com/bifipal/bollar-money/issues)
- **Documentation**: [Full docs](https://docs.bollar.money)
- **Telegram**: [@bollarmoney](https://t.me/bollarmoney)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- **DFINITY Foundation**: ICP blockchain technology
- **Bitcoin Core**: Underlying cryptocurrency
- **Unisat Wallet**: Bitcoin wallet integration
- **Trail of Bits**: Security auditing
- **Open Source Community**: Contributors and testers

---

**Built with ❤️ by the Bollar Money team**

[Website](https://bollar.money) | [Docs](https://docs.bollar.money) | [Discord](https://discord.gg/bollar) | [Twitter](https://twitter.com/bollarmoney)