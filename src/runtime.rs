use lmdb::DatabaseFlags;
use std::path::Path;

use crate::{blockchain::Transaction, db::DBResource};

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

fn init_lmdb(name: &str) -> lmdb::Result<(lmdb::Environment, lmdb::Database, lmdb::Database)> {
    std::fs::create_dir_all(name).unwrap();
    let env = lmdb::Environment::new()
        .set_max_dbs(2)
        .set_map_size(10 * 1024 * 1024)
        .open(Path::new(name))?;
    let data_db   = env.create_db(Some("pnyx"),   DatabaseFlags::empty())?;
    let scopes_db = env.create_db(Some("scopes"), DatabaseFlags::empty())?;
    Ok((env, data_db, scopes_db))
}

fn make_ambient(db: DBResource) -> smx::value::Ambient {
    let mut amb = smx::val!(ambient);
    amb.add_custom_resource(db.into_resource());
    amb
}

// ─────────────────────────────────────────────────────────────────────────────
// SMXRuntime
// ─────────────────────────────────────────────────────────────────────────────

pub struct SMXRuntime {
    pub amb: smx::value::Ambient,
}

impl SMXRuntime {
    pub fn new() -> Result<Self, String> {
        Self::new_at("db")
    }

    /// Constructor with explicit directory paths (useful in tests).
    pub fn new_at(path: &str) -> Result<Self, String> {
        let (env,  db1,  db2)  = init_lmdb(path).map_err(|e| e.to_string())?;
        Ok(Self {
            amb: make_ambient(DBResource::new(db1, db2, env)),
        })
    }

    // ── Public API ────────────────────────────────────────────────────────────


    pub fn validate_tx(&mut self, tx: &Transaction) -> bool {
        self.run_tx(tx, false).is_ok()
    }
 
    pub fn apply_tx(&mut self, tx: &Transaction) -> Result<smx::value::Value, String> {
        self.run_tx(tx, true)
    }

    /// Path: `"{scope}/contracts/{name}"`.
    pub fn deploy_contract(&mut self, scope: &str, name: &str, source: &str) -> Result<(), String> {
        let path = format!("{}/contracts/{}", scope.trim_end_matches('/'), name);
        write_to_amb(&mut self.amb, &path, source)
    }

    // ── Core pipeline ─────────────────────────────────────────────────────────

