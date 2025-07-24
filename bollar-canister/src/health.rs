//! System health monitoring and analytics for Bollar Money protocol

use crate::types::*;
use candid::{CandidType, Deserialize};

/// Comprehensive system health information
#[derive(Debug, Clone, CandidType, Deserialize)]
pub struct SystemHealthDetailed {
    pub total_collateral_satoshis: u64,
    pub total_minted_cents: u64,
    pub average_collateral_ratio: u32,
    pub active_cdps_count: u64,
    pub liquidated_cdps_count: u64,
    pub closed_cdps_count: u64,
    pub btc_price_cents: u64,
    pub system_utilization_ratio: u32,
    pub risk_assessment: RiskLevel,
    pub health_score: u32, // 0-1000 (basis points for precision)
    pub total_fees_collected_satoshis: u64,
    pub total_liquidation_penalties_satoshis: u64,
    pub total_closure_fees_satoshis: u64,
    pub protocol_revenue_satoshis: u64,
    pub last_updated: u64,
}

/// Risk assessment levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, CandidType, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl SystemHealthDetailed {
    pub fn new() -> Self {
        Self {
            total_collateral_satoshis: 0,
            total_minted_cents: 0,
            average_collateral_ratio: 10000, // 100% default
            active_cdps_count: 0,
            liquidated_cdps_count: 0,
            closed_cdps_count: 0,
            btc_price_cents: 65000000, // Default BTC price
            system_utilization_ratio: 0,
            risk_assessment: RiskLevel::Low,
            health_score: 1000, // Perfect score initially
            total_fees_collected_satoshis: 0,
            total_liquidation_penalties_satoshis: 0,
            total_closure_fees_satoshis: 0,
            protocol_revenue_satoshis: 0,
            last_updated: 0,
        }
    }

    pub fn calculate_health_score(&mut self, config: &SystemConfig) {
        let mut score = 1000_u32;
        
        // Risk factors based on system utilization
        if self.system_utilization_ratio > 8000 { // >80% utilization
            score = score.saturating_sub(400); // High risk
            self.risk_assessment = RiskLevel::Critical;
        } else if self.system_utilization_ratio > 7000 { // >70% utilization
            score = score.saturating_sub(200); // Medium risk
            self.risk_assessment = RiskLevel::High;
        } else if self.system_utilization_ratio > 5000 { // >50% utilization
            score = score.saturating_sub(100); // Low risk
            self.risk_assessment = RiskLevel::Medium;
        } else {
            self.risk_assessment = RiskLevel::Low;
        }

        // Risk factors based on average collateral ratio
        if self.average_collateral_ratio < 12000 { // <120% average ratio
            score = score.saturating_sub(150);
            if self.risk_assessment == RiskLevel::Low {
                self.risk_assessment = RiskLevel::Medium;
            }
        }
        
        if self.average_collateral_ratio < 10000 { // <100% average ratio
            score = score.saturating_sub(300);
            self.risk_assessment = RiskLevel::Critical;
        }

        self.health_score = score;
    }

    pub fn utilization_ratio(&self) -> u32 {
        if self.total_collateral_satoshis == 0 || self.btc_price_cents == 0 {
            return 0;
        }
        
        let collateral_value_cents = self.total_collateral_satoshis * self.btc_price_cents / 100_000_000;
        if collateral_value_cents == 0 {
            return 0;
        }
        
        (self.total_minted_cents * 10_000 / collateral_value_cents) as u32
    }
}

/// CDP performance metrics for individual CDPs
#[derive(Debug, Clone, CandidType, Deserialize)]
pub struct CDPMetrics {
    pub cdp_id: u64,
    pub current_collateral_ratio: u32,
    pub unrealized_pnl_cents: i64,
    pub days_active: u64,
    pub liquidation_risk_score: u32, // 0-100 risk score
    pub time_to_liquidation_hours: Option<u64>,
    pub health_status: CDPHealthStatus,
}

