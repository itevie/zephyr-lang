pub fn fg_red() -> String {
  "\x1b[31m".to_string()
}

pub fn fg_cyan() -> String {
  "\x1b[36m".to_string()
}

pub fn reset() -> String {
  "\x1b[0m".to_string()
}
