use std::{fs, path::PathBuf};

use crate::cli::TestPackage;

pub fn test(options: TestPackage) {
  let mut files: Vec<PathBuf> = vec![];
  let r = regex::Regex::new(&format!("({})$", options.pattern.replace('*', ".*"))).unwrap();
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

  files.retain(|z| r.is_match(&z.display().to_string()));

  println!("Found {} files to test...", files.len());

  for file in files {
    let start_time = std::time::Instant::now();

    print!("  Testing {}...", file.display());

    let contents = std::fs::read_to_string(file.clone()).unwrap();
    crate::executors::basic_run::basic_run(
      contents,
      file.canonicalize().unwrap().display().to_string(),
      directory.clone(),
    );

    let end_time = start_time.elapsed();
    println!(" {}ms", end_time.as_millis());
  }

  println!("All tests succeeded");
}
