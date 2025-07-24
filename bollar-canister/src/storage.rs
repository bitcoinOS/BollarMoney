//! Storage Layer Optimization - Task 1.4 Implementation
//! Efficient CDP storage with user indexing and atomic operations

use crate::types::*;
use crate::cdp_creation::*;
use candid::Principal;
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    storable::{bounded::BoundedStorable, Storable},
    DefaultMemoryImpl, StableBTreeMap,
};
use std::borrow::Cow;

// Memory configuration
const CDP_MEMORY_ID: MemoryId = MemoryId::new(0);
const USER_INDEX_MEMORY_ID: MemoryId = MemoryId::new(1);
const METADATA_MEMORY_ID: MemoryId = MemoryId::new(2);

// Memory types
type Memory = VirtualMemory<DefaultMemoryImpl>;

// Implement Storable for CDP
impl Storable for CDP {
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut bytes = Vec::new();
        // Serialize CDP to bytes
        bytes.extend_from_slice(&self.id.to_le_bytes());
        bytes.extend_from_slice(&self.owner.as_slice().len().to_le_bytes());
        bytes.extend_from_slice(self.owner.as_slice());
        bytes.extend_from_slice(&self.collateral_amount.to_le_bytes());
        bytes.extend_from_slice(&self.minted_amount.to_le_bytes());
        bytes.extend_from_slice(&self.created_at.to_le_bytes());
        bytes.extend_from_slice(&self.updated_at.to_le_bytes());
        bytes.push(self.is_liquidated as u8);
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let bytes = bytes.as_ref();
        let mut offset = 0;

        let id = u64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
        offset += 8;

        let owner_len = u64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;
        offset += 8;

        let owner_slice = &bytes[offset..offset + owner_len];
        let owner = Principal::from_slice(owner_slice);
        offset += owner_len;

        let collateral_amount = u64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
        offset += 8;

        let minted_amount = u64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
        offset += 8;

        let created_at = u64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
        offset += 8;

        let updated_at = u64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
        offset += 8;

        let is_liquidated = bytes[offset] != 0;

        CDP {
            id,
            owner,
            collateral_amount,
            minted_amount,
            created_at,
            updated_at,
            is_liquidated,
        }
    }
}

impl BoundedStorable for CDP {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// Implement Storable for Vec<u64> (user CDP list)
impl Storable for Vec<u64> {
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(self.len() as u64).to_le_bytes());
        for &id in self {
            bytes.extend_from_slice(&id.to_le_bytes());
        }
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let bytes = bytes.as_ref();
        let mut offset = 0;
        
        let len = u64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;
        offset += 8;

        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            let id = u64::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]);
            result.push(id);
            offset += 8;
        }
        result
    }
}

impl BoundedStorable for Vec<u64> {
    const MAX_SIZE: u32 = 2048; // Max 256 CDPs per user
    const IS_FIXED_SIZE: bool = false;
}

/// Storage manager for CDP data
pub struct CdpStorage {
    cdps: StableBTreeMap<u64, CDP, Memory>,
    user_cdps: StableBTreeMap<Principal, Vec<u64>, Memory>,
    next_cdp_id: u64,
}

impl CdpStorage {
    pub fn new(memory_manager: &MemoryManager) -> Self {
        Self {
            cdps: StableBTreeMap::init(memory_manager.get(CDP_MEMORY_ID)),
            user_cdps: StableBTreeMap::init(memory_manager.get(USER_INDEX_MEMORY_ID)),
            next_cdp_id: 1,
        }
    }

    /// Create CDP with atomic storage operations
    pub fn create_cdp(
        &mut self,
        owner: Principal,
        cdp: CDP,
    ) -> Result<u64, ProtocolError> {
        let cdp_id = self.next_cdp_id;
        let mut cdp = cdp;
        cdp.id = cdp_id;

        // Atomic operation: store CDP and update user index
        self.cdps.insert(cdp_id, cdp);
        
        let mut user_cdp_list = self.user_cdps.get(&owner).unwrap_or_default();
        user_cdp_list.push(cdp_id);
        self.user_cdps.insert(owner, user_cdp_list);
        
        self.next_cdp_id += 1;
        
        Ok(cdp_id)
    }