#[derive(Debug, Clone, CandidType, Deserialize)]
pub enum CDPHealthStatus {
    Healthy,
    AtRisk,     // 85-90% collateral ratio
    Warning,    // 80-85% collateral ratio
    Critical,   // Below 80% collateral ratio
    Liquidated,
    Closed,
}

impl CDPMetrics {
    pub fn new(cdp_id: u64, cdp: &CDP, btc_price_cents: u64) -> Self {
        let current_ratio = if cdp.minted_amount > 0 {
            cdp.calculate_collateral_ratio(btc_price_cents)
        } else {
            u32::MAX
        };

        let unrealized_pnl = if cdp.minted_amount > 0 {
            let collateral_value = cdp.collateral_amount * btc_price_cents / 100_000_000;
            collateral_value as i64 - cdp.minted_amount as i64
        } else {
            0
        };

        let liquidation_risk_score = Self::calculate_liquidation_risk(current_ratio);
        let health_status = Self::determine_health_status(current_ratio, cdp.is_liquidated);

        Self {
            cdp_id,
            current_collateral_ratio: current_ratio,
            unrealized_pnl_cents: unrealized_pnl,
            days_active: 0, // Will be calculated from timestamps
            liquidation_risk_score,
            time_to_liquidation_hours: None, // Simplified for MVP
            health_status,
        }
    }

    fn calculate_liquidation_risk(collateral_ratio: u32) -> u32 {
        match collateral_ratio {
            0..=8000 => 100, // Critical risk
            8001..=8500 => 85, // High risk
            8501..=9000 => 60, // Medium risk
            9001..=9500 => 30, // Low risk
            9501..=10000 => 15, // Minimal risk
            _ => 0, // Safe
        }
    }

    fn determine_health_status(current_ratio: u32, is_liquidated: bool) -> CDPHealthStatus {
        if is_liquidated {
            CDPHealthStatus::Liquidated
        } else if current_ratio <= 8000 {
            CDPHealthStatus::Critical
        } else if current_ratio <= 8500 {
            CDPHealthStatus::Warning
        } else if current_ratio <= 9000 {
            CDPHealthStatus::AtRisk
        } else {
            CDPHealthStatus::Healthy
        }
    }
}

/// Protocol analytics and statistics
#[derive(Debug, Clone, CandidType, Deserialize)]
pub struct ProtocolAnalytics {
    pub total_protocol_revenue_satoshis: u64,
    pub daily_active_users: u64,
    pub weekly_active_users: u64,
    pub monthly_active_users: u64,
    pub average_deposit_size_satoshis: u64,
    pub average_debt_size_cents: u64,
    pub most_active_hour: u32,
    pub peak_collateral_ratio: u32,
    pub lowest_collateral_ratio: u32,
    pub total_transactions_count: u64,
    pub liquidation_events_count: u64,
    pub closure_events_count: u64,
    pub mint_events_count: u64,
    pub create_events_count: u64,
}

impl ProtocolAnalytics {
    pub fn new() -> Self {
        Self {
            total_protocol_revenue_satoshis: 0,
            daily_active_users: 0,
            weekly_active_users: 0,
            monthly_active_users: 0,
            average_deposit_size_satoshis: 0,
            average_debt_size_cents: 0,
            most_active_hour: 0,
            peak_collateral_ratio: 0,
            lowest_collateral_ratio: u32::MAX,
            total_transactions_count: 0,
            liquidation_events_count: 0,
            closure_events_count: 0,
            mint_events_count: 0,
            create_events_count: 0,
        }
    }
}

