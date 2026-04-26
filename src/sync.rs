use crate::blockchain::Block;
use crate::consensus::{ConsensusResult, ConsensusError};
use crate::network::{NodeSynchronizer, PnyxNode, ConsensusMessage};
use primitive_types::H256;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Estado de sincronização
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    /// Aguardando sincronização
    Idle,
    /// Sincronizando com peers
    Syncing,
    /// Sincronizado com rede
    Synced,
    /// Dessincronizado (caiu atrás dos peers)
    OutOfSync,
}

/// Coordenador de sincronização
pub struct SyncCoordinator {
    pub state: Arc<RwLock<SyncState>>,
    pub synchronizer: Arc<NodeSynchronizer>,
    pub pending_blocks: Arc<RwLock<HashMap<u64, Block>>>,
    pub sync_timeout_secs: u64,
}

impl SyncCoordinator {
    /// Cria novo coordenador de sincronização
    pub fn new(local_height: u64, local_round: u64) -> Self {
        Self {
            state: Arc::new(RwLock::new(SyncState::Idle)),
            synchronizer: Arc::new(NodeSynchronizer::new(local_height, local_round)),
            pending_blocks: Arc::new(RwLock::new(HashMap::new())),
            sync_timeout_secs: 30,
        }
    }

    /// Adiciona um peer à rede
    pub async fn add_peer(&self, node: PnyxNode) -> ConsensusResult<()> {
        self.synchronizer.connect_peer(node).await?;
        self.check_sync_status().await?;
        Ok(())
    }

    /// Remove um peer
    pub async fn remove_peer(&self, node_id: u8) -> ConsensusResult<()> {
        self.synchronizer.disconnect_peer(node_id).await?;
        self.check_sync_status().await?;
        Ok(())
    }

    /// Processa proposta de bloco
    pub async fn process_block_proposal(&self, block: Block) -> ConsensusResult<()> {
        if let Some(height) = block.height {
            let mut pending = self.pending_blocks.write().await;
            pending.insert(height, block.clone());
            self.synchronizer.add_synced_block(block).await?;
        }
        Ok(())
    }

