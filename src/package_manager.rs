use crate::{cli::NewPackage, errors::ZephyrError, lexer::location::Location};
use serde::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Deserialize, Debug)]
pub struct PackageDependency {
  pub name: String,
  pub version: String,
}

#[derive(Deserialize, Debug)]
pub struct Package {
  #[serde(default)]
  pub name: String,
  #[serde(default)]
  pub version: String,
  #[serde(default)]
  pub author: String,
  #[serde(default)]
  pub description: String,
  #[serde(default)]
  pub entry_point: String,

  #[serde(default)]
  pub dependencies: Vec<PackageDependency>,
}

pub fn new(options: NewPackage, directory: PathBuf) {
  crate::debug(
    &format!("Creating package {}", options.name_pos),
    "package-manager",
  );

  // Construct path
  let mut path = directory;
  path.push(options.name_pos);

  // Check if folder already exists
  if path.exists() {
    crate::util::die(format!(
      "The folder {} already exists!",
      path.clone().display()
    ));
    return;
  }

  // Create folder
  match fs::create_dir(path.clone()) {
    Ok(_) => (),
    Err(err) => {
      crate::util::die(format!(
        "Failed to create directory {}: {}",
        path.display(),
        err
      ));
      return;
    }
  }

  crate::debug(
    &format!("Folder created {}", path.display()),
    "package-manager",
  );

  // Create index.zr
  let mut index_file = path.clone();
  index_file.push("index.zr");
  match fs::write(index_file.clone(), "Console.write_line(\"Hello, World!\");") {
    Ok(_) => (),
    Err(err) => {
      crate::util::die(format!(
        "Failed to create index.zr {}: {}",
        index_file.display(),
        err
      ));
      return;
    }
  }

  // Create package.toml
  let mut package_file = path.clone();
  package_file.push(crate::PACKAGE_FILE_NAME);
  match fs::write(package_file.clone(), "entry_point = \"index.zr\"") {
    Ok(_) => (),
    Err(err) => {
      crate::util::die(format!(
        "Failed to create {} {}: {}",
        crate::PACKAGE_FILE_NAME,
        package_file.display(),
        err
      ));
      return;
    }
  }

  // Done
  println!("Successfully created project!");
}

pub fn load_package(package_directory: PathBuf) -> Result<Package, ZephyrError> {
  // Try to locate package.toml
  let mut dir = package_directory.clone();
  dir.push(crate::PACKAGE_FILE_NAME);

  // Check if it exists
  if dir.exists() == false {
    return Err(ZephyrError::runtime(
      format!(
        "Cannot find the {} in {}",
        crate::PACKAGE_FILE_NAME,
        package_directory.display().to_string()
      ),
      Location::no_location(),
    ));
  }

  // Read it
  let data = crate::util::fs::read_file(dir.canonicalize().unwrap().display().to_string());
  let package = parse_package_file(package_directory, data)?;
  Ok(package)
}

pub fn parse_package_file(base_directroy: PathBuf, data: String) -> Result<Package, ZephyrError> {
  let data: Package = match toml::from_str(&data) {
    Ok(data) => data,
    Err(err) => {
      return Err(ZephyrError::runtime(
        format!(
          "Failed to parse {:?} ({} file): {:?}",
          base_directroy,
          crate::PACKAGE_FILE_NAME,
          err
        ),
        Location::no_location(),
      ))
    }
  };

  Ok(data)
}