/// Calculate comprehensive system health
pub fn calculate_system_health(btc_price_cents: u64, config: &SystemConfig) -> SystemHealthDetailed {
    use crate::cdp::CDPS;
    use crate::closure::TOTAL_CLOSURE_FEES;
    use crate::liquidation::TOTAL_LIQUIDATION_PENALTIES;
    use crate::mint::TOTAL_PROTOCOL_REVENUE;
    use ic_cdk::api::time;

    let mut total_collateral = 0u64;
    let mut total_minted = 0u64;
    let mut active_count = 0u64;
    let mut liquidated_count = 0u64;
    let mut closed_count = 0u64;
    let mut total_ratios = 0u64;
    let mut ratio_count = 0u64;

    // Aggregate CDP data
    CDPS.with(|cdps| {
        let cdps_ref = cdps.borrow();
        for (_, cdp) in cdps_ref.iter() {
            total_collateral += cdp.collateral_amount;
            total_minted += cdp.minted_amount;
            
            if cdp.is_liquidated {
                if cdp.minted_amount == 0 {
                    closed_count += 1;
                } else {
                    liquidated_count += 1;
                }
            } else {
                active_count += 1;
                if cdp.minted_amount > 0 {
                    total_ratios += cdp.calculate_collateral_ratio(btc_price_cents) as u64;
                    ratio_count += 1;
                }
            }
        }
    });

    let average_ratio = if ratio_count > 0 {
        (total_ratios / ratio_count) as u32
    } else {
        10000 // 100% default
    };

    let mut health = SystemHealthDetailed::new();
    health.total_collateral_satoshis = total_collateral;
    health.total_minted_cents = total_minted;
    health.average_collateral_ratio = average_ratio;
    health.active_cdps_count = active_count;
    health.liquidated_cdps_count = liquidated_count;
    health.closed_cdps_count = closed_count;
    health.btc_price_cents = btc_price_cents;
    health.system_utilization_ratio = health.utilization_ratio();
    health.last_updated = time() / 1_000_000_000; // Convert to seconds

    // Calculate fees collected
    health.total_fees_collected_satoshis = TOTAL_CLOSURE_FEES.with(|fees| fees.borrow().clone());
    health.total_liquidation_penalties_satoshis = TOTAL_LIQUIDATION_PENALTIES.with(|penalties| penalties.borrow().clone());
    health.total_closure_fees_satoshis = TOTAL_CLOSURE_FEES.with(|fees| fees.borrow().clone());
    health.protocol_revenue_satoshis = TOTAL_PROTOCOL_REVENUE.with(|revenue| revenue.borrow().clone());

    health.calculate_health_score(config);
    health
}