    /// `testing = false`  → commit
    /// `testing = true`   → abort
    fn run_tx(&mut self, tx: &Transaction, testing: bool) -> Result<smx::value::Value, String> {
        // 1. Parse "notes:create" → ("notes", "create")
        let (contract_name, func_name) = parse_contract_field(&tx.contract)?;

        // 2. Build contract path
        let contract_path = format!(
            "{}/contracts/{}",
            tx.scope.trim_end_matches('/'),
            contract_name
        );

        // 3. Read contract source from db
        //    Cloning the ambient is cheap (Arc clone); the underlying LMDB
        //    connection is shared, so reads see the real committed state.
        let source = {
            let mut reader = self.amb.clone();
            read_from_amb(&mut reader, &contract_path)
                .map_err(|e| format!("Failed to load contract '{}': {}", contract_path, e))?
        };

        // 4. Choose execution testing
        with_db(&mut self.amb, |db, _tx| {
            if let Some(db_t) = (db as &mut dyn std::any::Any).downcast_mut::<DBResource>() {
                if testing {db_t.testing = true;}
                else {db_t.testing = false;}
            } 
            Ok(())
        })?;

        // 5. Evaluate the contract source in the execution ambient
        let contract_amb = eval_contract_source(&source, &self.amb)
            .map_err(|e| format!("Contract eval error: {}", e))?;

        // 6. Extract the named function from contract.funcs
        let func = extract_func(&contract_amb, func_name)
            .map_err(|e| format!("Function lookup error: {}", e))?;

        // 7. Inject `tx_scope` so SMX code can build absolute DB paths
        self.amb.vars.insert(
            "tx_scope".to_string(),
            smx::val!(tx.scope.clone()),
        );

        // 8. Apply the function to tx.param
        smx::eval::apply(func, tx.param.clone(), &mut self.amb)
            .map_err(|e| format!("Contract execution error: {}", e))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// DB helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Call `f` with the first DB custom resource found in `amb`.
pub fn with_db<F, R>(amb: &mut smx::value::Ambient, f: F) -> Result<R, String>
where
    F: FnOnce(&mut dyn smx::value::IoObject, &mut smx::value::Ambient) -> Result<R, String>,
{
    let custom = amb.custom_resources.clone();
    for res in &custom {
        if res.lock().unwrap().name() == "DB" {
            let mut guard = res.lock().unwrap();
            return f(&mut *guard, amb);
        }
    }
    Err("No DB resource found in ambient".to_string())
}

fn read_from_amb(amb: &mut smx::value::Ambient, path: &str) -> Result<String, String> {
    with_db(amb, |db, amb| {
        let result = db
            .redirect(
                vec!["read".to_string()],
                smx::value::Value::Str(path.to_string()),
                amb,
            )
            .map_err(|e| e.to_string())?;
        match result {
            smx::value::Value::Str(s) => Ok(s),
            smx::value::Value::Nil    => Err(format!("Key not found: '{}'", path)),
            other                     => Err(format!("Unexpected value at '{}': {}", path, other)),
        }
    })
}

fn write_to_amb(amb: &mut smx::value::Ambient, path: &str, value: &str) -> Result<(), String> {
    // smx::eval(format!("_ @{{DB}} = \"{path}\",\"{value}\":DB.write").as_str(), amb)
    //     .map(|_| ())

    with_db(amb, |db, amb| {
        db.redirect(
            vec!["write".to_string()],
            smx::value::Value::Pair(
                Box::new(smx::val!(path.to_string())),
                Box::new(smx::val!(value.to_string())),
            ),
            amb,
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// SMX evaluation helpers
// ─────────────────────────────────────────────────────────────────────────────

fn parse_contract_field(s: &str) -> Result<(&str, &str), String> {
    let mut parts = s.splitn(2, ':');
    let contract = parts.next().ok_or("Empty contract field")?;
    let func     = parts.next()
        .ok_or_else(|| format!("Missing function name in '{}'", s))?;
    Ok((contract, func))
}

/// Evaluate SMX source starting from a copy of `parent_amb`.
/// Returns the resulting ambient (contains the `contract` variable etc.).
fn eval_contract_source(
    source: &str,
    parent_amb: &smx::value::Ambient,
) -> Result<smx::value::Ambient, String> {
    use smx::{ast::Parser, eval::{eval_assign, eval_resource}, lexer::Lexer};

    let tokens = Lexer::new(source)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let program = Parser::new(tokens)
        .parse_program()
        .map_err(|e| e.to_string())?;

    let mut amb = parent_amb.clone();

    for assign in &program.body {
        let _ = eval_resource(assign, &mut amb.rsrcs);
    }
    for assign in program.body {
        eval_assign(assign, &mut amb).map_err(|e| e.to_string())?;
    }

    Ok(amb)
}

/// Look up `contract.funcs.<func_name>` in the evaluated ambient.
fn extract_func(
    amb: &smx::value::Ambient,
    func_name: &str,
) -> Result<smx::value::Value, String> {
    let contract_val = amb.vars
        .get("contract")
        .cloned()
        .ok_or("No 'contract' variable found in contract source")?;

    let funcs_env = match contract_val {
        smx::value::Value::Environment(env) => {
            match env.get("funcs").cloned() {
                Some(smx::value::Value::Environment(f)) => f,
                Some(other) => return Err(format!("'funcs' is not an environment: {}", other)),
                None        => return Err("Contract has no 'funcs' field".to_string()),
            }
        }
        other => return Err(format!("'contract' is not an environment: {}", other)),
    };

    funcs_env
        .get(func_name)
        .cloned()
        .ok_or_else(|| format!("Function '{}' not found in contract funcs", func_name))
}