    /// Get CDP by ID
    pub fn get_cdp(&self,
        cdp_id: u64,
    ) -> Option<CDP> {
        self.cdps.get(&cdp_id)
    }

    /// Get all CDPs for a user
    pub fn get_user_cdps(&self,
        owner: Principal,
    ) -> Vec<CDP> {
        let user_cdp_ids = self.user_cdps.get(&owner).unwrap_or_default();
        user_cdp_ids
            .into_iter()
            .filter_map(|id| self.get_cdp(id))
            .collect()
    }

    /// Get CDPs with pagination
    pub fn get_cdps_paginated(
        &self,
        start_id: Option<u64>,
        limit: usize,
    ) -> Vec<CDP> {
        let start = start_id.unwrap_or(1);
        let end = start + limit as u64;
        
        (start..end)
            .filter_map(|id| self.get_cdp(id))
            .collect()
    }

    /// Get total CDP count
    pub fn get_total_cdps(&self,
    ) -> u64 {
        self.next_cdp_id.saturating_sub(1)
    }

    /// Update CDP state (for minting, liquidation, etc.)
    pub fn update_cdp(
        &mut self,
        cdp_id: u64,
        updater: impl FnOnce(&mut CDP,
        ) -> Result<(), ProtocolError>,
    ) -> Result<(), ProtocolError> {
        let mut cdp = self.cdps
            .get(&cdp_id)
            .ok_or(ProtocolError::CDPNotFound)?;
        
        updater(&mut cdp)?;
        cdp.updated_at = crate::utils::current_time();
        
        self.cdps.insert(cdp_id, cdp);
        Ok(())
    }

    /// Get liquidatable CDPs
    pub fn get_liquidatable_cdps(
        &self,
        btc_price_cents: u64,
        config: &SystemConfig,
    ) -> Vec<CDP> {
        self.cdps
            .iter()
            .filter(|(_, cdp)| !cdp.is_liquidated)
            .filter(|(_, cdp)| {
                let collateral_value = cdp.collateral_amount * btc_price_cents / 100_000_000;
                let current_ratio = if collateral_value > 0 {
                    (cdp.minted_amount * 10_000) / collateral_value
                } else {
                    0
                };
                current_ratio >= config.liquidation_threshold as u64
            })
            .map(|(_, cdp)| cdp)
            .collect()
    }

    /// Get system statistics
    pub fn get_system_stats(
        &self,
    ) -> SystemStats {
        let total_cdps = self.get_total_cdps();
        let active_cdps = self.cdps
            .iter()
            .filter(|(_, cdp)| !cdp.is_liquidated)
            .count() as u64;
        
        let total_collateral = self.cdps
            .iter()
            .filter(|(_, cdp)| !cdp.is_liquidated)
            .map(|(_, cdp)| cdp.collateral_amount)
            .sum();

        let total_minted = self.cdps
            .iter()
            .filter(|(_, cdp)| !cdp.is_liquidated)
            .map(|(_, cdp)| cdp.minted_amount)
            .sum();

        SystemStats {
            total_cdps,
            active_cdps,
            total_collateral,
            total_minted,
        }
    }
}

/// System statistics
#[derive(Debug, Clone, CandidType, Serialize, SerdeDeserialize)]
pub struct SystemStats {
    pub total_cdps: u64,
    pub active_cdps: u64,
    pub total_collateral: u64,
    pub total_minted: u64,
}

/// Memory manager for stable storage
pub struct StorageManager {
    memory_manager: MemoryManager,
    cdp_storage: CdpStorage,
}

impl StorageManager {
    pub fn new() -> Self {
        let memory_manager = MemoryManager::init(DefaultMemoryImpl::default());
        let cdp_storage = CdpStorage::new(&memory_manager);
        
        Self {
            memory_manager,
            cdp_storage,
        }
    }

    pub fn get_cdp_storage(&mut self,
    ) -> &mut CdpStorage {
        &mut self.cdp_storage
    }

