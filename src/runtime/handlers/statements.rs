use std::{cell::RefCell, rc::Rc};

use crate::{
  errors::ZephyrError,
  parser::nodes::{FunctionDeclaration, VariableDeclaration},
  runtime::{
    interpreter::interpret,
    scope::Scope,
    values::{Function, Null, RuntimeValue},
  },
};

pub fn variable_declaration(
  expr: VariableDeclaration,
  scope: &mut Scope,
) -> Result<RuntimeValue, ZephyrError> {
}

pub fn function_declaration(
  stmt: FunctionDeclaration,
  scope: &mut Scope,
) -> Result<RuntimeValue, ZephyrError> {
}