/// Calculate protocol analytics
pub fn calculate_protocol_analytics() -> ProtocolAnalytics {
    use crate::cdp::CDPS;
    use crate::closure::TOTAL_CLOSURE_EVENTS;
    use crate::liquidation::TOTAL_LIQUIDATION_EVENTS;
    use crate::mint::TOTAL_MINT_EVENTS;
    use crate::cdp::TOTAL_CREATE_EVENTS;

    let mut analytics = ProtocolAnalytics::new();
    let mut total_deposits = 0u64;
    let mut deposit_count = 0u64;
    let mut total_debts = 0u64;
    let mut debt_count = 0u64;
    let mut unique_users = std::collections::HashSet::new();
    let mut max_ratio = 0u32;
    let mut min_ratio = u32::MAX;

    // Collect CDP data
    CDPS.with(|cdps| {
        let cdps_ref = cdps.borrow();
        for (_, cdp) in cdps_ref.iter() {
            total_deposits += cdp.collateral_amount;
            deposit_count += 1;
            
            if cdp.minted_amount > 0 {
                total_debts += cdp.minted_amount;
                debt_count += 1;
            }
            
            unique_users.insert(cdp.owner);
            
            if !cdp.is_liquidated {
                let ratio = cdp.calculate_collateral_ratio(65000000); // Use default price
                max_ratio = max_ratio.max(ratio);
                min_ratio = min_ratio.min(ratio);
            }
        }
    });

    // Update analytics
    analytics.total_protocol_revenue_satoshis = 
        TOTAL_CLOSURE_EVENTS.with(|events| events.borrow().clone()) + 
        TOTAL_LIQUIDATION_EVENTS.with(|events| events.borrow().clone());
    
    analytics.unique_users_count = unique_users.len() as u64;
    analytics.total_transactions_count = 
        TOTAL_CREATE_EVENTS.with(|events| events.borrow().clone()) +
        TOTAL_MINT_EVENTS.with(|events| events.borrow().clone()) +
        TOTAL_LIQUIDATION_EVENTS.with(|events| events.borrow().clone()) +
        TOTAL_CLOSURE_EVENTS.with(|events| events.borrow().clone());

    analytics.liquidation_events_count = TOTAL_LIQUIDATION_EVENTS.with(|events| events.borrow().clone());
    analytics.closure_events_count = TOTAL_CLOSURE_EVENTS.with(|events| events.borrow().clone());
    analytics.mint_events_count = TOTAL_MINT_EVENTS.with(|events| events.borrow().clone());
    analytics.create_events_count = TOTAL_CREATE_EVENTS.with(|events| events.borrow().clone());

    analytics.average_deposit_size_satoshis = if deposit_count > 0 {
        total_deposits / deposit_count
    } else {
        0
    };

    analytics.average_debt_size_cents = if debt_count > 0 {
        total_debts / debt_count
    } else {
        0
    };

    analytics.peak_collateral_ratio = max_ratio;
    analytics.lowest_collateral_ratio = if min_ratio == u32::MAX { 0 } else { min_ratio };

    analytics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CDP;
    use candid::Principal;

    fn test_cdp(collateral: u64, minted: u64, is_liquidated: bool) -> CDP {
        CDP {
            id: 1,
            owner: Principal::anonymous(),
            collateral_amount: collateral,
            minted_amount: minted,
            created_at: 1699123456,
            updated_at: 1699123456,
            is_liquidated,
        }
    }

    #[test]
    fn test_system_health_utilization_ratio() {
        let mut health = SystemHealthDetailed::new();
        health.total_collateral_satoshis = 1_000_000; // 0.01 BTC
        health.total_minted_cents = 500_000; // $500
        health.btc_price_cents = 65_000_000; // $65,000
        
        let ratio = health.utilization_ratio();
        assert_eq!(ratio, 769); // ~7.69% utilization
    }

    #[test]
    fn test_system_health_score_calculation() {
        let mut health = SystemHealthDetailed::new();
        let config = SystemConfig::default();
        
        // Test low risk scenario
        health.system_utilization_ratio = 3000; // 30% utilization
        health.average_collateral_ratio = 15000; // 150% ratio
        health.calculate_health_score(&config);
        assert_eq!(health.health_score, 1000);
        assert_eq!(health.risk_assessment, RiskLevel::Low);

        // Test high risk scenario
        health.system_utilization_ratio = 8500; // 85% utilization
        health.average_collateral_ratio = 8000; // 80% ratio
        health.calculate_health_score(&config);
        assert_eq!(health.health_score, 300); // Reduced score
        assert_eq!(health.risk_assessment, RiskLevel::Critical);
    }

    #[test]
    fn test_cdp_metrics_health_status() {
        let btc_price = 65_000_000;
        let mut cdp = test_cdp(1_000_000, 500_000, false);
        let metrics = CDPMetrics::new(1, &cdp, btc_price);
        assert_eq!(metrics.health_status, CDPHealthStatus::Healthy);
        assert_eq!(metrics.current_collateral_ratio, 13000);

        // Test critical status
        cdp.minted_amount = 850_000; // ~76.47% ratio
        let critical_metrics = CDPMetrics::new(2, &cdp, btc_price);
        assert_eq!(critical_metrics.health_status, CDPHealthStatus::Critical);
    }

    #[test]
    fn test_cdp_metrics_unrealized_pnl() {
        let btc_price = 65_000_000;
        let cdp = test_cdp(1_000_000, 500_000, false); // 0.01 BTC collateral, $500 debt
        let metrics = CDPMetrics::new(1, &cdp, btc_price);
        assert_eq!(metrics.unrealized_pnl_cents, 150_000); // $650 - $500 = $150 profit
    }

    #[test]
    fn test_empty_system_health() {
        let health = SystemHealthDetailed::new();
        assert_eq!(health.total_collateral_satoshis, 0);
        assert_eq!(health.total_minted_cents, 0);
        assert_eq!(health.active_cdps_count, 0);
        assert_eq!(health.health_score, 1000);
    }
}