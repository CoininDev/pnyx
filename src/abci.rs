use tendermint_abci::Application;
use tendermint_proto::abci::{
    RequestInfo, ResponseInfo, RequestCheckTx, ResponseCheckTx,
    ResponseCommit, RequestFinalizeBlock, ResponseFinalizeBlock,
    ExecTxResult,
};
use std::sync::{Arc, Mutex};

use crate::{blockchain::Transaction, runtime::SMXRuntime};

#[derive(Clone)]
pub struct PnyxApp {
    runtime: Arc<Mutex<SMXRuntime>>,
}

impl PnyxApp {
    pub fn new(runtime: SMXRuntime) -> Self {
        Self {
            runtime: Arc::new(Mutex::new(runtime)),
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

    fn check_tx(&self, request: RequestCheckTx) -> ResponseCheckTx {
        let tx_bytes = request.tx.as_ref();

        let tx: Transaction = match serde_json::from_slice(tx_bytes) {
            Ok(t)  => t,
            Err(e) => {
                return ResponseCheckTx {
                    code: 1,
                    log: format!("Failed to deserialize transaction: {e}"),
                    ..Default::default()
                };
            }
        };

        let valid = self.runtime.lock().unwrap().validate_tx(&tx);

        if valid {
            ResponseCheckTx { code: 0, ..Default::default() }
        } else {
            ResponseCheckTx {
                code: 1,
                log: format!("Contract execution failed for '{}'", tx.contract),
                ..Default::default()
            }
        }
    }

    fn finalize_block(&self, request: RequestFinalizeBlock) -> ResponseFinalizeBlock {
        let mut tx_results = Vec::new();

        for raw_tx in &request.txs {
            let result = (|| -> Result<(), String> {
                let tx: Transaction = serde_json::from_slice(raw_tx.as_ref())
                    .map_err(|e| format!("Deserialize error: {e}"))?;

                self.runtime.lock().unwrap().apply_tx(&tx)?;
                Ok(())
            })();

            tx_results.push(match result {
                Ok(()) => ExecTxResult { code: 0, ..Default::default() },
                Err(e) => ExecTxResult {
                    code: 1,
                    log: e,
                    ..Default::default()
                },
            });
        }

        ResponseFinalizeBlock {
            events: vec![],
            tx_results,
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
