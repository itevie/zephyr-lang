use std::collections::HashMap;

use crate::errors::ZephyrError;
use crate::lexer::location::Location;

use super::values::RuntimeValue;

pub type MemoryAddress = u128;

#[derive(Debug, Clone)]
pub struct Memory {
  current_address: MemoryAddress,
  memory: HashMap<MemoryAddress, RuntimeValue>,
}

impl Memory {
  pub fn new() -> Memory {
    Memory {
      current_address: 0,
      memory: HashMap::new(),
    }
  }

  pub fn add_value(&mut self, value: RuntimeValue) -> MemoryAddress {
    let current_address = self.current_address;

    // Add value
    self.memory.insert(current_address, value);

    // Modify and return current address
    self.current_address += 1;
    current_address
  }

  pub fn set_value(
    &mut self,
    address: MemoryAddress,
    value: RuntimeValue,
  ) -> Result<MemoryAddress, ZephyrError> {
    self.get_value(address)?;
    self.memory.remove(&address);
    self.memory.insert(address, value);
    return Ok(address);
  }

  pub fn get_value(&self, address: MemoryAddress) -> Result<RuntimeValue, ZephyrError> {
    // Check if memory contains it
    if !self.memory.contains_key(&address) {
      return Err(ZephyrError::runtime(
        format!("Unknown address {}", address),
        Location::no_location(),
      ));
    }

    // Else get the value
    let value = self.memory.get(&address).unwrap().clone();
    Ok(value)
  }
}
