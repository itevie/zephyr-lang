use std::path::PathBuf;

use crate::{errors::ZephyrError, lexer::location::Location};

// Path rules:
// Starts with "/" - absolute
// Starts with "." - Relative
// Starts with "pkg:" - Looks in package folder

pub fn resolve(directory: PathBuf, new: &str) -> Result<PathBuf, ZephyrError> {
  // First check if new is a package (starts with pkg:)
  if new.starts_with("pkg:") {
    // Get the package directory
    let mut package_folder = resolve_package_folder(directory.clone())?;

    // Check if it is only importing the package and not a subfile or something
    if new.contains('/') {
      package_folder.push(new.replace("pkg:", ""));
      return Ok(package_folder.clone());
    } else {
      // Read the package
      package_folder.push(new.replace("pkg:", ""));
      let package = crate::package_manager::load_package(package_folder.clone())?;
      package_folder.push(package.entry_point);
      return Ok(package_folder.clone());
    }
  }

  let mut resolved = directory.clone();
  resolved.push(new);

  Ok(resolved.canonicalize().unwrap())
}

pub fn resolve_package_folder(directory: PathBuf) -> Result<PathBuf, ZephyrError> {
  let result: PathBuf;
  let mut current = directory.clone();

  loop {
    // Construct path
    let mut dir = current.clone();
    dir.push("zephyr_packages");

    // Check if it exists
    if dir.exists() {
      result = dir.clone();
      break;
    } else {
      // Check if parent
      match current.parent() {
        Some(d) => current = d.to_path_buf(),
        None => {
          return Err(ZephyrError::runtime(
            "Cannot find a valid directory to look for packages".to_string(),
            Location::no_location(),
          ))
        }
      };
    }
  }

  Ok(result.canonicalize().unwrap())
}
