use std::collections::HashMap;

use crate::runtime::values::{Number, Object, RuntimeValue};

#[derive(Clone, Copy, Debug)]
pub struct Location {
  pub char_start: u32,
  pub char_end: u32,
  pub line: u32,
  pub location_contents: u128,
}

impl Location {
  pub fn no_location() -> Location {
    Location {
      char_start: 0,
      char_end: 0,
      line: 0,
      location_contents: 0,
    }
  }

  pub fn combine_with(&self, location: Location) -> Location {
    Location {
      char_start: self.char_start,
      char_end: location.char_end,
      line: self.line,
      location_contents: self.location_contents,
    }
  }

  pub fn to_object(&self) -> RuntimeValue {
    Object::make(HashMap::from([
      (
        "char_start".to_string(),
        RuntimeValue::Number(Number {
          value: self.char_start as f64,
        }),
      ),
      (
        "char_end".to_string(),
        RuntimeValue::Number(Number {
          value: self.char_end as f64,
        }),
      ),
      (
        "line".to_string(),
        RuntimeValue::Number(Number {
          value: self.line as f64,
        }),
      ),
      (
        "contents_id".to_string(),
        RuntimeValue::Number(Number {
          value: self.location_contents as f64,
        }),
      ),
    ]))
    .create_container()
  }
}
