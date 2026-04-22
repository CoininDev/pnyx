use lmdb::Transaction;
use lmdb::WriteFlags;
use smx::eval_error;
use smx::value::IoObject;
use smx::error::EvalError;
use smx::error::EvalErrorType::*;
use smx::value::Value;

pub struct DBResource {
    db: lmdb::Database,
    env: lmdb::Environment,
}

impl DBResource {
    pub fn new(db: lmdb::Database, env: lmdb::Environment) -> Self {
        Self {db, env}
    }
}

impl IoObject for DBResource {
    fn name(&self) -> &str {
        "DB"
    }

    fn redirect(&mut self, function: Vec<String>, value: smx::value::Value, amb: &mut smx::value::Ambient) -> smx::eval::EvalResult<smx::value::Value> {
        match function.as_slice() {
            fun if fun == &["read"] => {
                if let smx::value::Value::Str(key) = value {
                    let txn = self.env.begin_ro_txn().unwrap();
                    match txn.get(self.db, &key) {
                        Ok(val) => Ok(smx::val!(String::from_utf8_lossy(val).to_string())),
                        Err(_) => Ok(smx::val!())
                    }
                } else {
                    Err(eval_error!(WrongTypes(function.join("."), smx::value::PatternType::String, value)))
                }
            }
            fun if fun == &["write"] => match value.clone() {
                Value::Pair(box Value::Str(key), box Value::Str(val)) => {
                    let mut txn = self.env.begin_rw_txn()
                        .map_err(|x| eval_error!(GenericError(x.to_string())))?;
                    txn.put(self.db, &key, &val, WriteFlags::empty());
                    txn.commit().map_err(|x| eval_error!(GenericError(x.to_string())))?;
                    Ok(smx::val!())
                }
                Value::Pair(box Value::Str(key), box val) => {
                    let mut txn = self.env.begin_rw_txn()
                        .map_err(|x| eval_error!(GenericError(x.to_string())))?;

                    let val = serde_json::to_string(&val).unwrap();
                    txn.put(self.db, &key, &val, WriteFlags::empty());
                    txn.commit().map_err(|x| eval_error!(GenericError(x.to_string())))?;
                    Ok(smx::val!())
                }
                cu => Err(eval_error!(WrongTypes(function.join("."), smx::value::PatternType::String, value)))
            }
            
            cu => Err(EvalError::new(smx::error::EvalErrorType::VariableDoesNotExists(cu.join(".")))),
        }
    }
}