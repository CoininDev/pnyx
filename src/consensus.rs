use crate::blockchain::Block;
use crate::crypto::KeyPair;
use primitive_types::H256;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Erros do consenso Tendermint BFT
#[derive(Error, Debug)]
pub enum ConsensusError {
    #[error("Validador não encontrado: {0}")]
    ValidatorNotFound(String),

    #[error("Quórum não atingido")]
    QuorumNotReached,

    #[error("Bloco inválido: {0}")]
    InvalidBlock(String),

    #[error("Erro de consenso: {0}")]
    ConsensusFailed(String),

    #[error("Erro de serialização: {0}")]
    SerializationError(String),
}

pub type ConsensusResult<T> = Result<T, ConsensusError>;

/// Validador simples
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Validator {
    pub address: H256,
    pub power: u64,
    pub node_id: u8,
}

/// Conjunto de validadores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSet {
    pub validators: Vec<Validator>,
    pub total_power: u64,
}

impl ValidatorSet {
    pub fn new(validators: Vec<Validator>) -> Self {
        let total_power = validators.iter().map(|v| v.power).sum();
        Self {
            validators,
            total_power,
        }
    }

    /// Quórum = 2/3 + 1 do poder total
    pub fn quorum(&self) -> u64 {
        (self.total_power * 2 / 3) + 1
    }

    /// Proposer por round (round-robin)
    pub fn proposer(&self, round: u64) -> Option<&Validator> {
        if self.validators.is_empty() {
            return None;
        }
        let index = (round as usize) % self.validators.len();
        Some(&self.validators[index])
    }

    /// Validador por endereço
    pub fn get(&self, address: &H256) -> Option<&Validator> {
        self.validators.iter().find(|v| v.address == *address)
    }
}

/// Estado de consenso simplificado
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusState {
    pub height: u64,
    pub round: u64,
    pub block: Option<Block>,
    pub committed_blocks: Vec<Block>,
}

impl ConsensusState {
    pub fn new() -> Self {
        Self {
            height: 1,
            round: 0,
            block: None,
            committed_blocks: Vec::new(),
        }
    }

    /// Propõe um bloco
    pub fn propose(&mut self, block: Block) -> ConsensusResult<()> {
        if self.block.is_some() {
            return Err(ConsensusError::ConsensusFailed(
                "Block already proposed for this round".to_string(),
            ));
        }
        self.block = Some(block);
        Ok(())
    }

    /// Commita um bloco
    pub fn commit(&mut self, block: Block) -> ConsensusResult<()> {
        self.committed_blocks.push(block);
        self.height += 1;
        self.round = 0;
        self.block = None;
        Ok(())
    }

    /// Avança para próximo round
    pub fn next_round(&mut self) {
        self.round += 1;
        self.block = None;
    }

    /// Retorna blocos recentes
    pub fn get_recent_blocks(&self, count: usize) -> Vec<Block> {
        let start = if self.committed_blocks.len() > count {
            self.committed_blocks.len() - count
        } else {
            0
        };
        self.committed_blocks[start..].to_vec()
    }
}

/// Engine de Consenso simplificado
pub struct TendermintBFT {
    pub node_id: u8,
    pub node_key: KeyPair,
    pub validator_set: ValidatorSet,
    pub state: ConsensusState,
    pub prevotes: HashMap<u64, HashMap<H256, u64>>,  // round -> (block_hash -> power)
    pub precommits: HashMap<u64, HashMap<H256, u64>>, // round -> (block_hash -> power)
}

impl TendermintBFT {
    /// Cria nova instância do engine
    pub fn new(node_id: u8, node_key: KeyPair, validator_set: ValidatorSet) -> Self {
        Self {
            node_id,
            node_key,
            validator_set,
            state: ConsensusState::new(),
            prevotes: HashMap::new(),
            precommits: HashMap::new(),
        }
    }

    /// Inicia consenso para o height atual
    pub fn start_round(&mut self) -> ConsensusResult<()> {
        // Limpa votos da rodada anterior
        self.prevotes.clear();
        self.precommits.clear();

        // Se é proposer, propõe bloco
        if self.is_proposer() {
            let block = self.create_block()?;
            self.state.propose(block)?;
        }

        Ok(())
    }

