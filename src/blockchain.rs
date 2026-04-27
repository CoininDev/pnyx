use primitive_types::H256;
use serde::{Deserialize, Serialize};
use smx::value::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub transactions: Vec<Transaction>,
    pub prev_hash: H256,
    pub scope: String,
    pub mpt_root_hash: H256,
    pub node_id: u8,
    pub maintainer: H256,
    pub sign: Vec<u8>,
    pub timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub contract: String,
    pub scope: String,
    pub param: Value,
    pub author: H256,
    pub sign: Vec<u8>,
}
