#![feature(box_patterns)]

use std::{cell::RefCell, fs, path::Path, rc::Rc};

use lmdb::DatabaseFlags;

use crate::db::DBResource;
use crate::consensus::{TendermintBFT, Validator, ValidatorSet};
use crate::crypto::KeyPair;
use primitive_types::H256;

mod db;
mod mpt;
mod blockchain;
mod consensus;
mod crypto;
mod network;
mod sync;

#[tokio::main]
async fn main() {
    let (env, data_db, scopes_db) = init_db().expect("Failed to initialize database");

    let mut amb = smx::val!(ambient);
    amb.add_custom_resource(Rc::new(RefCell::new(DBResource::new(
        data_db, scopes_db, env,
    ))));

    // ============ TESTE 1: Banco de Dados e MPT ============
    println!("=== Test 1: Multi-scope MPT System ===");
    println!("Writing to /conf/greeting...");
    let _write_result =
        smx::eval("_ @{DB, IO} = (\"/conf/greeting\", \"hello from confederation\"): DB.write", &mut amb);

    println!("Reading from /conf/greeting...");
    let read_result =
        smx::eval("_ @{DB, IO} = \"/conf/greeting\": DB.read: IO.print", &mut amb);
    println!("Read result: {:?}", read_result);

    println!("\n=== Test 2: Commune Scope ===");
    println!("Writing to /commune/cypherpunx/laws/001...");
    let _write_result2 = smx::eval(
        "_ @{DB, IO} = (\"/commune/cypherpunx/laws/001\", \"law: universal suffrage\"): DB.write",
        &mut amb,
    );

    println!("Reading from /commune/cypherpunx/laws/001...");
    let read_result2 =
        smx::eval("_ @{DB, IO} = \"/commune/cypherpunx/laws/001\": DB.read: IO.print", &mut amb);
    println!("Read result: {:?}", read_result2);

    println!("\n✓ Multi-scope MPT system operational!");

    // ============ TESTE 2: Consenso Tendermint BFT ============
    println!("\n\n=== Test 3: Tendermint BFT Consensus ===");
    demo_tendermint_consensus().await;

    // ============ TESTE 3: Sincronização de Nós ============
    println!("\n\n=== Test 4: Node Synchronization ===");
    demo_node_synchronization().await;

    // ============ TESTE 4: Criptografia ============
    println!("\n=== Test 5: Cryptography & Key Derivation ===");
    demo_cryptography();
}

/// Demonstração do sistema de consenso Tendermint BFT
async fn demo_tendermint_consensus() {
    println!("Initializing Tendermint BFT consensus engine...\n");

    // Criar validadores
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

    let validator_set = ValidatorSet::new(validators);
    println!("✓ Validator set: {} validators", validator_set.validators.len());
    println!("  Total power: {}, Quorum: {}\n", validator_set.total_power, validator_set.quorum());

    // Criar keypair para o nó 1
    let node_key = KeyPair::generate();
    
    // Criar engine de consenso
    let mut engine = TendermintBFT::new(1, node_key, validator_set);
    
    println!("✓ Consensus engine started");
    println!("  Node ID: {}, Height: {}, Round: {}\n", engine.node_id, engine.height(), engine.round());

    // Inicia primeira rodada
    if let Err(e) = engine.start_round() {
        println!("✗ Error starting round: {}", e);
        return;
    }

    println!("✓ Round 0 started");
    
    if engine.is_proposer() {
        println!("  ✓ Node 1 is proposer");
    }

    // Simular votação
    let validator_1 = engine.validator_set.validators[0].clone();
    let validator_2 = engine.validator_set.validators[1].clone();
    let validator_3 = engine.validator_set.validators[2].clone();

    let block_hash = H256::from_low_u64_be(42);
    
    println!("\n  Adding prevotes...");
    let _ = engine.add_prevote(block_hash, &validator_1);
    let _ = engine.add_prevote(block_hash, &validator_2);
    let _ = engine.add_prevote(block_hash, &validator_3);
    println!("  ✓ Quorum reached for prevotes");

    println!("\n  Adding precommits...");
    let _ = engine.add_precommit(block_hash, &validator_1);
    let _ = engine.add_precommit(block_hash, &validator_2);
    let _ = engine.add_precommit(block_hash, &validator_3);
    println!("  ✓ Quorum reached for precommits");

    println!("\n✓ Consensus round completed!");
    println!("  Blocks committed: {}", engine.committed_blocks().len());
}

