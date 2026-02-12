use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::consensus::ConsensusEngine;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub data: String,
    pub prev_hash: String,
    pub hash: String,
    pub nonce: u64,
}

impl Block {
    pub fn new(index: u64, timestamp: u64, data: String, prev_hash: String) -> Self {
        let mut block = Block {
            index,
            timestamp,
            data,
            prev_hash,
            hash: String::new(),
            nonce: 0,
        };
        block.hash = block.calculate_hash();
        block
    }

    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.index.to_be_bytes());
        hasher.update(self.timestamp.to_be_bytes());
        hasher.update(self.data.as_bytes());
        hasher.update(self.prev_hash.as_bytes());
        hasher.update(self.nonce.to_be_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }
}

#[derive(Debug)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub consensus: Box<dyn ConsensusEngine>,
}

impl Blockchain {
    pub fn new(consensus: Box<dyn ConsensusEngine>) -> Self {
        let mut genesis_block = Block::new(0, 0, "Genesis".to_string(), "0".repeat(64));
        consensus.mine(&mut genesis_block);

        Blockchain {
            chain: vec![genesis_block],
            consensus,
        }
    }

    pub fn add_block(&mut self, data: String) {
        let prev_block = self.chain.last().unwrap();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut new_block = Block::new(
            prev_block.index + 1,
            timestamp,
            data,
            prev_block.hash.clone(),
        );

        self.consensus.mine(&mut new_block);
        self.chain.push(new_block);
    }

    pub fn is_valid(&self) -> bool {
        for (i, block) in self.chain.iter().enumerate() {
            if i == 0 {
                continue;
            }
            let prev_block = &self.chain[i - 1];
            if block.prev_hash != prev_block.hash {
                return false;
            }
            if !self.consensus.verify_block(block) {
                return false;
            }
        }
        true
    }
}
