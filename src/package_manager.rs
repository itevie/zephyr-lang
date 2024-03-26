use std::{fs, path::PathBuf};

use crate::NewPackage;

pub struct _Package {
  pub name: String,
  pub version: String,
  pub author: String,
  pub description: String,
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
    crate::die(format!(
      "The folder {} already exists!",
      path.clone().display()
    ));
    return;
  }

  // Create folder
  match fs::create_dir(path.clone()) {
    Ok(_) => (),
    Err(err) => {
      crate::die(format!(
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
      crate::die(format!(
        "Failed to create index.zr {}: {}",
        index_file.display(),
        err
      ));
      return;
    }
  }

  // Done
  println!("Successfully created project!");
}