    /// Verifica se é proposer
    pub fn is_proposer(&self) -> bool {
        self.validator_set
            .proposer(self.state.round)
            .map(|v| v.node_id == self.node_id)
            .unwrap_or(false)
    }

    /// Cria um novo bloco
    fn create_block(&self) -> ConsensusResult<Block> {
        let last_block = self
            .state
            .committed_blocks
            .last()
            .map(|b| b.last_block)
            .unwrap_or_else(H256::zero);

        Ok(Block {
            transactions: Vec::new(),
            last_block,
            scope: "confederation".to_string(),
            mpt_root_hash: H256::zero(),
            node_id: self.node_id,
            maintainer: self.node_key.address(),
            sign: Vec::new(),
            height: Some(self.state.height),
            timestamp: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            ),
        })
    }

    /// Processa prevote
    pub fn add_prevote(&mut self, block_hash: H256, validator: &Validator) -> ConsensusResult<()> {
        let round = self.state.round;
        self.prevotes
            .entry(round)
            .or_insert_with(HashMap::new)
            .entry(block_hash)
            .and_modify(|p| *p += validator.power)
            .or_insert(validator.power);

        // Verifica quórum
        let power = self.prevotes[&round][&block_hash];
        if power >= self.validator_set.quorum() && block_hash != H256::zero() {
            // Pode fazer lock do bloco
            if let Some(_) = &self.state.block {
                return Ok(());
            }
        }

        Ok(())
    }

    /// Processa precommit
    pub fn add_precommit(
        &mut self,
        block_hash: H256,
        validator: &Validator,
    ) -> ConsensusResult<()> {
        let round = self.state.round;
        self.precommits
            .entry(round)
            .or_insert_with(HashMap::new)
            .entry(block_hash)
            .and_modify(|p| *p += validator.power)
            .or_insert(validator.power);

        // Verifica quórum para commit
        let power = self.precommits[&round][&block_hash];
        if power >= self.validator_set.quorum() && block_hash != H256::zero() {
            if let Some(block) = self.state.block.clone() {
                self.state.commit(block)?;
            }
        }

        Ok(())
    }

    /// Timeout de round
    pub fn timeout_round(&mut self) {
        self.state.next_round();
    }

    /// Retorna estado atual
    pub fn height(&self) -> u64 {
        self.state.height
    }

    pub fn round(&self) -> u64 {
        self.state.round
    }

    pub fn current_block(&self) -> Option<&Block> {
        self.state.block.as_ref()
    }

    pub fn committed_blocks(&self) -> &[Block] {
        &self.state.committed_blocks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_set() {
        let validators = vec![
            Validator {
                address: H256::from_low_u64_be(1),
                power: 100,
                node_id: 1,
            },
            Validator {
                address: H256::from_low_u64_be(2),
                power: 100,
                node_id: 2,
            },
            Validator {
                address: H256::from_low_u64_be(3),
                power: 100,
                node_id: 3,
            },
        ];

        let val_set = ValidatorSet::new(validators);
        assert_eq!(val_set.total_power, 300);
        assert_eq!(val_set.quorum(), 201);
        assert_eq!(val_set.proposer(0).unwrap().node_id, 1);
        assert_eq!(val_set.proposer(1).unwrap().node_id, 2);
    }

    #[test]
    fn test_consensus_state() {
        let mut state = ConsensusState::new();
        assert_eq!(state.height, 1);
        assert_eq!(state.round, 0);

        state.next_round();
        assert_eq!(state.round, 1);
    }

    #[test]
    fn test_bft_engine() {
        let validators = vec![Validator {
            address: H256::from_low_u64_be(1),
            power: 100,
            node_id: 1,
        }];

        let key = crate::crypto::KeyPair::generate();
        let engine = TendermintBFT::new(1, key, ValidatorSet::new(validators));

        assert_eq!(engine.height(), 1);
        assert_eq!(engine.round(), 0);
    }
}
