use std::{collections::HashMap, sync::Arc};

use crate::{
    errors::{ErrorCode, ZephyrError},
    parser::nodes::{self, Node},
    runtime::memory_store::OBJECT_STORE,
};

use super::{
    memory_store::store_get,
    values::{self, RuntimeValue},
    Interpreter, R,
};

impl Interpreter {
    pub fn run_member(&mut self, expr: nodes::Member, set: Option<RuntimeValue>) -> R {
        let left = self.run(*expr.left.clone())?;

        // Check if it is being accessed via x.a
        if !expr.computed {
            let key = match *expr.right {
                Node::Symbol(sym) => sym.value,
                _ => unreachable!(),
            };

            return self.member_check_basic(left.clone(), key, set);
        } else {
            let right = self.run(*expr.right.clone())?.check_ref()?;

            // Check for basic string key
            if let RuntimeValue::ZString(string) = right.0 {
                return self.member_check_basic(left.clone(), string.value, set);
            }
        }

        Ok(values::Null::new())
    }

    pub fn member_check_basic(
        &mut self,
        value: RuntimeValue,
        key: String,
        set: Option<RuntimeValue>,
    ) -> R {
        // Prescedence:
        // - __tag check
        // - object property check
        // - property chain check

        if &key == "__tags" {
            if let Some(_) = set {
                return Err(ZephyrError {
                    message: "Cannot assign to a value's __tags".to_string(),
                    code: ErrorCode::InvalidOperation,
                    location: None,
                });
            }

            return Ok(values::Object::new(
                value
                    .get_options()
                    .tags
                    .lock()
                    .unwrap()
                    .clone()
                    .iter()
                    .map(|v| (v.0.clone(), values::ZString::new(v.1.clone())))
                    .collect::<HashMap<String, RuntimeValue>>(),
            ));
        }

        match value.check_ref()? {
            (RuntimeValue::Object(mut obj), Some(r)) => {
                if let Some(setter) = set {
                    if obj.items.contains_key(&key) {
                        obj.items.remove(&key);
                    }

                    obj.items.insert(key, setter);

                    let mut lock = OBJECT_STORE.get().unwrap().lock().unwrap();
                    lock.remove(r.location);
                    lock.insert(r.location, Some(Arc::from(RuntimeValue::Object(obj))));

                    return Ok(values::Null::new());
                } else if let Some(val) = obj.items.get(&key) {
                    return Ok(val.clone());
                }
            }
            _ => (),
        }

        if let Some(proto_ref) = value.get_options().proto {
            let value = match store_get(proto_ref) {
                RuntimeValue::Object(o) => o,
                _ => panic!("Expected an object as the prototype."),
            };

            if let Some(proto_value) = value.items.get(&key) {
                return Ok(proto_value.clone());
            }
        }

        Ok(values::Null::new())
    }
}
