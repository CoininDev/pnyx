use crate::blockchain::Block;
use crate::consensus::{ConsensusError, ConsensusResult};
use primitive_types::H256;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Mensagens de consenso transmitidas na rede
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusMessage {
    /// Proposta de um novo bloco
    Proposal {
        height: u64,
        round: u64,
        block: Block,
    },
    /// Voto de prevote
    Prevote {
        height: u64,
        round: u64,
        block_hash: H256,
        validator_node_id: u8,
    },
    /// Voto de precommit
    Precommit {
        height: u64,
        round: u64,
        block_hash: H256,
        validator_node_id: u8,
    },
    /// Requisição de sincronização de blocos
    SyncRequest {
        from_height: u64,
        to_height: u64,
    },
    /// Resposta com blocos sincronizados
    SyncResponse {
        blocks: Vec<Block>,
    },
    /// Heartbeat do nó
    Heartbeat {
        node_id: u8,
        height: u64,
        round: u64,
        timestamp: u64,
    },
}

/// Estatísticas de um peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerStats {
    pub node_id: u8,
    pub last_seen: u64,
    pub blocks_received: u64,
    pub blocks_sent: u64,
    pub messages_received: u64,
    pub latency_ms: u64,
    pub is_synced: bool,
}

impl PeerStats {
    pub fn new(node_id: u8) -> Self {
        Self {
            node_id,
            last_seen: current_timestamp(),
            blocks_received: 0,
            blocks_sent: 0,
            messages_received: 0,
            latency_ms: 0,
            is_synced: false,
        }
    }

    pub fn update_heartbeat(&mut self) {
        self.last_seen = current_timestamp();
    }
}

/// Representa um nó peer na rede
#[derive(Debug, Clone)]
pub struct PnyxNode {
    pub node_id: u8,
    pub address: SocketAddr,
    pub commune: String,
    pub height: u64,
    pub round: u64,
}

impl PnyxNode {
    pub fn new(node_id: u8, address: SocketAddr, commune: String) -> Self {
        Self {
            node_id,
            address,
            commune,
            height: 1,
            round: 0,
        }
    }

    pub fn is_ahead(&self, other_height: u64, other_round: u64) -> bool {
        self.height > other_height || (self.height == other_height && self.round > other_round)
    }

    pub fn is_behind(&self, other_height: u64, other_round: u64) -> bool {
        self.height < other_height || (self.height == other_height && self.round < other_round)
    }
}

/// Gerenciador de sincronização entre nós
pub struct NodeSynchronizer {
    local_height: u64,
    local_round: u64,
    peers: Arc<RwLock<HashMap<u8, PnyxNode>>>,
    peer_stats: Arc<RwLock<HashMap<u8, PeerStats>>>,
    message_log: Arc<RwLock<Vec<ConsensusMessage>>>,
    synced_blocks: Arc<RwLock<Vec<Block>>>,
}