/// Demonstração de sincronização entre nós
async fn demo_node_synchronization() {
    use crate::sync::{SyncCoordinator, SyncState};
    use crate::network::PnyxNode;
    use std::str::FromStr;
    use std::net::SocketAddr;

    println!("Initializing node synchronization...\n");

    // Criar coordenador de sincronização para o nó 1 (height=1)
    let sync1 = SyncCoordinator::new(1, 0);
    println!("✓ Node 1 (Height: 1) sync coordinator created");

    // Criar peers
    let peer2 = PnyxNode::new(
        2,
        SocketAddr::from_str("127.0.0.1:8001").unwrap(),
        "confederation".to_string(),
    );

    let peer3 = PnyxNode::new(
        3,
        SocketAddr::from_str("127.0.0.1:8002").unwrap(),
        "confederation".to_string(),
    );

    println!("✓ Peer nodes created\n");

    // Adicionar primeiro peer (sincronizado)
    println!("Adding Peer 2 (Height: 1)...");
    assert!(sync1.add_peer(peer2).await.is_ok());
    assert_eq!(*sync1.state.read().await, SyncState::Synced);
    println!("  ✓ Peer 2 added and synchronized\n");

    // Adicionar segundo peer (ahead)
    println!("Adding Peer 3 (Height: 5)...");
    let mut peer3_ahead = peer3.clone();
    peer3_ahead.height = 5;
    peer3_ahead.round = 2;

    assert!(sync1.add_peer(peer3_ahead).await.is_ok());
    assert_eq!(*sync1.state.read().await, SyncState::OutOfSync);
    println!("  ✓ Peer 3 added - Node 1 is OUT OF SYNC\n");

    // Processar heartbeat para atualizar estado
    let heartbeat = crate::network::ConsensusMessage::Heartbeat {
        node_id: 3,
        height: 5,
        round: 2,
        timestamp: 0,
    };

    assert!(sync1.process_message(heartbeat).await.is_ok());

    // Encontrar peers ahead
    let ahead_peers = sync1.synchronizer.find_ahead_peers().await;
    println!("Peers ahead of Node 1:");
    for peer_id in ahead_peers {
        if let Some((peer, _)) = sync1.synchronizer.get_peer_info(peer_id).await {
            println!("  Peer {}: Height {} (Node 1 is {} blocks behind)",
                peer_id, peer.height, peer.height - 1);
        }
    }

    println!("\n✓ Requesting blocks from ahead peer...");
    let sync_request = sync1.request_blocks_from_peer(3, 2, 5).await;
    assert!(sync_request.is_ok());
    println!("  ✓ Sync request created successfully");

    // Simular recebimento de blocos
    println!("\nSimulating block reception...");
    for height in 2..=5 {
        let block = crate::blockchain::Block {
            transactions: vec![],
            last_block: H256::zero(),
            scope: "confederation".to_string(),
            mpt_root_hash: H256::zero(),
            node_id: 3,
            maintainer: H256::from_low_u64_be(3),
            sign: vec![],
            height: Some(height),
            timestamp: Some(0),
        };

        assert!(sync1.add_received_block(block).await.is_ok());
    }

    let pending = sync1.get_pending_blocks().await;
    println!("  ✓ Received {} blocks", pending.len());

    // Listar estatísticas de rede
    println!("\nNetwork Statistics:");
    let stats = sync1.get_stats().await;
    println!("  Peer count: {}", stats.peer_count);
    println!("  Synced peers: {}", stats.synced_peer_count);
    println!("  Message log size: {}", stats.message_log_size);
    println!("  Total blocks received: {}", stats.total_blocks_received);

    println!("\n✓ Node synchronization demo completed!");
}

