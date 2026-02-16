use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Mutex, OnceLock};
use crate::consensus::{proof_of_work, adjust_difficulty, calculate_hash};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub sender: String,
    pub receiver: String,
    pub amount: i64,
    pub signature: String, // hex string
    // Added field to support VM contract storage requirements
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub data: Vec<Transaction>,
    pub previous_hash: String,
    pub hash: String,
    pub nonce: u64,
}

impl Block {
    pub fn new(index: u64, data: Vec<Transaction>, previous_hash: String) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Block {
            index,
            timestamp,
            data,
            previous_hash,
            hash: String::new(),
            nonce: 0,
        }
    }
}

pub fn merkle_root(transactions: &[Transaction]) -> String {
    if transactions.is_empty() {
        return format!("{:x}", Sha256::digest(""));
    }

    let mut hashes: Vec<String> = transactions
        .iter()
        .map(|tx| {
            let json = serde_json::to_string(tx).unwrap();
            format!("{:x}", Sha256::digest(json.as_bytes()))
        })
        .collect();

    while hashes.len() > 1 {
        let mut new_hashes = Vec::new();
        let len = hashes.len();
        for i in (0..len).step_by(2) {
            let left = &hashes[i];
            let right = if i + 1 < len {
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

pub fn validate_block(block: &Block, previous_block: &Block, difficulty: u32) -> bool {
    if block.previous_hash != previous_block.hash {
        return false;
    }
    if block.index != previous_block.index + 1 {
        return false;
    }

    let target = "0".repeat(difficulty as usize);
    if !block.hash.starts_with(&target) {
        return false;
    }

    let recomputed = calculate_hash(block);
    if block.hash != recomputed {
        return false;
    }

    true
}

#[derive(Debug, Clone)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub pending_tx: Vec<Transaction>,
    pub difficulty: u32,
}

impl Blockchain {
    pub fn new() -> Self {
        let genesis = Block {
            index: 0,
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            data: vec![],
            previous_hash: "0".to_string(),
            hash: String::new(),
            nonce: 0,
        };
        let mut genesis = genesis;
        genesis.hash = calculate_hash(&genesis);

        Blockchain {
            chain: vec![genesis],
            pending_tx: Vec::new(),
            difficulty: 1,
        }
    }

    pub fn add_transaction(&mut self, tx: Transaction) -> bool {
        if tx.sender.is_empty() || tx.receiver.is_empty() {
             return false;
        }
        self.pending_tx.push(tx);
        true
    }

    pub fn mine_block(&mut self) -> Block {
        let previous_block = self.chain.last().unwrap();
        let index = previous_block.index + 1;
        let previous_hash = previous_block.hash.clone();
        let data = self.pending_tx.clone();

        let mut block = Block::new(index, data, previous_hash);

        self.difficulty = adjust_difficulty(&self.chain, self.difficulty);

        proof_of_work(&mut block, self.difficulty);

        self.chain.push(block.clone());
        self.pending_tx.clear();

        block
    }

    pub fn get_balance(&self, address: &str) -> i64 {
        let mut balance = 0;
        for block in &self.chain {
            for tx in &block.data {
                if tx.receiver == address {
                    balance += tx.amount;
                }
                if tx.sender == address {
                    balance -= tx.amount;
                }
            }
        }
        balance
    }

    pub fn is_chain_valid(&self) -> bool {
        let mut current_difficulty = 1;

        for i in 1..self.chain.len() {
            let current = &self.chain[i];
            let previous = &self.chain[i-1];

            current_difficulty = adjust_difficulty(&self.chain[0..i], current_difficulty);

            if !validate_block(current, previous, current_difficulty) {
                return false;
            }
        }
        true
    }
}

// Global Blockchain Instance for VM interaction
static CHAIN_INSTANCE: OnceLock<Mutex<Blockchain>> = OnceLock::new();

pub fn get_chain() -> &'static Mutex<Blockchain> {
    CHAIN_INSTANCE.get_or_init(|| Mutex::new(Blockchain::new()))
}

// Legacy/VM Support Functions

pub fn verify_code_hash(hash: &str) -> bool {
    let chain = get_chain().lock().unwrap();
    for block in &chain.chain {
        for tx in &block.data {
            let code_hash = format!("{:x}", Sha256::digest(tx.data.as_bytes()));
            if code_hash == hash {
                return true;
            }
        }
    }
    false
}

pub fn submit_code(code: &str) -> String {
    let tx = Transaction {
        sender: "SYSTEM".to_string(),
        receiver: "CONTRACT".to_string(),
        amount: 0,
        signature: "SIGNED_BY_SYSTEM".to_string(),
        data: code.to_string(),
    };

    let hash = format!("{:x}", Sha256::digest(code.as_bytes()));

    let mut chain = get_chain().lock().unwrap();
    chain.add_transaction(tx);
    chain.mine_block();

    hash
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_block_creation() {
        let bc = Blockchain::new();
        assert_eq!(bc.chain.len(), 1);
        let genesis = &bc.chain[0];
        assert_eq!(genesis.index, 0);
        assert_eq!(genesis.previous_hash, "0");
        assert!(genesis.data.is_empty());
    }

    #[test]
    fn test_add_transaction() {
        let mut bc = Blockchain::new();
        let tx = Transaction {
            sender: "Alice".to_string(),
            receiver: "Bob".to_string(),
            amount: 50,
            signature: "sig".to_string(),
            data: "".to_string(),
        };
        assert!(bc.add_transaction(tx.clone()));
        assert_eq!(bc.pending_tx.len(), 1);
    }

    #[test]
    fn test_mine_block() {
        let mut bc = Blockchain::new();
        let tx = Transaction {
            sender: "Alice".to_string(),
            receiver: "Bob".to_string(),
            amount: 50,
            signature: "sig".to_string(),
            data: "".to_string(),
        };
        bc.add_transaction(tx);
        let block = bc.mine_block();

        assert_eq!(block.index, 1);
        assert_eq!(bc.chain.len(), 2);
        assert_eq!(bc.pending_tx.len(), 0);
        assert_eq!(block.data.len(), 1);
        assert!(block.hash.starts_with("0")); // Difficulty 1
    }

    #[test]
    fn test_chain_validation_valid() {
        let mut bc = Blockchain::new();
        let tx = Transaction {
            sender: "Alice".to_string(),
            receiver: "Bob".to_string(),
            amount: 50,
            signature: "sig".to_string(),
            data: "".to_string(),
        };
        bc.add_transaction(tx);
        bc.mine_block();

        assert!(bc.is_chain_valid());
    }

    #[test]
    fn test_chain_validation_tampered() {
        let mut bc = Blockchain::new();
        let tx = Transaction {
            sender: "Alice".to_string(),
            receiver: "Bob".to_string(),
            amount: 50,
            signature: "sig".to_string(),
            data: "".to_string(),
        };
        bc.add_transaction(tx);
        bc.mine_block();

        // Tamper
        // We need to modify data.
        bc.chain[1].data[0].amount = 1000;

        // The hash should now be invalid because we changed data but didn't remine.
        assert!(!bc.is_chain_valid());
    }

    #[test]
    fn test_merkle_root_single_tx() {
        let tx = Transaction {
            sender: "A".to_string(),
            receiver: "B".to_string(),
            amount: 10,
            signature: "s".to_string(),
            data: "".to_string(),
        };
        let root = merkle_root(&[tx.clone()]);

        let json = serde_json::to_string(&tx).unwrap();
        let expected = format!("{:x}", Sha256::digest(json.as_bytes()));
        assert_eq!(root, expected);
    }

    #[test]
    fn test_merkle_root_multiple_tx() {
         let tx1 = Transaction {
            sender: "A".to_string(),
            receiver: "B".to_string(),
            amount: 10,
            signature: "s".to_string(),
            data: "".to_string(),
        };
        let tx2 = Transaction {
            sender: "B".to_string(),
            receiver: "C".to_string(),
            amount: 20,
            signature: "s".to_string(),
            data: "".to_string(),
        };

        let root = merkle_root(&[tx1.clone(), tx2.clone()]);

        let h1 = format!("{:x}", Sha256::digest(serde_json::to_string(&tx1).unwrap().as_bytes()));
        let h2 = format!("{:x}", Sha256::digest(serde_json::to_string(&tx2).unwrap().as_bytes()));
        let expected = format!("{:x}", Sha256::digest(format!("{}{}", h1, h2).as_bytes()));

        assert_eq!(root, expected);
    }

    #[test]
    fn test_balance_tracking() {
        let mut bc = Blockchain::new();
        // A sends 10 to B
        bc.add_transaction(Transaction {
            sender: "A".to_string(),
            receiver: "B".to_string(),
            amount: 10,
            signature: "s".to_string(),
            data: "".to_string(),
        });
        bc.mine_block();

        // B sends 5 to C
        bc.add_transaction(Transaction {
            sender: "B".to_string(),
            receiver: "C".to_string(),
            amount: 5,
            signature: "s".to_string(),
            data: "".to_string(),
        });
        bc.mine_block();

        assert_eq!(bc.get_balance("A"), -10);
        assert_eq!(bc.get_balance("B"), 5); // 10 - 5
        assert_eq!(bc.get_balance("C"), 5);
    }

    #[test]
    fn test_proof_of_work_difficulty_1() {
        let mut block = Block::new(1, vec![], "0".to_string());
        proof_of_work(&mut block, 1);
        assert!(block.hash.starts_with("0"));
    }
}
