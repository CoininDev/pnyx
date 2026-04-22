#![feature(box_patterns)]

use std::{cell::RefCell, fs, path::Path, rc::Rc};

use lmdb::{DatabaseFlags, Transaction};

use crate::rw::DBResource;

mod rw;

fn main() {
    let (env, db) = init_db().expect("Failed to initialize database");

    let mut amb =  smx::val!(ambient);
    amb.add_custom_resource(Rc::new(RefCell::new(DBResource::new(db, env))));

    let x = smx::eval("_ @{DB} = DB.write (\"hello\", \"world\") :\\_. DB.read \"hello\"", &mut amb).unwrap();
    println!("{x}");
}

fn init_db() -> lmdb::Result<(lmdb::Environment, lmdb::Database)> {
    fs::create_dir_all("./meu_banco").unwrap();
    let env = lmdb::Environment::new()
        .set_max_dbs(1)
        .set_map_size(10 * 1024 * 1024)
        .open(Path::new("./meu_banco"))?;

    let db = env.create_db(Some("cu"), DatabaseFlags::empty())?;

    Ok((env, db))
}