impl NodeSynchronizer {
    pub fn new(local_height: u64, local_round: u64) -> Self {
        Self {
            local_height,
            local_round,
            peers: Arc::new(RwLock::new(HashMap::new())),
            peer_stats: Arc::new(RwLock::new(HashMap::new())),
            message_log: Arc::new(RwLock::new(Vec::new())),
            synced_blocks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Conecta a um novo peer
    pub async fn connect_peer(&self, node: PnyxNode) -> ConsensusResult<()> {
        let node_id = node.node_id;
        
        let mut peers = self.peers.write().await;
        peers.insert(node_id, node);

        let mut stats = self.peer_stats.write().await;
        stats.insert(node_id, PeerStats::new(node_id));

        Ok(())
    }

    /// Desconecta de um peer
    pub async fn disconnect_peer(&self, node_id: u8) -> ConsensusResult<()> {
        let mut peers = self.peers.write().await;
        peers.remove(&node_id);

        let mut stats = self.peer_stats.write().await;
        stats.remove(&node_id);

        Ok(())
    }

    /// Obtém lista de peers online
    pub async fn get_peers(&self) -> Vec<PnyxNode> {
        self.peers.read().await.values().cloned().collect()
    }

    /// Conta peers sincronizados
    pub async fn synced_peer_count(&self) -> usize {
        self.peer_stats
            .read()
            .await
            .values()
            .filter(|s| s.is_synced)
            .count()
    }

    /// Atualiza estado de um peer
    pub async fn update_peer_state(
        &self,
        node_id: u8,
        height: u64,
        round: u64,
    ) -> ConsensusResult<()> {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(&node_id) {
            peer.height = height;
            peer.round = round;
        }

        if let Some(stats) = self.peer_stats.write().await.get_mut(&node_id) {
            stats.update_heartbeat();
        }

        Ok(())
    }

    /// Registra mensagem recebida
    pub async fn log_message(&self, msg: ConsensusMessage) -> ConsensusResult<()> {
        let mut log = self.message_log.write().await;
        log.push(msg);
        
        // Mantém apenas últimas 1000 mensagens
        if log.len() > 1000 {
            log.remove(0);
        }

        Ok(())
    }

    /// Encontra peers que precisam sincronização
    pub async fn find_out_of_sync_peers(&self) -> Vec<u8> {
        let peers = self.peers.read().await;
        peers
            .values()
            .filter(|p| p.is_behind(self.local_height, self.local_round))
            .map(|p| p.node_id)
            .collect()
    }

    /// Encontra peers ahead
    pub async fn find_ahead_peers(&self) -> Vec<u8> {
        let peers = self.peers.read().await;
        peers
            .values()
            .filter(|p| p.is_ahead(self.local_height, self.local_round))
            .map(|p| p.node_id)
            .collect()
    }

    /// Marca peer como sincronizado
    pub async fn mark_synced(&self, node_id: u8) -> ConsensusResult<()> {
        if let Some(stats) = self.peer_stats.write().await.get_mut(&node_id) {
            stats.is_synced = true;
        }
        Ok(())
    }

    /// Marca peer como dessincronizado
    pub async fn mark_out_of_sync(&self, node_id: u8) -> ConsensusResult<()> {
        if let Some(stats) = self.peer_stats.write().await.get_mut(&node_id) {
            stats.is_synced = false;
        }
        Ok(())
    }

    /// Adiciona bloco sincronizado
    pub async fn add_synced_block(&self, block: Block) -> ConsensusResult<()> {
        let mut blocks = self.synced_blocks.write().await;
        blocks.push(block);
        Ok(())
    }

    /// Obtém blocos sincronizados
    pub async fn get_synced_blocks(&self, from: u64, to: u64) -> Vec<Block> {
        self.synced_blocks
            .read()
            .await
            .iter()
            .filter(|b| {
                if let Some(height) = b.height {
                    height >= from && height <= to
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }

    /// Limpa blocos sincronizados
    pub async fn clear_synced_blocks(&self) -> ConsensusResult<()> {
        self.synced_blocks.write().await.clear();
        Ok(())
    }

    /// Retorna estatísticas de rede
    pub async fn get_network_stats(&self) -> NetworkStats {
        let peers = self.peers.read().await;
        let stats = self.peer_stats.read().await;

        let peer_count = peers.len();
        let synced_count = stats.values().filter(|s| s.is_synced).count();
        let total_blocks_received: u64 = stats.values().map(|s| s.blocks_received).sum();
        let total_blocks_sent: u64 = stats.values().map(|s| s.blocks_sent).sum();

        NetworkStats {
            peer_count,
            synced_peer_count: synced_count,
            total_blocks_received,
            total_blocks_sent,
            message_log_size: self.message_log.read().await.len(),
        }
    }

    /// Retorna informações de um peer específico
    pub async fn get_peer_info(&self, node_id: u8) -> Option<(PnyxNode, PeerStats)> {
        let peers = self.peers.read().await;
        let stats = self.peer_stats.read().await;

        peers.get(&node_id).and_then(|peer| {
            stats.get(&node_id).map(|stat| (peer.clone(), stat.clone()))
        })
    }
}

/// Estatísticas de rede geral
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub peer_count: usize,
    pub synced_peer_count: usize,
    pub total_blocks_received: u64,
    pub total_blocks_sent: u64,
    pub message_log_size: usize,
}

/// Obtém timestamp atual em segundos
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_node_synchronizer_creation() {
        let sync = NodeSynchronizer::new(1, 0);
        assert_eq!(sync.local_height, 1);
        assert_eq!(sync.local_round, 0);
    }

    #[tokio::test]
    async fn test_connect_peer() {
        let sync = NodeSynchronizer::new(1, 0);
        let peer = PnyxNode::new(
            2,
            SocketAddr::from_str("127.0.0.1:8001").unwrap(),
            "commune1".to_string(),
        );

        assert!(sync.connect_peer(peer).await.is_ok());
        assert_eq!(sync.get_peers().await.len(), 1);
    }

    #[tokio::test]
    async fn test_peer_state_update() {
        let sync = NodeSynchronizer::new(1, 0);
        let peer = PnyxNode::new(
            2,
            SocketAddr::from_str("127.0.0.1:8001").unwrap(),
            "commune1".to_string(),
        );

        sync.connect_peer(peer).await.unwrap();
        sync.update_peer_state(2, 5, 2).await.unwrap();

        let peers = sync.get_peers().await;
        assert_eq!(peers[0].height, 5);
        assert_eq!(peers[0].round, 2);
    }

    #[tokio::test]
    async fn test_find_out_of_sync_peers() {
        let sync = NodeSynchronizer::new(10, 0);

        let peer1 = PnyxNode::new(
            1,
            SocketAddr::from_str("127.0.0.1:8001").unwrap(),
            "commune1".to_string(),
        );
        let peer2 = PnyxNode::new(
            2,
            SocketAddr::from_str("127.0.0.1:8002").unwrap(),
            "commune1".to_string(),
        );

        sync.connect_peer(peer1).await.unwrap();
        sync.connect_peer(peer2).await.unwrap();

        sync.update_peer_state(1, 5, 0).await.unwrap();
        sync.update_peer_state(2, 8, 0).await.unwrap();

        let out_of_sync = sync.find_out_of_sync_peers().await;
        assert_eq!(out_of_sync.len(), 2);
    }

    #[tokio::test]
    async fn test_network_stats() {
        let sync = NodeSynchronizer::new(1, 0);
        let peer = PnyxNode::new(
            2,
            SocketAddr::from_str("127.0.0.1:8001").unwrap(),
            "commune1".to_string(),
        );

        sync.connect_peer(peer).await.unwrap();
        let stats = sync.get_network_stats().await;

        assert_eq!(stats.peer_count, 1);
        assert_eq!(stats.synced_peer_count, 0);
    }
}

    /// Processa mensagens do buffer
    pub async fn process_messages(&self) -> Vec<ConsensusMessage> {
        let mut buffer = self.message_buffer.write().await;
        buffer.drain(..).collect()
    }

    /// Broadcast de mensagem para todos os peers
    pub async fn broadcast(&self, msg: ConsensusMessage) -> ConsensusResult<()> {
        // Em uma implementação real, seria enviado via TCP/UDP
        // Por enquanto, apenas enfileira localmente para testes
        self.queue_message(msg).await
    }

    /// Envio unicast para um peer específico
    pub async fn send_to(&self, node_id: u8, msg: ConsensusMessage) -> ConsensusResult<()> {
        let peers = self.peers.read().await;

        if !peers.contains_key(&node_id) {
            return Err(ConsensusError::ValidatorNotFound(format!(
                "Node {} not found",
                node_id
            )));
        }

        // Em implementação real, seria enviado direto ao nó
        self.queue_message(msg).await
    }

    /// Retorna informações do nó local
    pub fn local_node(&self) -> &PnyxNode {
        &self.local_node
    }

    /// Retorna total de peers
    pub async fn peer_count(&self) -> usize {
        self.peers.read().await.len()
    }
}

/// Builder para facilitar construção de rede
pub struct NetworkBuilder {
    node_id: u8,
    address: SocketAddr,
    commune: String,
}

impl NetworkBuilder {
    pub fn new(node_id: u8, address: SocketAddr, commune: String) -> Self {
        Self {
            node_id,
            address,
            commune,
        }
    }

    pub fn build(self) -> P2PNetwork {
        P2PNetwork::new(self.node_id, self.address, self.commune)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_network_creation() {
        let addr = SocketAddr::from_str("127.0.0.1:8000").unwrap();
        let network = P2PNetwork::new(1, addr, "test_commune".to_string());

        assert_eq!(network.local_node().node_id, 1);
        assert_eq!(network.peer_count().await, 0);
    }

    #[tokio::test]
    async fn test_add_peer() {
        let addr = SocketAddr::from_str("127.0.0.1:8000").unwrap();
        let network = P2PNetwork::new(1, addr, "test_commune".to_string());

        let peer_addr = SocketAddr::from_str("127.0.0.1:8001").unwrap();
        let peer = PnyxNode {
            node_id: 2,
            address: peer_addr,
            commune: "test_commune".to_string(),
        };

        assert!(network.add_peer(peer).await.is_ok());
        assert_eq!(network.peer_count().await, 1);
    }

    #[tokio::test]
    async fn test_message_queueing() {
        let addr = SocketAddr::from_str("127.0.0.1:8000").unwrap();
        let network = P2PNetwork::new(1, addr, "test_commune".to_string());

        let msg = ConsensusMessage::StatusRequest {
            height: 1,
            round: 0,
        };

        assert!(network.queue_message(msg).await.is_ok());

        let messages = network.process_messages().await;
        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn test_broadcast() {
        let addr = SocketAddr::from_str("127.0.0.1:8000").unwrap();
        let network = P2PNetwork::new(1, addr, "test_commune".to_string());

        let msg = ConsensusMessage::StatusRequest {
            height: 1,
            round: 0,
        };

        assert!(network.broadcast(msg).await.is_ok());
    }
}
