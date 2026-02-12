use crate::blockchain::Block;
use std::fmt::Debug;

pub trait ConsensusEngine: Debug {
    fn verify_block(&self, block: &Block) -> bool;
    fn mine(&self, block: &mut Block);
}

#[derive(Debug)]
pub struct PoW {
    pub difficulty: usize,
}

impl PoW {
    pub fn new(difficulty: usize) -> Self {
        PoW { difficulty }
    }
}

impl ConsensusEngine for PoW {
    fn verify_block(&self, block: &Block) -> bool {
        let hash = block.calculate_hash();
        if hash != block.hash {
            return false;
        }
        hash.starts_with(&"0".repeat(self.difficulty))
    }

    fn mine(&self, block: &mut Block) {
        let prefix = "0".repeat(self.difficulty);
        loop {
            let hash = block.calculate_hash();
            if hash.starts_with(&prefix) {
                block.hash = hash;
                break;
            }
            block.nonce += 1;
        }
    }
}

#[derive(Debug)]
pub struct PoS {}

impl ConsensusEngine for PoS {
    fn verify_block(&self, _block: &Block) -> bool {
        true
    }
    fn mine(&self, _block: &mut Block) {
        // PoS mining logic stub
    }
}
