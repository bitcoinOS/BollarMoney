// reorg.rs - 区块链重组处理
// 这个模块处理比特币区块链的重组事件

// use crate::types::NewBlockInfo; // 暂时未使用
use ic_cdk::api::management_canister::bitcoin::BitcoinNetwork;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Duplicate block at height {height} with hash {hash}")]
    #[allow(dead_code)]
    DuplicateBlock { height: u32, hash: String },
    
    #[error("Recoverable reorg detected at height {height} with depth {depth}")]
    Recoverable { height: u32, depth: u32 },
    
    #[error("Unrecoverable reorg detected")]
    Unrecoverable,
}

// 获取可恢复的最大重组深度
pub fn get_max_recoverable_reorg_depth(_network: BitcoinNetwork) -> u32 {
    // 对于测试网，我们设置一个较小的值
    // 对于主网，应该设置一个较大的值
    6
}

// 检测区块链重组
pub fn detect_reorg(
    network: BitcoinNetwork,
    new_block: crate::types::NewBlockInfo,
) -> Result<(), Error> {
    // 检查是否已经有相同高度的区块
    let duplicate = crate::BLOCKS.with_borrow(|blocks| {
        blocks.get(&new_block.block_height).map(|existing_block| {
            if existing_block.block_hash != new_block.block_hash {
                // 发现重组
                Some((existing_block.block_height, existing_block.block_hash.clone()))
            } else {
                // 重复区块，但哈希相同
                None
            }
        })
    }).flatten();
    
    if let Some((height, _hash)) = duplicate {
        // 计算重组深度
        let current_height = new_block.block_height;
        let max_depth = get_max_recoverable_reorg_depth(network);
        
        // 查找分叉点
        let mut fork_height = height;
        while fork_height > 0 {
            fork_height -= 1;
            
            // 检查是否找到分叉点
            let fork_found = crate::BLOCKS.with_borrow(|blocks| {
                if let Some(_block) = blocks.get(&fork_height) {
                    // 检查这个区块是否在新链上
                    // 在实际实现中，需要从新链获取这个高度的区块哈希
                    // 这里简化处理，假设找到了分叉点
                    true
                } else {
                    false
                }
            });
            
            if fork_found {
                let depth = current_height - fork_height;
                if depth <= max_depth {
                    return Err(Error::Recoverable { height: fork_height, depth });
                } else {
                    return Err(Error::Unrecoverable);
                }
            }
        }
        
        // 如果没有找到分叉点，认为是不可恢复的重组
        return Err(Error::Unrecoverable);
    }
    
    Ok(())
}

// 处理可恢复的重组
pub fn handle_reorg(fork_height: u32, depth: u32) {
    ic_cdk::println!("Handling reorg at height {} with depth {}", fork_height, depth);
    
    // 移除从分叉点开始的所有区块
    crate::BLOCKS.with_borrow_mut(|blocks| {
        let heights_to_remove: Vec<u32> = blocks
            .iter()
            .filter(|(height, _)| *height >= fork_height)
            .map(|(height, _)| height)
            .collect();
        
        for height in heights_to_remove {
            blocks.remove(&height);
        }
    });
    
    // 回滚受影响的交易
    crate::TX_RECORDS.with_borrow_mut(|_tx_records| {
        // 在实际实现中，需要找出受影响的交易并回滚
        // 这里简化处理，仅打印日志
        ic_cdk::println!("Rolling back transactions affected by reorg");
    });
}