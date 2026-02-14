// core/src/consensus.rs

use crate::blockchain::Block;

pub trait ConsensusEngine {
    fn verify_block(&self, block: &Block) -> bool;
}

pub struct PoW {
    pub difficulty: usize,
}

impl ConsensusEngine for PoW {
    fn verify_block(&self, block: &Block) -> bool {
        let target = "0".repeat(self.difficulty);
        block.hash == block.calculate_hash() && block.hash.starts_with(&target)
    }
}
