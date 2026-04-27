use tendermint_abci::Application;
use tendermint_proto::abci::{
    RequestInfo, ResponseInfo, RequestCheckTx, ResponseCheckTx, ResponseCommit, RequestFinalizeBlock, ResponseFinalizeBlock
};
use std::sync::Arc;
use lmdb::{Environment, Database};

#[derive(Clone, Debug)]
pub struct PnyxApp {
    env: Arc<Environment>,
    data_db: Database,
    scopes_db: Database,
}

impl PnyxApp {
    pub fn new(env: Environment, data_db: Database, scopes_db: Database) -> Self {
        Self {
            env: Arc::new(env),
            data_db,
            scopes_db,
        }
    }
}

impl Application for PnyxApp {
    fn info(&self, _request: RequestInfo) -> ResponseInfo {
        ResponseInfo {
            data: "pnyx-rs".to_string(),
            version: "0.1.0".to_string(),
            app_version: 1,
            last_block_height: 0,
            last_block_app_hash: vec![].into(),
        }
    }

    fn check_tx(&self, _request: RequestCheckTx) -> ResponseCheckTx {
        ResponseCheckTx {
            code: 0,
            ..Default::default()
        }
    }

    fn finalize_block(&self, _request: RequestFinalizeBlock) -> ResponseFinalizeBlock {
        ResponseFinalizeBlock {
            events: vec![],
            tx_results: vec![],
            validator_updates: vec![],
            consensus_param_updates: None,
            app_hash: vec![0u8; 32].into(),
        }
    }

    fn commit(&self) -> ResponseCommit {
        ResponseCommit {
            retain_height: 0,
            ..Default::default()
        }
    }
}
