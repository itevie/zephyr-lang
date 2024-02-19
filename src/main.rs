use std::{
  fs::{self, File},
  io::{ErrorKind, Write},
  sync::{Arc, Mutex},
};

use once_cell::sync::Lazy;
use runtime::memory::Memory;
use structopt::StructOpt;

#[path = "./repl.rs"]
mod repl;

#[path = "./bundler.rs"]
mod bundler;

#[path = "./mini.rs"]
mod mini;

#[path = "./basic_run.rs"]
mod basic_run;

//use std::io::Write;

pub mod errors;
pub mod lexer;
pub mod parser;
pub mod runtime;
pub mod util;

#[derive(StructOpt, Debug, Clone)]
pub struct Args {
  #[structopt(
    long = "file",
    empty_values = false,
    short = "f",
    help = "The index file to run",
    value_name = "PATH_FLAG"
  )]
  pub file_flag: Option<String>,

  #[structopt(
    value_name = "PATH",
    empty_values = false,
    conflicts_with = "file-flag"
  )]
  pub file_pos: Option<String>,

  #[structopt(
    long = "directory",
    short = "d",
    empty_values = false,
    help = "The directory to run the project in",
    value_name = "WORKING_DIRECTORY"
  )]
  pub directory: Option<String>,

  #[structopt(long, help = "Whether or not to log special debug logs")]
  pub debug: bool,

  #[structopt(
    long = "out",
    short = "o",
    value_name = "OUT_FILE",
    help = "The file to write the output of actions such as --bundle or --minimise"
  )]
  pub out_file: Option<String>,

  #[structopt(long, help = "Bundle Zephyr project into one file.")]
  pub bundle: bool,

  #[structopt(
    long = "bundle-executable",
    help = "Bundle Zephyr project into one executable file."
  )]
  pub bundle_executable: bool,

  #[structopt(long, help = "Minimise a Zephyr file.")]
  pub minimise: bool,
}

static MEMORY: Lazy<Arc<Mutex<Memory>>> = Lazy::new(|| Arc::from(Mutex::from(Memory::new())));
static ARGS: Lazy<Args> = Lazy::new(|| Args::from_args());

pub fn debug(contents: &str, what: &str) {
  if ARGS.debug {
    println!("[DEBUG:{}]: {}", what, contents);
  }
}

fn main() {
  // This will be present if --bundle-executable is ran
  let bundled_data = "";
  let args = ARGS.clone();

  // Get the directory to run in
  let dir = if let Some(directory) = args.directory.clone() {
    let mut path = std::path::PathBuf::new();
    path.push(&directory);
    path
  } else {
    std::env::current_dir().unwrap()
  };

  // Check if bundled_data
  if bundled_data != "" {
    basic_run::basic_run(
      String::from(bundled_data),
      std::env::current_exe().unwrap().display().to_string(),
      std::env::current_dir().unwrap(),
    );
    return ();
  }

  // Check if the provided dir exists
  if dir.exists() == false {
    return die(format!(
      "The directory ({}) provided with --directory does not exist",
      dir.display()
    ));
  }

  debug(
    &format!(
      "The current directory is set to: {}",
      dir.display().to_string()
    ),
    "main",
  );

  // Check if should run in repl mode
  if matches!(args.file_flag, None) && matches!(args.file_pos, None) {
    repl::repl(args, dir.display().to_string());
  } else {
    // Collect the file
    let file_name = &if let Some(f) = args.file_flag {
      f
    } else if let Some(f) = args.file_pos {
      f
    } else {
      panic!()
    };

    let mut input = match std::fs::read_to_string(file_name) {
      Ok(ok) => ok,
      Err(err) => {
        return die(match err.kind() {
          ErrorKind::NotFound => format!("File {} does not exist", file_name),
          ErrorKind::PermissionDenied => format!("Failed to read {}: permission denied", file_name),
          _ => format!("Failed to open {}: {}", file_name, err),
        })
      }
    };

    let proper_file_name = fs::canonicalize(file_name).unwrap();

    // Check if should have out file
    let should_out;
    if args.bundle || args.minimise || args.bundle_executable {
      if !matches!(args.out_file, Some(_)) {
        return die(
          "The --bundle or --minimise flags were used, but no --out was provided".to_string(),
        );
      }

      should_out = true;
    } else {
      should_out = false;
    }
    // Check if should bundle executable
    if args.bundle_executable {
      bundler::bundle_executable(
        input,
        proper_file_name.display().to_string(),
        ARGS.clone().out_file.unwrap(),
      );
      return ();
    }

    // Check if it should bundle
    if args.bundle {
      input = bundler::bundle(input, proper_file_name.display().to_string());
    }

    // Check if it should minimise
    if args.minimise {
      input = mini::minimise(input, proper_file_name.display().to_string());
    }

    // Check if it should output
    if should_out {
      // Write it
      let mut f = File::create(ARGS.clone().out_file.unwrap()).unwrap();
      f.write_all(input.as_bytes()).unwrap();
      return ();
    }

    basic_run::basic_run(input, file_name.clone(), dir);
  }
}

fn die(err: String) -> () {
  println!("Fatal Error: {}", err);
}
