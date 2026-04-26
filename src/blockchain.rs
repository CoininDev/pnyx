use primitive_types::H256;
use smx::value::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub transactions: Vec<Transaction>,
    pub last_block: H256,
    pub scope: String,
    pub mpt_root_hash: H256,
    pub node_id: u8,
    pub maintainer: H256,
    pub sign: Vec<u8>,
    pub height: Option<u64>,
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