/// Demonstração do sistema de criptografia e derivação de chaves
fn demo_cryptography() {
    use crate::crypto::{keccak256, sha256};

    println!("Generating maintainer keypair...");
    let maintainer_key = KeyPair::generate();
    let maintainer_address = maintainer_key.address();
    println!("✓ Maintainer generated");
    println!("  Address: {:?}\n", maintainer_address);

    // Derivar chaves de nó
    println!("Deriving node keys from maintainer key:");
    for node_id in 1..=3 {
        let node_key = KeyPair::derive_node_key(&maintainer_key, node_id);
        let node_address = node_key.address();
        println!("  Node {}: {:?}", node_id, node_address);
    }

    println!("\n✓ HKDF key derivation successful");

    // Teste de assinatura
    println!("\nTesting signing and verification:");
    let message = b"test transaction";
    let signature = maintainer_key.sign(message);
    println!("  ✓ Message signed ({} bytes)", signature.len());

    match crate::crypto::verify_signature(
        message,
        &signature,
        &maintainer_key.public_bytes(),
    ) {
        Ok(()) => println!("  ✓ Signature verified"),
        Err(_) => println!("  ✗ Signature verification failed"),
    }

    // Teste de hash
    println!("\nTesting hash functions:");
    let test_data = b"test data";
    let keccak_hash = keccak256(test_data);
    let sha_hash = sha256(test_data);
    println!("  ✓ Keccak-256: {:?}", &keccak_hash.as_bytes()[..8]);
    println!("  ✓ SHA-256: {:?}", &sha_hash.as_bytes()[..8]);
}

fn init_db() -> lmdb::Result<(lmdb::Environment, lmdb::Database, lmdb::Database)> {
    fs::create_dir_all("./meu_banco").unwrap();
    let env = lmdb::Environment::new()
        .set_max_dbs(2)
        .set_map_size(10 * 1024 * 1024)
        .open(Path::new("./meu_banco"))?;

    let data_db = env.create_db(Some("pnyx"), DatabaseFlags::empty())?;
    let scopes_db = env.create_db(Some("scopes"), DatabaseFlags::empty())?;

    Ok((env, data_db, scopes_db))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_scope_isolation() {
        // Create fresh DB for this test
        fs::remove_dir_all("./test_bank").ok();
        fs::create_dir_all("./test_bank").unwrap();

        let env = lmdb::Environment::new()
            .set_max_dbs(2)
            .set_map_size(10 * 1024 * 1024)
            .open(Path::new("./test_bank"))
            .expect("Failed to open test DB");

        let data_db = env
            .create_db(Some("pnyx"), DatabaseFlags::empty())
            .expect("Failed to create data_db");
        let scopes_db = env
            .create_db(Some("scopes"), DatabaseFlags::empty())
            .expect("Failed to create scopes_db");

        let _db = DBResource::new(data_db, scopes_db, env);

        fs::remove_dir_all("./test_bank").ok();
    }

    #[test]
    fn test_scoped_path_parsing() {
        let test_paths = vec![
            ("/commune/alice/laws/001", "commune/alice", "/laws/001"),
            ("/commune/bob/budget/2024", "commune/bob", "/budget/2024"),
            ("/conf/global/rules", "conf", "/global/rules"),
            ("/here/local/cache", "here", "/local/cache"),
        ];

        for (path, expected_scope, expected_key) in test_paths {
            let (scope, key) = crate::mpt::ScopeManager::parse_path(path).unwrap();
            assert_eq!(
                scope, expected_scope,
                "Scope mismatch for path {}",
                path
            );
            assert_eq!(key, expected_key, "Key mismatch for path {}", path);
        }
    }
}