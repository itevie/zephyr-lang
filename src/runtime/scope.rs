use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::memory::MemoryAddress;
use super::native_functions::{iter, print, reverse};
use super::values::{Boolean, NativeFunction, Null, RuntimeValue};
use crate::errors::ZephyrError;
use crate::{
  errors::{self, runtime_error},
  lexer::location::Location,
};

#[derive(Debug)]
pub struct Scope {
  pub variables: RefCell<HashMap<String, MemoryAddress>>,
  parent: Option<Rc<Scope>>,
  pub pure_functions_only: RefCell<bool>,
}

impl Scope {
  pub fn new() -> Scope {
    let values: HashMap<String, MemoryAddress> = HashMap::from([
      // Basic values
      ("null", RuntimeValue::Null(Null {})),
      ("true", RuntimeValue::Boolean(Boolean { value: true })),
      ("false", RuntimeValue::Boolean(Boolean { value: false })),
      // Global functions
      (
        "print",
        RuntimeValue::NativeFunction(NativeFunction { func: &print }),
      ),
      (
        "iter",
        RuntimeValue::NativeFunction(NativeFunction { func: &iter }),
      ),
      (
        "reverse",
        RuntimeValue::NativeFunction(NativeFunction { func: &reverse }),
      ),
    ])
    .iter()
    .map(|(key, val)| {
      (String::from(*key), unsafe {
        crate::MEMORY.add_value((*val).clone())
      })
    })
    .collect();
    let x = {
      Scope {
        variables: RefCell::from(values),
        pure_functions_only: RefCell::from(false),
        parent: None,
      }
    };
    x
  }

  pub fn new_with_parent(parent: &Rc<Scope>) -> Scope {
    Scope {
      variables: RefCell::from(HashMap::from([])),
      parent: Some(parent.clone()),
      pure_functions_only: RefCell::from(false),
    }
  }

  pub fn declare_variable(
    &self,
    name: &str,
    value: RuntimeValue,
  ) -> Result<(), errors::ZephyrError> {
    // Check if the variable already exists
    if self.has_variable(name) {
      return Err(runtime_error!(format!(
        "The variable {} already exists",
        name
      )));
    }

    self
      .variables
      .borrow_mut()
      .insert(String::from(name), unsafe {
        crate::MEMORY.add_value(value)
      });
    Ok(())
  }

  pub fn modify_variable(
    &self,
    name: &str,
    new_value: RuntimeValue,
  ) -> Result<RuntimeValue, errors::ZephyrError> {
    let value = self.get_variable_address(name)?;
    unsafe { crate::MEMORY.set_value(value, new_value.clone())? };
    Ok(new_value.clone())
  }

  pub fn has_variable(&self, name: &str) -> bool {
    self.variables.borrow().contains_key(name)
  }

  pub fn get_variable(&self, name: &str) -> Result<RuntimeValue, errors::ZephyrError> {
    let addr = self.get_variable_address(name);
    match addr {
      Ok(val) => unsafe { crate::MEMORY.get_value(val) },
      Err(err) => Err(err),
    }
  }

  pub fn get_variable_address(&self, name: &str) -> Result<MemoryAddress, errors::ZephyrError> {
    if self.has_variable(name) == false {
      // Check if it has a parent
      if let Some(parent) = &self.parent {
        let val = (*parent).get_variable_address(name)?;
        return Ok(val);
      }

      return Err(errors::ZephyrError::runtime(
        format!("The variable {} does not exist", &name),
        Location::no_location(),
      ));
    }

    Ok(*self.variables.borrow().get(name).unwrap())
  }

  pub fn create_child(self: &Rc<Scope>) -> Rc<Scope> {
    Rc::new(Scope {
      parent: Some(self.clone()),
      pure_functions_only: RefCell::from(false),
      variables: RefCell::from(HashMap::new()),
    })
  }
}
