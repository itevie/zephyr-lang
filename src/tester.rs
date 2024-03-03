use std::{fs, path::PathBuf};

use crate::TestPackage;

pub fn test(options: TestPackage) -> () {
  let mut files: Vec<PathBuf> = vec![];
  let r = regex::Regex::new(&format!("({})$", options.pattern.replace("*", ".*"))).unwrap();
  let directory = PathBuf::from(options.name_pos);

  // Gather files
  if directory.is_file() {
    files.push(directory.clone());
  } else {
    let paths = fs::read_dir(directory.clone()).unwrap();
    for i in paths {
      files.push(i.unwrap().path());
    }
  }

  files = files
    .iter()
    .filter(|z| r.is_match(&z.display().to_string()))
    .map(|x| x.clone())
    .collect();

  println!("Found {} files to test...", files.len());

  for file in files {
    println!("  Testing {}...", file.display().to_string());
    let contents = std::fs::read_to_string(file.clone()).unwrap();
    crate::basic_run::basic_run(contents, file.display().to_string(), directory.clone());
  }

  println!("All tests succeeded");
}
