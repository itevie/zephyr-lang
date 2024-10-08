use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;

use super::memory::MemoryAddress;
use super::values::{Boolean, Null, RuntimeValue, StringValue};
use crate::errors::ZephyrError;
use crate::util;
use crate::{
  errors::{self},
  lexer::location::Location,
};

static CURRENT_SCOPE_ID: Lazy<Arc<Mutex<u128>>> = Lazy::new(|| Arc::from(Mutex::from(0_u128)));

#[derive(Debug, Copy, Clone)]
pub struct ScopeContainer {
  pub id: u128,
}

impl ScopeContainer {
  pub fn new(directory: String) -> ScopeContainer {
    // Create default values
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

    // Get ID
    *CURRENT_SCOPE_ID.lock().unwrap() += 1;
    let id = *CURRENT_SCOPE_ID.lock().unwrap();

    // Create the scope
    let x = {
      Scope {
        variables: RefCell::from(values),
        exports: RefCell::new(HashMap::new()),
        details: ScopeDetails {
          can_export: RefCell::from(false),
          pure_functions_only: RefCell::from(false),
          directory: RefCell::from(directory),
          file: RefCell::from(None),
        },
        parent_id: None,
        id,
      }
    };

    // Add scope to SCOPES
    crate::SCOPES
      .lock()
      .unwrap()
      .insert(id, Arc::from(Mutex::from(x)));

    util::verbose(
      &format!(
        "{} created, there are now {} scopes",
        id,
        crate::SCOPES.lock().unwrap().len()
      ),
      "scope",
    );

    // Return container
    ScopeContainer { id }
  }

  pub fn create_child(&self) -> Result<ScopeContainer, ZephyrError> {
    let details = self.get_self_details()?;

    // Get ID
    *CURRENT_SCOPE_ID.lock().unwrap() += 1;
    let id = *CURRENT_SCOPE_ID.lock().unwrap();

    // Create the scope
    let scope = Scope {
      parent_id: Some(self.id),
      variables: RefCell::from(HashMap::new()),
      exports: RefCell::new(HashMap::new()),
      details,
      id,
    };

    // Add scope to SCOPES
    crate::SCOPES
      .lock()
      .unwrap()
      .insert(id, Arc::from(Mutex::from(scope)));

    util::verbose(
      &format!(
        "{} created, there are now {} scopes",
        id,
        crate::SCOPES.lock().unwrap().len()
      ),
      "scope",
    );

    // Return container
    Ok(ScopeContainer { id })
  }

