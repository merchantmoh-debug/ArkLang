use crate::blockchain::Block;
use sha2::{Digest, Sha256};

pub fn proof_of_work(block: &mut Block, difficulty: u32) {
    let target = "0".repeat(difficulty as usize);
    loop {
        let hash = calculate_hash(block);
        if hash.starts_with(&target) {
            block.hash = hash;
            break;
        }
        block.nonce += 1;
    }
}

pub fn calculate_hash(block: &Block) -> String {
    let data_json = serde_json::to_string(&block.data).expect("operation failed");
    let input = format!(
        "{}{}{}{}{}",
        block.index, block.timestamp, data_json, block.previous_hash, block.nonce
    );
    format!("{:x}", Sha256::digest(input.as_bytes()))
}

pub fn adjust_difficulty(chain: &[Block], current_difficulty: u32) -> u32 {
    if chain.len() < 10 {
        return current_difficulty;
    }

    let latest_block = &chain[chain.len() - 1];
    let prev_block = &chain[chain.len() - 10];
    // Check if timestamps are valid (monotonicity is checked in validation, but here we just need diff)
    // We assume timestamp is u64 or i64. Prompt says u64.
    // If timestamp is u64, subtraction might underflow if not monotonic, but blockchain usually enforces it.
    // We'll use saturating_sub just in case or simple check.

    // Assuming timestamp is u64 as per "TASKS FOR blockchain.rs".
    if latest_block.timestamp < prev_block.timestamp {
        return current_difficulty; // Should not happen in valid chain
    }

    let time_taken = latest_block.timestamp - prev_block.timestamp;

    if time_taken < 100 {
        current_difficulty + 1
    } else if time_taken > 100 {
        if current_difficulty > 1 {
            current_difficulty - 1
        } else {
            1
        }
    } else {
        current_difficulty
    }
}
