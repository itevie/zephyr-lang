use std::fs;
use std::io::ErrorKind;

pub fn read_file(path: String) -> String {
  match std::fs::read_to_string(path.clone()) {
    Ok(ok) => ok,
    Err(err) => {
      super::die(match err.kind() {
        ErrorKind::NotFound => format!("File {} does not exist", path),
        ErrorKind::PermissionDenied => format!("Failed to read {}: permission denied", path),
        _ => format!("Failed to open {}: {}", path, err),
      });
      return "".to_string();
    }
  }
}

pub fn write_file(path: &str, contents: String) -> () {
  match fs::write(path, contents) {
    Ok(_) => (),
    Err(err) => {
      super::die(match err.kind() {
        ErrorKind::NotFound => format!("File {} does not exist", path),
        ErrorKind::PermissionDenied => format!("Failed to read {}: permission denied", path),
        _ => format!("Failed to open {}: {}", path, err),
      });
    }
  }
}
