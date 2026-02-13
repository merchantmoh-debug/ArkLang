use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::sync::{Mutex, OnceLock};

// --- 1. Basic Structures ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub id: String,
    pub timestamp: i64,
    pub payload: String,   // Deployment Code or Data
    pub signature: String, // Placeholder for future Ed25519
}

impl Transaction {
    pub fn new(payload: String) -> Self {
        let timestamp = Utc::now().timestamp();
        let payload_hash = format!("{:x}", Sha256::digest(payload.as_bytes()));
        let id = format!("{}-{}", timestamp, payload_hash);

        Transaction {
            id,
            timestamp,
            payload,
            signature: "UNSIGNED_DEV".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub index: u64,
    pub timestamp: i64,
    pub prev_hash: String,
    pub merkle_root: String,
    pub hash: String,
    pub nonce: u64,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn new(
        index: u64,
        prev_hash: String,
        transactions: Vec<Transaction>,
        difficulty: usize,
    ) -> Self {
        let timestamp = Utc::now().timestamp();
        let merkle_root = Self::calculate_merkle_root(&transactions);

        let mut block = Block {
            index,
            timestamp,
            prev_hash,
            merkle_root,
            hash: String::new(),
            nonce: 0,
            transactions,
        };

        block.mine_block(difficulty);
        block
    }

    pub fn genesis(difficulty: usize) -> Self {
        let genesis_tx = Transaction::new("GENESIS_BLOCK_PROTOCOL_OMEGA".to_string());
        Self::new(0, "0".to_string(), vec![genesis_tx], difficulty)
    }

    pub fn calculate_hash(&self) -> String {
        let input = format!(
            "{}{}{}{}{}",
            self.index, self.timestamp, self.prev_hash, self.merkle_root, self.nonce
        );
        format!("{:x}", Sha256::digest(input.as_bytes()))
    }

    pub fn mine_block(&mut self, difficulty: usize) {
        let target = "0".repeat(difficulty);
        loop {
            self.hash = self.calculate_hash();
            if self.hash.starts_with(&target) {
                break;
            }
            self.nonce += 1;
        }
    }

    fn calculate_merkle_root(transactions: &[Transaction]) -> String {
        if transactions.is_empty() {
            return String::new();
        }

        let mut hashes: Vec<String> = transactions
            .iter()
            .map(|tx| format!("{:x}", Sha256::digest(tx.id.as_bytes())))
            .collect();

        while hashes.len() > 1 {
            let mut new_hashes = Vec::new();
            for i in (0..hashes.len()).step_by(2) {
                let left = &hashes[i];
                let right = if i + 1 < hashes.len() {
                    &hashes[i + 1]
                } else {
                    left
                };

                let combined = format!("{}{}", left, right);
                new_hashes.push(format!("{:x}", Sha256::digest(combined.as_bytes())));
            }
            hashes = new_hashes;
        }

        hashes[0].clone()
    }
}

// --- 2. The Chain ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: usize,
}

impl Blockchain {
    pub fn new(difficulty: usize) -> Self {
        Blockchain {
            chain: vec![Block::genesis(difficulty)],
            difficulty,
        }
    }

    pub fn add_block(&mut self, transactions: Vec<Transaction>) {
        let prev_block = self.chain.last().unwrap();
        let new_block = Block::new(
            prev_block.index + 1,
            prev_block.hash.clone(),
            transactions,
            self.difficulty,
        );
        self.chain.push(new_block);
    }

    pub fn is_valid(&self) -> bool {
        for i in 1..self.chain.len() {
            let current = &self.chain[i];
            let prev = &self.chain[i - 1];

            if current.hash != current.calculate_hash() {
                return false;
            }

            if current.prev_hash != prev.hash {
                return false;
            }
        }
        true
    }

    pub fn get_contract(&self, hash: &str) -> Option<&Transaction> {
        for block in &self.chain {
            for tx in &block.transactions {
                // Check if payload hash matches
                let payload_hash = format!("{:x}", Sha256::digest(tx.payload.as_bytes()));
                if payload_hash == hash {
                    return Some(tx);
                }
            }
        }
        None
    }
}

static CHAIN_INSTANCE: OnceLock<Mutex<Blockchain>> = OnceLock::new();

pub fn get_chain() -> &'static Mutex<Blockchain> {
    CHAIN_INSTANCE.get_or_init(|| Mutex::new(Blockchain::new(4)))
}

pub fn verify_code_hash(hash: &str) -> bool {
    let chain = get_chain().lock().unwrap();
    chain.get_contract(hash).is_some()
}

pub fn submit_code(code: &str) -> String {
    let tx = Transaction::new(code.to_string());
    let hash = format!("{:x}", Sha256::digest(code.as_bytes()));
    let mut chain = get_chain().lock().unwrap();
    chain.add_block(vec![tx]);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_difficulty() {
        // Test difficulty 1
        let mut chain1 = Blockchain::new(1);
        let genesis1 = &chain1.chain[0];
        // Ensure genesis block meets the difficulty requirement
        assert!(genesis1.hash.starts_with("0"));

        // Add a block with difficulty 1
        let tx = Transaction::new("Test Payload".to_string());
        chain1.add_block(vec![tx]);
        let block1 = &chain1.chain[1];
        assert!(block1.hash.starts_with("0"));

        // Test difficulty 3 (higher difficulty)
        let mut chain2 = Blockchain::new(3);
        let genesis2 = &chain2.chain[0];
        assert!(genesis2.hash.starts_with("000"));

        let tx2 = Transaction::new("Test Payload 2".to_string());
        chain2.add_block(vec![tx2]);
        let block2 = &chain2.chain[1];
        assert!(block2.hash.starts_with("000"));
    }
}
