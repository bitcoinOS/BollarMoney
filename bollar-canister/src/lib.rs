//! Bollar Money - Bitcoin-collateralized stablecoin protocol on ICP

mod types;
mod price;
mod errors;
mod cdp;
mod mint;
mod liquidation;
mod closure;
mod health;

use candid::{CandidType, Deserialize, Principal};
use ic_cdk::{query, update};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use serde::{Deserialize as SerdeDeserialize, Serialize};
use std::cell::RefCell;

use types::*;
use price::*;
use cdp::*;
use mint::*;
use liquidation::*;
use closure::*;
use health::*;

// Memory management
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );
    
    static CDPS: RefCell<StableBTreeMap<u64, CDP, VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))),
        )
    );
    
    static USER_CDPS: RefCell<StableBTreeMap<Principal, Vec<u64>, VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))),
        )
    );
    
    static SYSTEM_CONFIG: RefCell<SystemConfig> = RefCell::new(SystemConfig::default());
    
    static PRICE_ORACLE: RefCell<PriceOracle> = RefCell::new(
        PriceOracle::new(OracleConfig::default())
    );
    
    static NEXT_CDP_ID: RefCell<u64> = RefCell::new(1);
}

/// Initialize the canister
#[ic_cdk::init]
fn init() {
    ic_cdk::print("Bollar Money protocol initialized");
}

/// Get system configuration
#[query]
fn get_system_config() -> SystemConfig {
    SYSTEM_CONFIG.with(|config| config.borrow().clone())
}

/// Get system health information
#[query]
fn get_system_health() -> SystemHealth {
    let btc_price = PRICE_ORACLE.with(|oracle| oracle.borrow().get_cached_price().unwrap_or(65_000_000));
    
    CDPS.with(|cdps| {
        let cdps_ref = cdps.borrow();
        let active_cdps: Vec<_> = cdps_ref.iter().filter(|(_, cdp)| !cdp.is_liquidated).collect();
        
        let total_collateral: u64 = active_cdps.iter().map(|(_, cdp)| cdp.collateral_amount).sum();
        let total_minted: u64 = active_cdps.iter().map(|(_, cdp)| cdp.minted_amount).sum();
        
        let average_ratio = if total_minted > 0 {
            let total_ratio: u64 = active_cdps.iter()
                .map(|(_, cdp)| cdp.calculate_collateral_ratio(btc_price) as u64)
                .sum();
            (total_ratio / active_cdps.len() as u64) as u32
        } else {
            0
        };
        
        SystemHealth {
            total_collateral_satoshis: total_collateral,
            total_minted_cents: total_minted,
            average_collateral_ratio: average_ratio,
            active_cdps_count: active_cdps.len() as u64,
            btc_price_cents: btc_price,
            system_utilization_ratio: 0, // Calculate based on total minted vs total collateral
        }
    })
}

/// Get a specific CDP
#[query]
fn get_cdp(cdp_id: u64) -> Option<CDP> {
    CDPS.with(|cdps| cdps.borrow().get(&cdp_id))
}

/// Get user's CDPs
#[query]
fn get_user_cdps(user: Principal) -> Vec<CDP> {
    USER_CDPS.with(|user_cdps| {
        user_cdps.borrow()
            .get(&user)
            .unwrap_or_default()
            .iter()
            .filter_map(|cdp_id| CDPS.with(|cdps| cdps.borrow().get(cdp_id)))
            .collect()
    })
}

/// Get current BTC price
#[query]
fn get_btc_price() -> u64 {
    PRICE_ORACLE.with(|oracle| oracle.borrow().get_cached_price().unwrap_or(65_000_000))
}

/// Update BTC price (admin function)
#[update]
fn update_btc_price(price_cents: u64) -> Result<(), ProtocolError> {
    // In production, this would be called by oracle canister
    // For now, allow admin to update for testing
    let caller = ic_cdk::caller();
    if caller != ic_cdk::id() {
        return Err(ProtocolError::UnauthorizedAccess);
    }
    
    PRICE_ORACLE.with(|oracle| {
        oracle.borrow_mut().cache = Some(PriceCache::new(
            price_cents,
            "admin".to_string(),
            95,
            300,
        ));
    });
    
    Ok(())
}

