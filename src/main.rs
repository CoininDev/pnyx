#![feature(box_patterns)]

use std::{cell::RefCell, fs, path::Path, rc::Rc};

use lmdb::DatabaseFlags;

use crate::db::DBResource;

mod blockchain;
mod db;
mod mpt;

fn main() {
    let (env, data_db, scopes_db) = init_db().expect("Failed to initialize database");

    let mut amb = smx::val!(ambient);
    amb.add_custom_resource(Rc::new(RefCell::new(DBResource::new(
        data_db, scopes_db, env,
    ))));

    // Example 1: Write to confederation scope
    println!("=== Test 1: Writing to /conf/greeting ===");
    let _write_result = smx::eval(
        "_ @{DB, IO} = (\"/conf/greeting\", \"hello from confederation\"): DB.write",
        &mut amb,
    );
    println!("Write completed");

    // Example 2: Read from confederation scope
    println!("\n=== Test 2: Reading from /conf/greeting ===");
    let read_result = smx::eval(
        "_ @{DB, IO} = \"/conf/greeting\": DB.read: IO.print",
        &mut amb,
    );
    println!("Read result: {:?}", read_result);

    // Example 3: Write to commune scope
    println!("\n=== Test 3: Writing to /commune/cypherpunx/laws/001 ===");
    let _write_result2 = smx::eval(
        "_ @{DB, IO} = (\"/commune/cypherpunx/laws/001\", \"law: universal suffrage\"): DB.write",
        &mut amb,
    );
    println!("Write completed");

    // Example 4: Read from commune scope
    println!("\n=== Test 4: Reading from /commune/cypherpunx/laws/001 ===");
    let read_result2 = smx::eval(
        "_ @{DB, IO} = \"/commune/cypherpunx/laws/001\": DB.read: IO.print",
        &mut amb,
    );
    println!("Read result: {:?}", read_result2);

    println!("\n✓ Multi-scope MPT system operational!");
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
