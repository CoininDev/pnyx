use lmdb::Transaction;
use lmdb::WriteFlags;
use serde_json;

use smx::eval::EvalResult;
use smx::eval_error;
use smx::value::*;
use smx::error::{EvalError, EvalErrorType::*, MapEvalError};

use crate::mpt::{ScopeManager, ScopeMetadata, ScopedMerkleTree};
use std::collections::HashMap;

pub struct DBResource {
    data_db: lmdb::Database,
    scopes_db: lmdb::Database,
    env: lmdb::Environment,
    scope_manager: ScopeManager,
    merkle_trees: HashMap<String, ScopedMerkleTree>,
    pub testing: bool
}

impl DBResource {
    pub fn new(
        data_db: lmdb::Database,
        scopes_db: lmdb::Database,
        env: lmdb::Environment,
    ) -> Self {
        let mut scope_manager = ScopeManager::new();
        // Register default scopes
        let _ = scope_manager.register_scope("conf");
        let _ = scope_manager.register_scope("here");

        Self {
            data_db,
            scopes_db,
            env,
            scope_manager,
            merkle_trees: HashMap::new(),
            testing: false,
        }
    }

    /// Load or register a scope
    fn ensure_scope(&mut self, scope: &str) -> Result<(), String> {
        if self.scope_manager.get_scope(scope).is_none() {
            self.scope_manager.register_scope(scope)?;
            // Initialize scope metadata in database
            self.store_scope_metadata(scope)?;
        }
        Ok(())
    }

    /// Store scope metadata in scopes database
    fn store_scope_metadata(&self, scope: &str) -> Result<(), String> {
        let metadata = self
            .scope_manager
            .get_scope(scope)
            .ok_or("Scope not found")?;

        let json = serde_json::to_string(metadata).map_err(|e| format!("JSON error: {}", e))?;
        let key = format!("scope:{}", scope);

        let mut txn = self
            .env
            .begin_rw_txn()
            .map_err(|e| format!("Transaction error: {}", e))?;
        txn.put(self.scopes_db, &key, &json, WriteFlags::empty())
            .map_err(|e| format!("Put error: {}", e))?;
        txn.commit().map_err(|e| format!("Commit error: {}", e))?;

        Ok(())
    }

    /// Retrieve scope metadata from database
    fn load_scope_metadata(&self, scope: &str) -> Result<Option<ScopeMetadata>, String> {
        let key = format!("scope:{}", scope);
        let txn = self
            .env
            .begin_ro_txn()
            .map_err(|e| format!("Transaction error: {}", e))?;

        match txn.get(self.scopes_db, &key) {
            Ok(val) => {
                let json_str = String::from_utf8_lossy(val);
                let metadata: ScopeMetadata =
                    serde_json::from_str(&json_str).map_err(|e| format!("JSON error: {}", e))?;
                Ok(Some(metadata))
            }
            Err(lmdb::Error::NotFound) => Ok(None),
            Err(e) => Err(format!("Get error: {}", e)),
        }
    }
}


impl IoObject for DBResource {
    fn name(&self) -> &str {
        "DB"
    }

    fn redirect(&mut self, function: Vec<String>, value: Value, _: &mut Ambient) -> EvalResult<smx::value::Value> {
        match function.as_slice() {
            fun if fun == &["read"] => {
                if let smx::value::Value::Str(path) = value {
                    self.read_scoped(&path)
                } else {
                    Err(eval_error!(WrongTypes(function.join("."), smx::value::PatternType::String, value)))
                }
            }
            fun if fun == &["write"] => match value.clone() {
                Value::Pair(box Value::Str(path), box Value::Str(val)) => {
                    self.write_scoped(&path, &val)?;
                    Ok(smx::val!())
                }
                Value::Pair(box Value::Str(path), box val) => {
                    let val = serde_json::to_string(&val).unwrap();
                    self.write_scoped(&path, &val)?;
                    Ok(smx::val!())
                }
                _ => Err(eval_error!(WrongTypes(function.join("."), PatternType::List([PatternType::String, PatternType::Any].into()), value)))
            }
            fun if fun == &["remove"] => {
                if let smx::value::Value::Str(path) = value {
                    self.remove_scoped(&path)?;
                    Ok(smx::val!())
                } else {
                    Err(eval_error!(WrongTypes(function.join("."), smx::value::PatternType::String, value)))
                }
            }
            
            cu => Err(EvalError::new(smx::error::EvalErrorType::VariableDoesNotExists(cu.join(".")))),
        }
    }
}