    pub fn get_cdp_storage_ref(&self,
    ) -> &CdpStorage {
        &self.cdp_storage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    fn test_principal() -> Principal {
        Principal::anonymous()
    }

    fn test_config() -> SystemConfig {
        SystemConfig {
            max_collateral_ratio: 9000,
            liquidation_threshold: 8500,
            min_collateral_amount: 100_000,
            min_mint_amount: 1_000,
        }
    }

    #[test]
    fn test_cdp_storage_creation() {
        let memory_manager = MemoryManager::init(DefaultMemoryImpl::default());
        let mut storage = CdpStorage::new(&memory_manager);
        
        let owner = test_principal();
        let cdp = CDP {
            id: 0, // Will be set by storage
            owner,
            collateral_amount: 1_000_000,
            minted_amount: 0,
            created_at: 1700000000,
            updated_at: 1700000000,
            is_liquidated: false,
        };

        let cdp_id = storage.create_cdp(owner, cdp).unwrap();
        assert_eq!(cdp_id, 1);

        let retrieved = storage.get_cdp(cdp_id).unwrap();
        assert_eq!(retrieved.collateral_amount, 1_000_000);
    }

    #[test]
    fn test_user_cdp_indexing() {
        let memory_manager = MemoryManager::init(DefaultMemoryImpl::default());
        let mut storage = CdpStorage::new(&memory_manager);
        
        let owner = test_principal();
        let cdp1 = CDP {
            id: 0,
            owner,
            collateral_amount: 1_000_000,
            minted_amount: 0,
            created_at: 1700000000,
            updated_at: 1700000000,
            is_liquidated: false,
        };

        let cdp2 = CDP {
            id: 0,
            owner,
            collateral_amount: 2_000_000,
            minted_amount: 0,
            created_at: 1700000001,
            updated_at: 1700000001,
            is_liquidated: false,
        };

        let id1 = storage.create_cdp(owner, cdp1).unwrap();
        let id2 = storage.create_cdp(owner, cdp2).unwrap();

        let user_cdps = storage.get_user_cdps(owner);
        assert_eq!(user_cdps.len(), 2);
        assert!(user_cdps.iter().any(|cdp| cdp.id == id1));
        assert!(user_cdps.iter().any(|cdp| cdp.id == id2));
    }

    #[test]
    fn test_pagination() {
        let memory_manager = MemoryManager::init(DefaultMemoryImpl::default());
        let mut storage = CdpStorage::new(&memory_manager);
        
        let owner = test_principal();

        // Create 10 CDPs
        for i in 0..10 {
            let cdp = CDP {
                id: 0,
                owner,
                collateral_amount: (i + 1) * 100_000,
                minted_amount: 0,
                created_at: 1700000000 + i,
                updated_at: 1700000000 + i,
                is_liquidated: false,
            };
            storage.create_cdp(owner, cdp).unwrap();
        }

        let page1 = storage.get_cdps_paginated(Some(1), 5);
        assert_eq!(page1.len(), 5);

        let page2 = storage.get_cdps_paginated(Some(6), 5);
        assert_eq!(page2.len(), 5);
    }

    #[test]
    fn test_system_stats() {
        let memory_manager = MemoryManager::init(DefaultMemoryImpl::default());
        let mut storage = CdpStorage::new(&memory_manager);
        
        let owner = test_principal();
        let config = test_config();
        let btc_price = 50_000_000;

        // Create test CDPs
        for i in 0..3 {
            let cdp = CDP {
                id: 0,
                owner,
                collateral_amount: (i + 1) * 1_000_000,
                minted_amount: (i + 1) * 500_000,
                created_at: 1700000000 + i,
                updated_at: 1700000000 + i,
                is_liquidated: i == 2, // Last one liquidated
            };
            storage.create_cdp(owner, cdp).unwrap();
        }

        let stats = storage.get_system_stats();
        assert_eq!(stats.total_cdps, 3);
        assert_eq!(stats.active_cdps, 2);
        assert_eq!(stats.total_collateral, 6_000_000);
        assert_eq!(stats.total_minted, 3_000_000);
    }
}