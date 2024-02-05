#[derive(Clone, Debug)]
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
}
