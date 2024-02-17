use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::memory::MemoryAddress;
use super::values::{Boolean, Null, RuntimeValue, StringValue};
use crate::errors::ZephyrError;
use crate::{
  errors::{self, runtime_error},
  lexer::location::Location,
};

#[derive(Debug)]
pub struct Scope {
  pub variables: RefCell<HashMap<String, MemoryAddress>>,
  pub exports: RefCell<HashMap<String, MemoryAddress>>,
  pub can_export: RefCell<bool>,
  pub parent: Option<Rc<Scope>>,
  pub pure_functions_only: RefCell<bool>,
  pub directory: RefCell<String>,
}

impl Scope {
  pub fn new(directory: String) -> Scope {
    let values: HashMap<String, MemoryAddress> = HashMap::from([
      // Basic values
      ("null", RuntimeValue::Null(Null {})),
      ("true", RuntimeValue::Boolean(Boolean { value: true })),
      ("false", RuntimeValue::Boolean(Boolean { value: false })),
      (
        "__dirname",
        RuntimeValue::StringValue(StringValue {
          value: directory.clone(),
        }),
      ),
    ])
    .iter()
    .map(|(key, val)| {
      (
        String::from(*key),
        crate::MEMORY.lock().unwrap().add_value((*val).clone()),
      )
    })
    .collect();
    let x = {
      Scope {
        variables: RefCell::from(values),
        exports: RefCell::new(HashMap::new()),
        can_export: RefCell::from(false),
        pure_functions_only: RefCell::from(false),
        parent: None,
        directory: RefCell::from(directory.clone()),
      }
    };
    x
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

    self.variables.borrow_mut().insert(
      String::from(name),
      crate::MEMORY.lock().unwrap().add_value(value),
    );
    Ok(())
  }

  pub fn modify_variable(
    &self,
    name: &str,
    new_value: RuntimeValue,
  ) -> Result<RuntimeValue, errors::ZephyrError> {
    let value = self.get_variable_address(name)?;
    crate::MEMORY
      .lock()
      .unwrap()
      .set_value(value, new_value.clone())?;
    Ok(new_value.clone())
  }

  pub fn has_variable(&self, name: &str) -> bool {
    self.variables.borrow().contains_key(name)
  }

  pub fn get_variable(&self, name: &str) -> Result<RuntimeValue, errors::ZephyrError> {
    let addr = self.get_variable_address(name);
    match addr {
      Ok(val) => crate::MEMORY.lock().unwrap().get_value(val),
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

  pub fn export(&self, name: String, address: MemoryAddress) -> Result<(), ZephyrError> {
    // Check if current scope can export
    if *self.can_export.borrow() {
      self.exports.borrow_mut().insert(name.clone(), address);
      crate::debug(
        &format!("Exported {} with memory address {}", name.clone(), address),
        "scope",
      );
    } else {
      // Check if has parent
      if let Some(parent) = &self.parent {
        parent.export(name, address)?;
      } else {
        return Err(ZephyrError::runtime(
          "Cannot export here".to_string(),
          Location::no_location(),
        ));
      }
    }

    Ok(())
  }

  pub fn create_child(self: &Rc<Scope>) -> Rc<Scope> {
    Rc::new(Scope {
      parent: Some(self.clone()),
      pure_functions_only: RefCell::from(*self.pure_functions_only.borrow()),
      variables: RefCell::from(HashMap::new()),
      exports: RefCell::new(HashMap::new()),
      can_export: RefCell::from(false),
      directory: RefCell::from(self.directory.clone()),
    })
  }
}
