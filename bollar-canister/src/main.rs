// Main entry point for Bollar Money canister
use ic_cdk_macros::*;

mod types;
mod price;
mod errors;
mod cdp;
mod mint;
mod liquidation;
mod closure;
mod health;

use types::*;

#[init]
fn init() {
    ic_cdk::println!("Bollar Money Canister initialized");
    
    // Initialize system config
    let config = SystemConfig::default();
    ic_cdk::storage::stable_save((config,)).unwrap();
}

#[pre_upgrade]
fn pre_upgrade() {
    ic_cdk::println!("Pre-upgrade hook called");
}

#[post_upgrade]
fn post_upgrade() {
    ic_cdk::println!("Post-upgrade hook called");
}

#[query]
fn health_check() -> String {
    "Bollar Money Canister is healthy".to_string()
}

// Export candid interface
ic_cdk::export_candid!();