impl DBResource {
    /// Read from a scoped path like `/commune/cypherpunx/laws/042`
    pub fn read_scoped(&mut self, path: &str) -> EvalResult<Value> {
        let (scope, key) = crate::mpt::ScopeManager::parse_path(path)
            .map_err(|e| eval_error!(VariableDoesNotExists(format!("Invalid path: {}", e))))?;

        self.ensure_scope(&scope)
            .map_err(|e| eval_error!(VariableDoesNotExists(format!("Scope error: {}", e))))?;

        // Construct full LMDB key: scope::key
        let full_key = format!("{}::{}", scope, key);

        let txn = self.env.begin_ro_txn().map_eval_error()?;
        match txn.get(self.data_db, &full_key) {
            Ok(val) => Ok(smx::val!(String::from_utf8_lossy(val).to_string())),
            Err(_) => Ok(smx::val!()),
        }
    }

    /// Write to a scoped path like `/commune/cypherpunx/laws/042`
    pub fn write_scoped(&mut self, path: &str, value: &str) -> EvalResult<()> {
        let (scope, key) = crate::mpt::ScopeManager::parse_path(path).map_eval_error()?;
        self.ensure_scope(&scope).map_eval_error()?;

        // Construct full LMDB key: scope::key
        let full_key = format!("{}::{}", scope, key);
        let val_string = value.to_string(); // Convert &str to String

        // Start isolated transaction for this scope
        let mut txn = self.env.begin_rw_txn().map_eval_error()?;
        txn.put(self.data_db, &full_key, &val_string, WriteFlags::empty()).map_eval_error()?;

        // Update or create scoped Merkle tree
        if !self.merkle_trees.contains_key(&scope) {
            self.merkle_trees.insert(scope.clone(), ScopedMerkleTree::new(scope.clone()));
        }

        if let Some(tree) = self.merkle_trees.get_mut(&scope) {
            let new_root = tree.insert(&key, value.as_bytes());

            // Update scope metadata with new root hash
            if let Ok(mut meta) = self.load_scope_metadata(&scope)
                .map_err(|e| eval_error!(GenericError(format!("Metadata load error: {}", e))))?
                .ok_or_else(|| eval_error!(GenericError("Scope metadata not found".to_string())))
            {
                meta.root_hash = new_root;
                meta.version += 1;

                let json = serde_json::to_string(&meta).map_eval_error()?;
                let meta_key = format!("scope:{}", scope);
                txn.put(self.scopes_db, &meta_key, &json, WriteFlags::empty()).map_eval_error()?;
            }
        }
        
        if !self.testing {
            txn.commit().map_eval_error()?;
        } else {
            txn.abort();
        }
        
        Ok(())
    }

    /// Remove a key at a scoped path like `/commune/cypherpunx/notes/note001`
    pub fn remove_scoped(&mut self, path: &str) -> EvalResult<()> {
        let (scope, key) = crate::mpt::ScopeManager::parse_path(path).map_eval_error()?;
        self.ensure_scope(&scope).map_eval_error()?;

        let full_key = format!("{}::{}", scope, key);

        let mut txn = self.env.begin_rw_txn().map_eval_error()?;
        match txn.del(self.data_db, &full_key, None) {
            Ok(()) | Err(lmdb::Error::NotFound) => {}
            Err(e) => return Err(eval_error!(GenericError(format!("Remove error: {e}")))),
        }
        txn.commit().map_eval_error()?;
        Ok(())
    }

    /// Wrap this DBResource in Arc<Mutex<>> for use as an SMX custom resource.
    pub fn into_resource(self) -> std::sync::Arc<std::sync::Mutex<dyn smx::value::IoObject + Send>> {
        std::sync::Arc::new(std::sync::Mutex::new(self))
    }
}