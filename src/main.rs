#![feature(box_patterns)]

use std::{cell::RefCell, fs, path::Path, rc::Rc};

use lmdb::DatabaseFlags;

use crate::db::DBResource;

mod abci;
mod blockchain;
mod db;
mod mpt;
mod runtime;

fn main() {
    let (env, data_db, scopes_db) = init_db().expect("Failed to initialize database");

    let app = abci::PnyxApp::new(env, data_db, scopes_db);

    println!("Starting Pnyx ABCI server on 127.0.0.1:26658...");
    let server = tendermint_abci::ServerBuilder::default()
        .bind("127.0.0.1:26658", app)
        .expect("Failed to bind ABCI server");

    server.listen().expect("ABCI server failed");
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

        // Verify DBResource was created successfully with both databases
        // (Full integration testing would involve reading/writing through DBResource)

        fs::remove_dir_all("./test_bank").ok();
    }

    #[test]
    fn test_scoped_path_parsing() {
        let test_paths = vec![
            ("/commune/rj/laws/001", "commune/rj", "/laws/001"),
            (
                "/commune/sp/members/adbkfng98234jk",
                "commune/sp",
                "/members/adbkfng98234jk",
            ),
            ("/conf/global/rules", "conf", "/global/rules"),
            ("/here/local/cache", "here", "/local/cache"),
        ];

        for (path, expected_scope, expected_key) in test_paths {
            let (scope, key) = crate::mpt::ScopeManager::parse_path(path).unwrap();
            assert_eq!(scope, expected_scope, "Scope mismatch for path {}", path);
            assert_eq!(key, expected_key, "Key mismatch for path {}", path);
        }
    }
}