    /// Processa mensagem de consenso
    pub async fn process_message(&self, msg: ConsensusMessage) -> ConsensusResult<()> {
        self.synchronizer.log_message(msg.clone()).await?;

        match msg {
            ConsensusMessage::Proposal { block, height, .. } => {
                self.process_block_proposal(block).await?;
            }
            ConsensusMessage::Heartbeat {
                node_id,
                height,
                round,
                ..
            } => {
                self.synchronizer.update_peer_state(node_id, height, round).await?;
                self.check_sync_status().await?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Verifica status de sincronização
    pub async fn check_sync_status(&self) -> ConsensusResult<()> {
        let peers = self.synchronizer.get_peers().await;
        
        if peers.is_empty() {
            *self.state.write().await = SyncState::Idle;
            return Ok(());
        }

        let out_of_sync = self.synchronizer.find_out_of_sync_peers().await;
        let ahead = self.synchronizer.find_ahead_peers().await;

        if !ahead.is_empty() {
            *self.state.write().await = SyncState::OutOfSync;
            return Ok(());
        }

        if out_of_sync.is_empty() {
            *self.state.write().await = SyncState::Synced;
            for peer_id in peers.iter().map(|p| p.node_id) {
                self.synchronizer.mark_synced(peer_id).await?;
            }
        } else {
            *self.state.write().await = SyncState::Syncing;
        }

        Ok(())
    }

    /// Encontra peers para requisitar blocos
    pub async fn find_sync_peers(&self) -> Vec<u8> {
        self.synchronizer.find_out_of_sync_peers().await
    }

    /// Obtém lista de blocos a sincronizar
    pub async fn get_blocks_to_sync(
        &self,
        from_height: u64,
        to_height: u64,
    ) -> ConsensusResult<Vec<Block>> {
        Ok(self.synchronizer.get_synced_blocks(from_height, to_height).await)
    }

    /// Adiciona bloco recebido de peer
    pub async fn add_received_block(&self, block: Block) -> ConsensusResult<()> {
        if let Some(height) = block.height {
            let mut pending = self.pending_blocks.write().await;
            pending.insert(height, block.clone());
        }
        self.synchronizer.add_synced_block(block).await
    }

    /// Obtém blocos pendentes
    pub async fn get_pending_blocks(&self) -> Vec<Block> {
        self.pending_blocks.read().await.values().cloned().collect()
    }

    /// Limpa blocos pendentes
    pub async fn clear_pending_blocks(&self) -> ConsensusResult<()> {
        self.pending_blocks.write().await.clear();
        Ok(())
    }

    /// Retorna estado atual de sincronização
    pub async fn current_state(&self) -> SyncState {
        *self.state.read().await
    }

    /// Retorna estatísticas de rede
    pub async fn get_stats(&self) -> crate::network::NetworkStats {
        self.synchronizer.get_network_stats().await
    }

    /// Tenta requisitar blocos de um peer
    pub async fn request_blocks_from_peer(
        &self,
        node_id: u8,
        from_height: u64,
        to_height: u64,
    ) -> ConsensusResult<ConsensusMessage> {
        if let Some(_) = self.synchronizer.get_peer_info(node_id).await {
            Ok(ConsensusMessage::SyncRequest {
                from_height,
                to_height,
            })
        } else {
            Err(ConsensusError::ConsensusFailed(
                format!("Peer {} not found", node_id),
            ))
        }
    }

    /// Calcula hash de confirmação para bloco
    pub fn block_hash(&self, block: &Block) -> H256 {
        use crate::crypto::sha256;
        let data = serde_json::to_vec(block).unwrap_or_default();
        sha256(&data)
    }

    /// Valida bloco recebido
    pub fn validate_received_block(&self, block: &Block) -> ConsensusResult<()> {
        // Verificação básica
        if block.transactions.len() > 10000 {
            return Err(ConsensusError::InvalidBlock(
                "Block has too many transactions".to_string(),
            ));
        }

        if block.maintainer == H256::zero() {
            return Err(ConsensusError::InvalidBlock(
                "Invalid maintainer address".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_sync_coordinator_creation() {
        let coord = SyncCoordinator::new(1, 0);
        assert_eq!(*coord.state.read().await, SyncState::Idle);
    }

    #[tokio::test]
    async fn test_add_peer_to_sync() {
        let coord = SyncCoordinator::new(1, 0);
        let peer = PnyxNode::new(
            2,
            SocketAddr::from_str("127.0.0.1:8001").unwrap(),
            "commune1".to_string(),
        );

        assert!(coord.add_peer(peer).await.is_ok());
        assert_eq!(*coord.state.read().await, SyncState::Synced);
    }

    #[tokio::test]
    async fn test_check_sync_status_with_ahead_peer() {
        let coord = SyncCoordinator::new(5, 0);
        let mut peer = PnyxNode::new(
            2,
            SocketAddr::from_str("127.0.0.1:8001").unwrap(),
            "commune1".to_string(),
        );
        peer.height = 10;

        coord.synchronizer.connect_peer(peer).await.unwrap();
        coord.check_sync_status().await.unwrap();

        assert_eq!(*coord.state.read().await, SyncState::OutOfSync);
    }

    #[tokio::test]
    async fn test_process_heartbeat() {
        let coord = SyncCoordinator::new(1, 0);
        let peer = PnyxNode::new(
            2,
            SocketAddr::from_str("127.0.0.1:8001").unwrap(),
            "commune1".to_string(),
        );

        coord.add_peer(peer).await.unwrap();

        let msg = ConsensusMessage::Heartbeat {
            node_id: 2,
            height: 5,
            round: 2,
            timestamp: 0,
        };

        assert!(coord.process_message(msg).await.is_ok());

        let peers = coord.synchronizer.get_peers().await;
        assert_eq!(peers[0].height, 5);
    }

    #[tokio::test]
    async fn test_pending_blocks() {
        let coord = SyncCoordinator::new(1, 0);
        let block = Block {
            transactions: vec![],
            last_block: H256::zero(),
            scope: "test".to_string(),
            mpt_root_hash: H256::zero(),
            node_id: 1,
            maintainer: H256::from_low_u64_be(1),
            sign: vec![],
            height: Some(2),
            timestamp: Some(0),
        };

        coord.add_received_block(block.clone()).await.unwrap();
        let pending = coord.get_pending_blocks().await;
        assert_eq!(pending.len(), 1);
    }

    #[tokio::test]
    async fn test_validate_block() {
        let coord = SyncCoordinator::new(1, 0);
        
        let valid_block = Block {
            transactions: vec![],
            last_block: H256::zero(),
            scope: "test".to_string(),
            mpt_root_hash: H256::zero(),
            node_id: 1,
            maintainer: H256::from_low_u64_be(1),
            sign: vec![],
            height: Some(1),
            timestamp: Some(0),
        };

        assert!(coord.validate_received_block(&valid_block).is_ok());

        let invalid_block = Block {
            transactions: vec![],
            last_block: H256::zero(),
            scope: "test".to_string(),
            mpt_root_hash: H256::zero(),
            node_id: 1,
            maintainer: H256::zero(),  // Invalid
            sign: vec![],
            height: Some(1),
            timestamp: Some(0),
        };

        assert!(coord.validate_received_block(&invalid_block).is_err());
    }
}
