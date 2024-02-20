use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, RwLock};

use once_cell::sync::Lazy;

use super::memory::MemoryAddress;
use super::values::{Boolean, Null, RuntimeValue, StringValue};
use crate::errors::ZephyrError;
use crate::{
  errors::{self, runtime_error},
  lexer::location::Location,
};

static CURRENT_SCOPE_ID: Lazy<Arc<Mutex<u128>>> = Lazy::new(|| Arc::from(Mutex::from(0 as u128)));

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
        crate::MEMORY.write().unwrap().add_value((*val).clone()),
      )
    })
    .collect();

    // Get ID
    *CURRENT_SCOPE_ID.lock().unwrap() += 1;
    let id = *CURRENT_SCOPE_ID.lock().unwrap();

    // Create the scope
    let x = {
      Scope {
        variables: Arc::from(RwLock::from(values)),
        exports: RefCell::new(HashMap::new()),
        details: ScopeDetails {
          can_export: RefCell::from(false),
          pure_functions_only: RefCell::from(false),
          directory: RefCell::from(directory.clone()),
        },
        parent_id: None,
        id,
      }
    };

    // Add scope to SCOPES
    crate::SCOPES
      .write()
      .unwrap()
      .insert(id, Arc::from(Mutex::from(x)));

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
      variables: Arc::from(RwLock::from(HashMap::new())),
      exports: RefCell::new(HashMap::new()),
      details,
      id,
    };

    // Add scope to SCOPES
    crate::SCOPES
      .write()
      .unwrap()
      .insert(id, Arc::from(Mutex::from(scope)));

    // Return container
    Ok(ScopeContainer { id })
  }

  pub fn has_variable(&self, name: &str) -> Result<bool, errors::ZephyrError> {
    match crate::SCOPES.read().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok(ok.variables.read().unwrap().contains_key(name)),
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
    match crate::SCOPES.read().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => {
        ok.variables.write().unwrap().insert(
          name.to_string(),
          crate::MEMORY.write().unwrap().add_value(value),
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
    match crate::SCOPES.read().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => {
        ok.variables
          .write()
          .unwrap()
          .insert(name.to_string(), value);
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
      return Err(runtime_error!(format!(
        "The variable {} already exists",
        name
      )));
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
    // Check if disregard
    if name == "_" {
      return Ok(());
    }

    // Check if the variable already exists
    if self.has_variable(name)? {
      return Err(runtime_error!(format!(
        "The variable {} already exists",
        name
      )));
    }

    // Declare it
    self.insert_variable(name, value)?;

    Ok(())
  }

  pub fn get_self_details(&self) -> Result<ScopeDetails, ZephyrError> {
    match crate::SCOPES.read().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok(ok.details.clone()),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn get_exports(&self) -> Result<HashMap<String, MemoryAddress>, ZephyrError> {
    match crate::SCOPES.read().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok(ok.exports.borrow().clone()),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn get_can_export(&self) -> Result<bool, ZephyrError> {
    match crate::SCOPES.read().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok(*ok.details.can_export.borrow()),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn set_can_export(&self, to: bool) -> Result<(), ZephyrError> {
    match crate::SCOPES.read().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok(*ok.details.can_export.borrow_mut() = to),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn get_pure_functions_only(&self) -> Result<bool, ZephyrError> {
    match crate::SCOPES.read().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok(*ok.details.pure_functions_only.borrow()),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn set_pure_functions_only(&self, to: bool) -> Result<(), ZephyrError> {
    match crate::SCOPES.read().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok(*ok.details.pure_functions_only.borrow_mut() = to),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn get_directory(&self) -> Result<String, ZephyrError> {
    match crate::SCOPES.read().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok((*ok.details.directory.borrow()).clone()),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn set_directory(&self, to: String) -> Result<(), ZephyrError> {
    match crate::SCOPES.read().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok(*ok.details.directory.borrow_mut() = to),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn get_parent(&self) -> Result<Option<ScopeContainer>, ZephyrError> {
    match crate::SCOPES.read().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => Ok(if let Some(parent) = ok.parent_id {
        Some(ScopeContainer { id: parent })
      } else {
        None
      }),
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn get_variable_addr(&self, name: &str) -> Result<MemoryAddress, errors::ZephyrError> {
    match crate::SCOPES.read().unwrap().get(&self.id).unwrap().lock() {
      Ok(ok) => {
        // Check if it exists
        if !ok.variables.read().unwrap().contains_key(name) {
          Err(ZephyrError::runtime(
            format!("The variable {} does not exist", name),
            Location::no_location(),
          ))
        } else {
          Ok(*ok.variables.read().unwrap().get(name).unwrap())
        }
      }
      Err(_) => Err(ZephyrError::runtime(
        format!("Failed to get scope with ID: {}", self.id),
        Location::no_location(),
      )),
    }
  }

  pub fn get_variable(&self, name: &str) -> Result<RuntimeValue, errors::ZephyrError> {
    if self.has_variable(name)? == false {
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

    Ok(
      crate::MEMORY
        .read()
        .unwrap()
        .get_value(self.get_variable_addr(name)?)?,
    )
  }

  pub fn get_variable_address(&self, name: &str) -> Result<MemoryAddress, errors::ZephyrError> {
    if self.has_variable(name)? == false {
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

    Ok(self.get_variable_addr(name)?)
  }

  pub fn modify_variable(
    &self,
    name: &str,
    new_value: RuntimeValue,
  ) -> Result<RuntimeValue, errors::ZephyrError> {
    let value = self.get_variable_address(name)?;
    crate::MEMORY
      .write()
      .unwrap()
      .set_value(value, new_value.clone())?;
    Ok(new_value.clone())
  }

  pub fn export(&self, name: String, address: MemoryAddress) -> Result<(), ZephyrError> {
    // Check if current scope can export
    if *self.get_self_details()?.can_export.borrow() {
      match crate::SCOPES.read().unwrap().get(&self.id).unwrap().lock() {
        Ok(ok) => ok.exports.borrow_mut().insert(name.clone(), address),
        Err(_) => {
          return Err(ZephyrError::runtime(
            format!("Failed to get scope with ID: {}", self.id),
            Location::no_location(),
          ))
        }
      };
      crate::debug(
        &format!("Exported {} with memory address {}", name.clone(), address),
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
}

#[derive(Debug)]
pub struct Scope {
  pub id: u128,
  pub parent_id: Option<u128>,
  pub variables: Arc<RwLock<HashMap<String, MemoryAddress>>>,
  pub exports: RefCell<HashMap<String, MemoryAddress>>,
  pub details: ScopeDetails,
}

unsafe impl Sync for ScopeContainer {}
unsafe impl Send for ScopeContainer {}

impl ScopeContainer {}
