use lmdb::DatabaseFlags;
use std::{cell::RefCell, path::Path, rc::Rc};

use crate::{blockchain::Transaction, db::DBResource};
pub struct SMXRuntime {
    test_amb: smx::value::Ambient,
    canon_amb: smx::value::Ambient,
}

fn init_db(name: &str) -> lmdb::Result<(lmdb::Environment, lmdb::Database, lmdb::Database)> {
    std::fs::create_dir_all(&name).unwrap();
    let env = lmdb::Environment::new()
        .set_max_dbs(2)
        .set_map_size(10 * 1024 * 1024)
        .open(Path::new(&name))?;

    let data_db = env.create_db(Some("pnyx"), DatabaseFlags::empty())?;
    let scopes_db = env.create_db(Some("scopes"), DatabaseFlags::empty())?;

    Ok((env, data_db, scopes_db))
}

impl SMXRuntime {
    pub fn new() -> Result<Self, String> {
        let (tenv, tdb1, tdb2) = init_db("test_db").map_err(|e| e.to_string())?;
        let mut test_amb = smx::val!(ambient);
        test_amb.add_custom_resource(Rc::new(RefCell::new(DBResource::new(tdb1, tdb2, tenv))));

        let (env, db1, db2) = init_db("db").map_err(|e| e.to_string())?;
        let mut canon_amb = smx::val!(ambient);
        canon_amb.add_custom_resource(Rc::new(RefCell::new(DBResource::new(db1, db2, env))));

        Ok(Self {
            test_amb,
            canon_amb,
        })
    }

    pub fn validate_tx(&self, transaction: &Transaction) -> bool {}
}
