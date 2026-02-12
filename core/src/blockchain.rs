use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub payload: String,
    pub signature: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub timestamp: i64,
    pub prev_hash: String,
    pub hash: String,
    pub nonce: u64,
    pub transactions: Vec<Transaction>,
    pub merkle_root: String,
}

impl Block {
    pub fn new(index: u64, transactions: Vec<Transaction>, prev_hash: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;

        let mut block = Block {
            index,
            timestamp,
            prev_hash,
            hash: String::new(),
            nonce: 0,
            transactions,
            merkle_root: String::new(),
        };
        block.merkle_root = block.calculate_merkle_root();
        block
    }

    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();
        let data = format!(
            "{}{}{}{}{}",
            self.index,
            self.timestamp,
            self.prev_hash,
            self.nonce,
            self.merkle_root
        );
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    pub fn calculate_merkle_root(&self) -> String {
        if self.transactions.is_empty() {
            return String::new();
        }

        let mut hashes: Vec<String> = self.transactions.iter().map(|tx| {
            let mut hasher = Sha256::new();
            // Use transaction ID and payload for the hash
            let data = format!("{}{}{}{}", tx.id, tx.payload, tx.signature, tx.timestamp);
            hasher.update(data);
            hex::encode(hasher.finalize())
        }).collect();

        while hashes.len() > 1 {
            let mut new_hashes = Vec::new();
            for chunk in hashes.chunks(2) {
                let mut hasher = Sha256::new();
                hasher.update(&chunk[0]);
                if chunk.len() > 1 {
                    hasher.update(&chunk[1]);
                } else {
                    hasher.update(&chunk[0]); // Duplicate last if odd
                }
                new_hashes.push(hex::encode(hasher.finalize()));
            }
            hashes = new_hashes;
        }
        hashes[0].clone()
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
}

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: usize,
}

impl Blockchain {
    pub fn new(difficulty: usize) -> Self {
        let mut chain = Blockchain {
            chain: Vec::new(),
            difficulty,
        };
        chain.create_genesis_block();
        chain
    }

    fn create_genesis_block(&mut self) {
        let mut genesis = Block::new(0, vec![], String::from("0"));
        // Genesis block mining
        genesis.mine_block(self.difficulty);
        self.chain.push(genesis);
    }

    pub fn add_block(&mut self, txs: Vec<Transaction>) {
        let prev_block = self.chain.last().expect("Chain should at least have genesis block");
        let mut new_block = Block::new(
            prev_block.index + 1,
            txs,
            prev_block.hash.clone()
        );
        new_block.mine_block(self.difficulty);
        self.chain.push(new_block);
    }

    pub fn is_valid(&self) -> bool {
        for (i, block) in self.chain.iter().enumerate() {
            if i == 0 {
                continue; // Skip genesis check for simplicity, or verify hardcoded
            }
            let prev_block = &self.chain[i - 1];

            if block.prev_hash != prev_block.hash {
                return false;
            }

            if block.hash != block.calculate_hash() {
                return false;
            }

            // Recalculate merkle root to ensure transactions weren't tampered with
            if block.merkle_root != block.calculate_merkle_root() {
                return false;
            }

            // Check difficulty
            if !block.hash.starts_with(&"0".repeat(self.difficulty)) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blockchain_creation() {
        let blockchain = Blockchain::new(2);
        assert_eq!(blockchain.chain.len(), 1);
        assert!(blockchain.is_valid());
    }

    #[test]
    fn test_add_block() {
        let mut blockchain = Blockchain::new(2);
        let tx = Transaction {
            id: "tx1".to_string(),
            payload: "payload".to_string(),
            signature: "sig".to_string(),
            timestamp: 1234567890,
        };
        blockchain.add_block(vec![tx]);

        assert_eq!(blockchain.chain.len(), 2);
        assert!(blockchain.is_valid());
        assert_eq!(blockchain.chain[1].transactions.len(), 1);
        assert_eq!(blockchain.chain[1].transactions[0].id, "tx1");
    }

    #[test]
    fn test_is_valid_tampered() {
        let mut blockchain = Blockchain::new(2);
        let tx = Transaction {
            id: "tx1".to_string(),
            payload: "payload".to_string(),
            signature: "sig".to_string(),
            timestamp: 1234567890,
        };
        blockchain.add_block(vec![tx]);

        // Tamper with the transaction
        blockchain.chain[1].transactions[0].payload = "tampered".to_string();

        // This should fail because merkle root won't match recalculated one
        assert!(!blockchain.is_valid());

        // Fix merkle root manually? No, is_valid recalculates it.
        // Even if we fix merkle root, hash will change.
        blockchain.chain[1].merkle_root = blockchain.chain[1].calculate_merkle_root();

        // Now hash is invalid
        assert!(!blockchain.is_valid());

        // If we re-mine, it would be valid but prev_hash of next block (if any) would mismatch.
        // Since it's the last block, re-mining would make it valid for *this* block,
        // but that requires nonce change.
        blockchain.chain[1].mine_block(2);
        assert!(blockchain.is_valid());
    }
}