/// Create a new CDP with BTC collateral
#[update]
fn create_cdp(btc_address: String, amount_satoshis: u64) -> ApiResponse<u64> {
    let caller = ic_cdk::caller();
    let config = SYSTEM_CONFIG.with(|c| c.borrow().clone());
    let btc_price = PRICE_ORACLE.with(|oracle| {
        oracle.borrow_mut().get_btc_price()
            .unwrap_or_else(|_| 65_000_000)
    });
    
    // Validate BTC address
    if let Err(e) = cdp::validate_btc_address(&btc_address) {
        return ApiResponse::error(e);
    }
    
    // Create CDP
    match cdp::create_cdp_logic(caller, amount_satoshis, btc_price, &config) {
        Ok(mut new_cdp) => {
            // Assign CDP ID
            let cdp_id = NEXT_CDP_ID.with(|next_id| {
                let id = *next_id.borrow();
                *next_id.borrow_mut() += 1;
                id
            });
            
            new_cdp.id = cdp_id;
            
            // Store CDP
            CDPS.with(|cdps| {
                cdps.borrow_mut().insert(cdp_id, new_cdp.clone());
            });
            
            // Update user's CDP list
            USER_CDPS.with(|user_cdps| {
                let mut user_cdps_ref = user_cdps.borrow_mut();
                let mut user_list = user_cdps_ref.get(&caller).unwrap_or_default();
                user_list.push(cdp_id);
                user_cdps_ref.insert(caller, user_list);
            });
            
            // Increment tracking counter
            cdp::TOTAL_CREATE_EVENTS.with(|counter| {
                *counter.borrow_mut() += 1;
            });
            
            ic_cdk::print(format!("CDP {} created for user {} with {} satoshis", 
                         cdp_id, caller, amount_satoshis));
            
            ApiResponse::success(cdp_id)
        }
        Err(e) => ApiResponse::error(e),
    }
}

/// Get CDP creation preview without actually creating
#[query]
fn get_cdp_preview(collateral_amount: u64) -> ApiResponse<CdpPreview> {
    let config = SYSTEM_CONFIG.with(|c| c.borrow().clone());
    let btc_price = PRICE_ORACLE.with(|oracle| {
        oracle.borrow_mut().get_btc_price()
            .unwrap_or_else(|_| 65_000_000)
    });
    
    match cdp::get_cdp_preview(collateral_amount, btc_price, &config) {
        Ok(preview) => ApiResponse::success(preview),
        Err(e) => ApiResponse::error(e),
    }
}

/// Mint Bollar against existing CDP collateral
#[update]
fn mint_bollar(cdp_id: u64, amount_cents: u64) -> ApiResponse<u64> {
    mint::mint_bollar(cdp_id, amount_cents)
}

/// Get mint preview for a CDP without actual minting
#[query]  
fn get_mint_preview(cdp_id: u64, amount_cents: u64) -> ApiResponse<MintPreview> {
    mint::get_mint_preview(cdp_id, amount_cents)
}

/// Liquidate an undercollateralized CDP
#[update]
fn liquidate_cdp(cdp_id: u64) -> ApiResult<LiquidationResult> {
    let caller = ic_cdk::caller();
    let config = SYSTEM_CONFIG.with(|c| c.borrow().clone());
    let btc_price = PRICE_ORACLE.with(|oracle| {
        oracle.borrow_mut().get_btc_price()
            .unwrap_or_else(|_| 65_000_000)
    });

    CDPS.with(|cdps| {
        let mut cdps_ref = cdps.borrow_mut();
        let mut cdp = match cdps_ref.get(&cdp_id) {
            Some(cdp) => cdp,
            None => return ApiResponse::error(ProtocolError::CDPNotFound(cdp_id)),
        };

        // Don't check ownership - anyone can liquidate undercollateralized CDPs
        match liquidation::execute_liquidation_safe(
            cdp.clone(), 
            btc_price, 
            500, // 5% penalty
            500  // 5% reward
        ) {
            Ok(liquidation_result) => {
                cdps_ref.insert(cdp_id, liquidation_result.cdp.clone());
                ic_cdk::print(format!(
                    "CDP {} liquidated by {} with reward {} satoshis",
                    cdp_id, caller, liquidation_result.liquidator_reward_satoshis
                ));
                ApiResponse::success(liquidation_result)
            }
            Err(e) => ApiResponse::error(e),
        }
    })
}

/// Get liquidation preview for a CDP
#[query]
fn get_liquidation_preview(cdp_id: u64) -> ApiResult<LiquidationPreview> {
    let btc_price = PRICE_ORACLE.with(|oracle| {
        oracle.borrow_mut().get_btc_price()
            .unwrap_or_else(|_| 65_000_000)
    });

    CDPS.with(|cdps| {
        let cdps_ref = cdps.borrow();
        let cdp = match cdps_ref.get(&cdp_id) {
            Some(cdp) => cdp,
            None => return ApiResponse::error(ProtocolError::CDPNotFound(cdp_id)),
        };

        let amounts = liquidation::calculate_liquidation_amounts(
            &cdp, btc_price, 500, 500
        );

        Ok(LiquidationPreview {
            is_eligible: liquidation::should_liquidate(&cdp, btc_price, 8500),
            current_ratio: cdp.calculate_collateral_ratio(btc_price),
            total_repayment: amounts.total_repayment,
            penalty_amount: amounts.penalty_amount,
            liquidator_reward_satoshis: amounts.liquidator_reward_satoshis,
            protocol_fee_satoshis: amounts.remaining_collateral_satoshis,
        })
    })
}

/// Find all liquidatable CDPs
#[query]
fn find_liquidatable_cdps() -> Vec<u64> {
    let btc_price = PRICE_ORACLE.with(|oracle| {
        oracle.borrow_mut().get_btc_price()
            .unwrap_or_else(|_| 65_000_000)
    });

    liquidation::find_liquidatable_cdps(btc_price, 8500)
}

