use std::rc::Rc;

use crate::{
  errors,
  parser::nodes::{Block, Program},
  runtime::{
    interpreter::interpret,
    scope::Scope,
    values::{Null, RuntimeValue},
  },
};

pub fn program(prog: Program, scope: Rc<Scope>) -> Result<RuntimeValue, errors::ZephyrError> {
  let mut last_value: Option<RuntimeValue> = None;

  for expr in prog.nodes {
    last_value = Some(match interpret(*expr, scope) {
      Ok(val) => val,
      Err(err) => return Err(err),
    });
  }

  match last_value {
    None => Ok(RuntimeValue::Null(Null {})),
    Some(val) => Ok(val),
  }
}

pub fn block(prog: Block, scope: Rc<Scope>) -> Result<RuntimeValue, errors::ZephyrError> {
  let mut last_value: Option<RuntimeValue> = None;
  let mut scope = Rc::from(Scope::new_with_parent(scope));
  for expr in prog.nodes {
    last_value = Some(match interpret(*expr, scope) {
      Ok(val) => val,
      Err(err) => return Err(err),
    });
  }

  match last_value {
    None => Ok(RuntimeValue::Null(Null {})),
    Some(val) => Ok(val),
  }
}
