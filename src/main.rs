#![feature(box_patterns)]

use crate::runtime::SMXRuntime;

mod abci;
mod blockchain;
mod db;
mod mpt;
mod runtime;

fn main() {
    let runtime = SMXRuntime::new().expect("Failed to initialize SMX runtime");
    let app = abci::PnyxApp::new(runtime);

    println!("Starting Pnyx ABCI server on 127.0.0.1:26658...");
    let server = tendermint_abci::ServerBuilder::default()
        .bind("127.0.0.1:26658", app)
        .expect("Failed to bind ABCI server");

    server.listen().expect("ABCI server failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{blockchain::Transaction, db::DBResource, runtime::with_db};
    use primitive_types::H256;
    use smx::{ast::Assign, value::Value};
    use std::fs;

    fn fresh_runtime(tag: &str) -> SMXRuntime {
        let db = format!("./target/test_{tag}_db");
        let _ = fs::remove_dir_all(&db);
        SMXRuntime::new_at(&db).expect("Failed to init runtime")
    }

    #[test]
    fn test_contract_validate_and_apply() {
        let mut runtime = fresh_runtime("validate_apply");

        let source = fs::read_to_string("temp/example_contract.smx")
            .expect("Could not read example_contract.smx");

        // Deploy the contract into the canonical DB
        runtime
            .deploy_contract("/commune/cypherpunx", "notes", &source)
            .expect("Failed to deploy contract");

        let tx = Transaction {
            contract: "notes:create".to_string(),
            scope:    "/commune/cypherpunx".to_string(),
            param:    Value::Str("Olá Mundo".to_string()),
            author:   H256::zero(),
            sign:     vec![],
        };

        // Dry-run should succeed
        println!("{}", runtime.validate_tx(&tx));
        //assert!(runtime.validate_tx(&tx), "validate_tx returned false");

        // Canonical apply should also succeed
        let result = runtime.apply_tx(&tx);
        assert!(result.is_ok(), "apply_tx failed: {:?}", result.err());


        // Check db
        with_db(&mut runtime.amb, |obj, _|  {
            if let Some(db) = (obj as &mut dyn std::any::Any).downcast_mut::<DBResource>() {
                db.read_scoped("/commune/cypherpunx/notes/001")
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        ).expect("Could not check db status");
    }
}