  pub fn delete(&self) -> Result<(), ZephyrError> {
    // Check if scope exists
    if !crate::SCOPES.lock().unwrap().contains_key(&self.id) {
      return Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      ));
    }

    // Delete
    crate::SCOPES.lock().unwrap().remove(&self.id);

    util::verbose(
      &format!(
        "{} deleted, there are now {} scopes",
        self.id,
        crate::SCOPES.lock().unwrap().len()
      ),
      "scope",
    );

    Ok(())
  }

  pub fn has_variable(&self, name: &str) -> Result<bool, errors::ZephyrError> {
    util::verbose(&format!("{} has variable {}", self.id, name), "scope READ");
    match { crate::SCOPES.lock().unwrap().get(&self.id) }
      .unwrap()
      .lock()
    {
      Ok(ok) => Ok(ok.variables.borrow().contains_key(name)),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn insert_variable(
    &self,
    name: &str,
    value: RuntimeValue,
  ) -> Result<(), errors::ZephyrError> {
    util::verbose(
      &format!("{} insert variable {}", self.id, name),
      "scope WRITE",
    );
    let scope = { crate::SCOPES.lock().unwrap().get(&self.id).cloned() };
    match scope.unwrap().lock() {
      Ok(ok) => {
        ok.variables.borrow_mut().insert(
          name.to_string(),
          crate::MEMORY.lock().unwrap().add_value(value),
        );
        Ok(())
      }
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn insert_variable_with_address(
    &self,
    name: &str,
    value: MemoryAddress,
  ) -> Result<(), errors::ZephyrError> {
    util::verbose(
      &format!("{} insert variable {} = &{}", self.id, name, value),
      "scope WRITE",
    );
    match crate::SCOPES.lock().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => {
        ok.variables.borrow();
        ok.variables.borrow_mut().insert(name.to_string(), value);
        Ok(())
      }
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn declare_variable_with_address(
    &self,
    name: &str,
    value: MemoryAddress,
  ) -> Result<(), errors::ZephyrError> {
    // Check if disregard
    if name == "_" {
      return Ok(());
    }

    // Check if the variable already exists
    if self.has_variable(name)? {
      return Err(ZephyrError::runtime(
        format!("The variable {} already exists", name),
        Location::no_location(),
      ));
    }

    // Declare it
    self.insert_variable_with_address(name, value)?;

    Ok(())
  }

  pub fn declare_variable(
    &self,
    name: &str,
    value: RuntimeValue,
  ) -> Result<(), errors::ZephyrError> {
    util::verbose(
      &format!("{} declare variable {}", self.id, name),
      "scope WRITE",
    );
    // Check if disregard
    if name == "_" {
      return Ok(());
    }

    // Check if the variable already exists
    if self.has_variable(name)? {
      return Err(ZephyrError::runtime(
        format!("The variable {} already exists", name),
        Location::no_location(),
      ));
    }

    // Declare it
    self.insert_variable(name, value)?;

    Ok(())
  }

  pub fn get_self_details(&self) -> Result<ScopeDetails, ZephyrError> {
    util::verbose(&format!("{} details", self.id), "scope READ");
    match crate::SCOPES.lock().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok(ok.details.clone()),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn get_exports(&self) -> Result<HashMap<String, MemoryAddress>, ZephyrError> {
    match crate::SCOPES.lock().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok(ok.exports.borrow().clone()),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn get_can_export(&self) -> Result<bool, ZephyrError> {
    match crate::SCOPES.lock().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok(*ok.details.can_export.borrow()),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn set_can_export(&self, to: bool) -> Result<(), ZephyrError> {
    match crate::SCOPES.lock().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => {
        *ok.details.can_export.borrow_mut() = to;
        Ok(())
      }
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn get_pure_functions_only(&self) -> Result<bool, ZephyrError> {
    match crate::SCOPES.lock().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok(*ok.details.pure_functions_only.borrow()),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn set_pure_functions_only(&self, to: bool) -> Result<(), ZephyrError> {
    match crate::SCOPES.lock().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => {
        *ok.details.pure_functions_only.borrow_mut() = to;
        Ok(())
      }
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn get_directory(&self) -> Result<String, ZephyrError> {
    match crate::SCOPES.lock().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok((*ok.details.directory.borrow()).clone()),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn set_directory(&self, to: String) -> Result<(), ZephyrError> {
    match crate::SCOPES.lock().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => {
        *ok.details.directory.borrow_mut() = to;
        Ok(())
      }
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn get_file(&self) -> Result<Option<String>, ZephyrError> {
    match crate::SCOPES.lock().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok((*ok.details.file.borrow()).clone()),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn set_file(&self, to: Option<String>) -> Result<(), ZephyrError> {
    match crate::SCOPES.lock().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => {
        *ok.details.file.borrow_mut() = to;
        Ok(())
      }
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn get_parent(&self) -> Result<Option<ScopeContainer>, ZephyrError> {
    util::verbose(&format!("{} parent", self.id), "scope READ");
    match crate::SCOPES.lock().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok(ok.parent_id.map(|parent| ScopeContainer { id: parent })),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn get_variable_addr(&self, name: &str) -> Result<MemoryAddress, errors::ZephyrError> {
    match crate::SCOPES.lock().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => {
        // Check if it exists
        if !ok.variables.borrow().contains_key(name) {
          Err(ZephyrError::runtime(
            format!("The variable {} does not exist", name),
            Location::no_location(),
          ))
        } else {
          Ok(*ok.variables.borrow().get(name).unwrap())
        }
      }
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn get_variable(&self, name: &str) -> Result<RuntimeValue, errors::ZephyrError> {
    util::verbose(&format!("{} get variable {}", self.id, name), "scope READ");
    if !(self.has_variable(name)?) {
      // Check if it has a parent
      if let Some(parent) = self.get_parent()? {
        let val = parent.get_variable(name)?;
        return Ok(val);
      }

      return Err(errors::ZephyrError::runtime(
        format!("The variable {} does not exist", &name),
        Location::no_location(),
      ));
    }

    crate::MEMORY
      .lock()
      .unwrap()
      .get_value(self.get_variable_addr(name)?)
  }

  pub fn get_variable_address(&self, name: &str) -> Result<MemoryAddress, errors::ZephyrError> {
    if !(self.has_variable(name)?) {
      // Check if it has a parent
      if let Some(parent) = self.get_parent()? {
        let val = parent.get_variable_address(name)?;
        return Ok(val);
      }

      return Err(errors::ZephyrError::runtime(
        format!("The variable {} does not exist", &name),
        Location::no_location(),
      ));
    }

    self.get_variable_addr(name)
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
    Ok(new_value)
  }

  pub fn export(&self, name: String, address: MemoryAddress) -> Result<(), ZephyrError> {
    // Check if current scope can export
    if *self.get_self_details()?.can_export.borrow() {
      match crate::SCOPES.lock().unwrap().get(&self.id).unwrap().lock() {
        Ok(ok) => ok.exports.borrow_mut().insert(name.clone(), address),
        Err(_) => {
          return Err(ZephyrError::runtime(
            format!("Failed to get scope with ID: {}", self.id),
            Location::no_location(),
          ))
        }
      };
      util::verbose(
        &format!("Exported {} with memory address {}", name, address),
        "scope",
      );
    } else {
      // Check if has parent
      if let Some(parent) = &self.get_parent()? {
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
}

#[derive(Debug, Clone)]
pub struct ScopeDetails {
  pub can_export: RefCell<bool>,
  pub pure_functions_only: RefCell<bool>,
  pub directory: RefCell<String>,
  pub file: RefCell<Option<String>>,
}

#[derive(Debug)]
pub struct Scope {
  pub id: u128,
  pub parent_id: Option<u128>,
  pub variables: RefCell<HashMap<String, MemoryAddress>>,
  pub exports: RefCell<HashMap<String, MemoryAddress>>,
  pub details: ScopeDetails,
}

unsafe impl Sync for ScopeContainer {}
unsafe impl Send for ScopeContainer {}

impl ScopeContainer {}
