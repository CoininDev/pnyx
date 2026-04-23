use std::collections::HashMap;
use primitive_types::H256;
use serde::{Serialize, Deserialize};
use tiny_keccak::Hasher;

/// Represents a single scope's metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeMetadata {
    pub root_hash: H256,
    pub version: u64,
    pub created_at: u64,
}

impl ScopeMetadata {
    pub fn new() -> Self {
        Self {
            root_hash: H256::zero(),
            version: 0,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// Manages scope routing and validation
/// Scopes are: commune/<name>, conf, here
#[derive(Debug)]
pub struct ScopeManager {
    scopes: HashMap<String, ScopeMetadata>,
}

impl ScopeManager {
    pub fn new() -> Self {
        Self {
            scopes: HashMap::new(),
        }
    }

    /// Register a scope with initial metadata
    pub fn register_scope(&mut self, scope_path: &str) -> Result<(), String> {
        if !self.is_valid_scope(scope_path) {
            return Err(format!("Invalid scope: {}", scope_path));
        }

        if !self.scopes.contains_key(scope_path) {
            self.scopes.insert(scope_path.to_string(), ScopeMetadata::new());
        }

        Ok(())
    }

    /// Parse a path like `/commune/cypherpunx/laws/042` into scope and key
    /// Returns (scope, key)
    /// Example: `/commune/cypherpunx/laws/042` -> (`commune/cypherpunx`, `/laws/042`)
    /// Example: `/conf/law/042` -> (`conf`, `/law/042`)
    /// Example: `/here/local/data` -> (`here`, `/local/data`)
    pub fn parse_path(path: &str) -> Result<(String, String), String> {
        let path = path.trim_start_matches('/');

        if path.is_empty() {
            return Err("Empty path".to_string());
        }

        let parts: Vec<&str> = path.split('/').collect();

        if parts.is_empty() {
            return Err("Invalid path format".to_string());
        }

        let root = parts[0];

        match root {
            "commune" => {
                if parts.len() < 2 {
                    return Err("Commune scope requires at least a commune name".to_string());
                }
                let commune_name = parts[1];
                let scope = format!("commune/{}", commune_name);
                let remaining_path = if parts.len() > 2 {
                    format!("/{}", parts[2..].join("/"))
                } else {
                    "/".to_string()
                };
                Ok((scope, remaining_path))
            }
            "conf" => {
                let remaining_path = if parts.len() > 1 {
                    format!("/{}", parts[1..].join("/"))
                } else {
                    "/".to_string()
                };
                Ok(("conf".to_string(), remaining_path))
            }
            "here" => {
                let remaining_path = if parts.len() > 1 {
                    format!("/{}", parts[1..].join("/"))
                } else {
                    "/".to_string()
                };
                Ok(("here".to_string(), remaining_path))
            }
            _ => Err(format!("Unknown scope root: {}", root)),
        }
    }

    /// Check if a scope path is valid
    fn is_valid_scope(&self, scope: &str) -> bool {
        scope == "conf" || 
        scope == "here" ||
        (scope.starts_with("commune/") && scope.len() > "commune/".len())
    }

    /// Get scope metadata
    pub fn get_scope(&self, scope: &str) -> Option<&ScopeMetadata> {
        self.scopes.get(scope)
    }

    /// Update scope root hash
    pub fn update_root_hash(&mut self, scope: &str, new_root: H256) -> Result<(), String> {
        if let Some(meta) = self.scopes.get_mut(scope) {
            meta.root_hash = new_root;
            meta.version += 1;
            Ok(())
        } else {
            Err(format!("Scope not registered: {}", scope))
        }
    }

    /// List all registered scopes
    pub fn list_scopes(&self) -> Vec<String> {
        self.scopes.keys().cloned().collect()
    }
}

/// Wrapper around a Merkle Patricia Trie for a specific scope
#[derive(Debug)]
pub struct ScopedMerkleTree {
    scope: String,
    // In-memory representation; in production would be lazy-loaded from LMDB
    data: HashMap<Vec<u8>, Vec<u8>>,
    root_hash: H256,
}

impl ScopedMerkleTree {
    pub fn new(scope: String) -> Self {
        Self {
            scope,
            data: HashMap::new(),
            root_hash: H256::zero(),
        }
    }

    /// Insert a key-value pair and update root hash
    pub fn insert(&mut self, key: &str, value: &[u8]) -> H256 {
        self.data.insert(key.as_bytes().to_vec(), value.to_vec());
        self.recalculate_root_hash()
    }

    /// Get value by key
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.data.get(key.as_bytes()).cloned()
    }

    /// Recalculate root hash (simplified - in production would use actual trie)
    fn recalculate_root_hash(&mut self) -> H256 {
        // Simple deterministic hash of all data
        // In production, this would use the actual Merkle Patricia Trie
        let mut sorted_items: Vec<_> = self.data.iter().collect();
        sorted_items.sort_by_key(|a| a.0);

        let mut hasher = tiny_keccak::Keccak::v256();
        for (k, v) in sorted_items {
            hasher.update(k);
            hasher.update(v);
        }

        let mut result = [0u8; 32];
        hasher.finalize(&mut result);
        self.root_hash = H256::from_slice(&result);
        self.root_hash
    }

    pub fn root_hash(&self) -> H256 {
        self.root_hash
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_path_commune() {
        let (scope, key) = ScopeManager::parse_path("/commune/cypherpunx/laws/042").unwrap();
        assert_eq!(scope, "commune/cypherpunx");
        assert_eq!(key, "/laws/042");
    }

    #[test]
    fn test_parse_path_conf() {
        let (scope, key) = ScopeManager::parse_path("/conf/law/042").unwrap();
        assert_eq!(scope, "conf");
        assert_eq!(key, "/law/042");
    }

    #[test]
    fn test_parse_path_here() {
        let (scope, key) = ScopeManager::parse_path("/here/local_data/x").unwrap();
        assert_eq!(scope, "here");
        assert_eq!(key, "/local_data/x");
    }

    #[test]
    fn test_parse_path_single_key() {
        let (scope, key) = ScopeManager::parse_path("/conf").unwrap();
        assert_eq!(scope, "conf");
        assert_eq!(key, "/");
    }

    #[test]
    fn test_parse_path_invalid_root() {
        let result = ScopeManager::parse_path("/invalid/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_scope_manager_register() {
        let mut mgr = ScopeManager::new();
        assert!(mgr.register_scope("commune/cypherpunx").is_ok());
        assert!(mgr.register_scope("conf").is_ok());
        assert!(mgr.register_scope("here").is_ok());

        let scopes = mgr.list_scopes();
        assert_eq!(scopes.len(), 3);
    }

    #[test]
    fn test_scope_manager_invalid_scope() {
        let mut mgr = ScopeManager::new();
        assert!(mgr.register_scope("invalid").is_err());
    }

    #[test]
    fn test_scoped_merkle_tree() {
        let mut tree = ScopedMerkleTree::new("commune/test".to_string());
        let hash1 = tree.insert("key1", b"value1");
        let hash2 = tree.insert("key2", b"value2");

        assert_ne!(hash1, hash2, "Root hash should change after new insert");
        assert_eq!(tree.get("key1"), Some(b"value1".to_vec()));
        assert_eq!(tree.get("key2"), Some(b"value2".to_vec()));
    }
}