/// Close a CDP and redeem BTC collateral
#[update]
fn close_cdp(cdp_id: u64, repayment_amount: u64) -> ApiResult<ClosureResult> {
    let caller = ic_cdk::caller();
    let config = SYSTEM_CONFIG.with(|c| c.borrow().clone());
    let btc_price = PRICE_ORACLE.with(|oracle| {
        oracle.borrow_mut().get_btc_price()
            .unwrap_or_else(|_| 65_000_000)
    });

    CDPS.with(|cdps| {
        let mut cdps_ref = cdps.borrow_mut();
        let mut cdp = match cdps_ref.get(&cdp_id) {
            Some(cdp) => cdp,
            None => return ApiResponse::error(ProtocolError::CDPNotFound(cdp_id)),
        };

        match closure::execute_closure_safe(
            cdp.clone(),
            repayment_amount,
            btc_price,
            100 // 1% closure fee
        ) {
            Ok(closure_result) => {
                cdps_ref.insert(cdp_id, closure_result.cdp.clone());
                
                // Remove CDP from user's list
                USER_CDPS.with(|user_cdps| {
                    let mut user_cdps_ref = user_cdps.borrow_mut();
                    let mut user_list = user_cdps_ref.get(&caller).unwrap_or_default();
                    user_list.retain(|&id| id != cdp_id);
                    user_cdps_ref.insert(caller, user_list);
                });
                
                ic_cdk::print(format!(
                    "CDP {} closed by {} with {} satoshis redeemed",
                    cdp_id, caller, closure_result.redemption_amount_satoshis
                ));
                ApiResponse::success(closure_result)
            }
            Err(e) => ApiResponse::error(e),
        }
    })
}

/// Get closure preview for a CDP
#[query]
fn get_closure_preview(cdp_id: u64) -> ApiResult<ClosurePreview> {
    let btc_price = PRICE_ORACLE.with(|oracle| {
        oracle.borrow_mut().get_btc_price()
            .unwrap_or_else(|_| 65_000_000)
    });

    CDPS.with(|cdps| {
        let cdps_ref = cdps.borrow();
        let cdp = match cdps_ref.get(&cdp_id) {
            Some(cdp) => cdp,
            None => return ApiResponse::error(ProtocolError::CDPNotFound(cdp_id)),
        };

        let preview = closure::calculate_closure_preview(&cdp, btc_price, 100 // 1% closure fee
        );
        ApiResponse::success(preview)
    })
}

/// Get comprehensive system health information
#[query]
fn get_system_health() -> ApiResult<SystemHealthDetailed> {
    let config = SYSTEM_CONFIG.with(|c| c.borrow().clone());
    let btc_price = PRICE_ORACLE.with(|oracle| {
        oracle.borrow_mut().get_btc_price()
            .unwrap_or_else(|_| 65_000_000)
    });

    let mut health = health::calculate_system_health(btc_price, &config);
    health.calculate_health_score(&config);
    
    ApiResponse::success(health)
}

/// Get system health summary
#[query]
fn get_system_health_summary() -> ApiResult<SystemHealth> {
    let config = SYSTEM_CONFIG.with(|c| c.borrow().clone());
    let btc_price = PRICE_ORACLE.with(|oracle| {
        oracle.borrow_mut().get_btc_price()
            .unwrap_or_else(|_| 65_000_000)
    });

    let health = health::calculate_system_health(btc_price, &config);
    ApiResponse::success(SystemHealth {
        total_collateral_satoshis: health.total_collateral_satoshis,
        total_minted_cents: health.total_minted_cents,
        average_collateral_ratio: health.average_collateral_ratio,
        active_cdps_count: health.active_cdps_count,
        btc_price_cents: health.btc_price_cents,
        system_utilization_ratio: health.utilization_ratio(),
    })
}

/// Get detailed metrics for a specific CDP
#[query]
fn get_cdp_metrics(cdp_id: u64) -> ApiResult<CDPMetrics> {
    let btc_price = PRICE_ORACLE.with(|oracle| {
        oracle.borrow_mut().get_btc_price()
            .unwrap_or_else(|_| 65_000_000)
    });

    CDPS.with(|cdps| {
        let cdps_ref = cdps.borrow();
        let cdp = match cdps_ref.get(&cdp_id) {
            Some(cdp) => cdp,
            None => return ApiResponse::error(ProtocolError::CDPNotFound(cdp_id)),
        };

        let metrics = CDPMetrics::new(cdp_id, &cdp, btc_price);
        ApiResponse::success(metrics)
    })
}

/// Get protocol analytics and statistics
#[query]
fn get_protocol_analytics() -> ApiResult<ProtocolAnalytics> {
    let analytics = health::calculate_protocol_analytics();
    ApiResponse::success(analytics)
}

// Export candid interface
ic_cdk::export_candid!();