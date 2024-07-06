use std::{
  collections::HashMap,
  fs,
  io::ErrorKind,
  path::PathBuf,
  sync::{Arc, Mutex, RwLock},
};

use once_cell::sync::Lazy;
use runtime::memory::Memory;
use std::sync::atomic::AtomicUsize;
use structopt::StructOpt;

#[path = "./repl.rs"]
mod repl;

#[path = "./bundler.rs"]
mod bundler;

#[path = "./tester.rs"]
mod tester;

#[path = "./mini.rs"]
mod mini;

#[path = "./basic_run.rs"]
mod basic_run;

#[path = "./package_manager.rs"]
mod package_manager;

#[path = "./cli.rs"]
pub mod cli;

//use std::io::Write;

pub mod errors;
pub mod lexer;
pub mod parser;
pub mod runtime;
pub mod util;

// Basic configs
static PACKAGE_FILE_NAME: &'static str = "package.toml";

// Other static items
static MEMORY: Lazy<Arc<Mutex<Memory>>> = Lazy::new(|| Arc::from(Mutex::from(Memory::new())));
static SCOPES: Lazy<Arc<Mutex<HashMap<u128, Arc<Mutex<runtime::scope::Scope>>>>>> =
  Lazy::new(|| Arc::from(Mutex::from(HashMap::new())));
static ARGS: Lazy<cli::Args> = Lazy::new(cli::Args::from_args);
static ZEPHYR_ARGS: Lazy<Arc<RwLock<Vec<String>>>> = Lazy::new(|| Arc::from(RwLock::from(vec![])));
static GLOBAL_THREAD_COUNT: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(0));

pub fn debug(contents: &str, what: &str) {
  if ARGS.debug || ARGS.verbose {
    println!(
      "[DEBUG:{} THREAD: {:?}]: {}",
      what,
      std::thread::current().id(),
      contents
    );
  }
}

pub fn verbose(contents: &str, what: &str) {
  if ARGS.verbose {
    println!("[VERBOSE:{}]: {}", what, contents);
  }
}

pub fn get_data_dir() -> PathBuf {
  let mut buf = PathBuf::from(directories::UserDirs::new().unwrap().home_dir());
  buf.push(".zephyr");
  buf
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
  if !bundled_data.is_empty() {
    basic_run::basic_run(
      String::from(bundled_data),
      std::env::current_exe().unwrap().display().to_string(),
      std::env::current_dir().unwrap(),
    );
    return;
  }

  // Check if the provided dir exists
  if !dir.exists() {
    return die(format!(
      "The directory ({}) provided with --directory does not exist",
      dir.display()
    ));
  }

  debug(
    &format!(
      "The current directory is set to: {}, app data dir is {}",
      dir.display(),
      get_data_dir().display(),
    ),
    "main",
  );

  // Check for subcommands
  if let Some(subcommand) = ARGS.clone().subcommand {
    match subcommand {
      cli::Subcommands::New(new) => package_manager::new(new, dir),
      cli::Subcommands::Test(new) => tester::test(new),
      cli::Subcommands::Repl(repl) => repl::repl(repl, ".".to_string()),
      cli::Subcommands::Run(run) => {
        *ZEPHYR_ARGS.write().unwrap() = run.args;
        let input = match std::fs::read_to_string(run.file_pos.clone()) {
          Ok(ok) => ok,
          Err(err) => {
            return die(match err.kind() {
              ErrorKind::NotFound => format!("File {} does not exist", run.file_pos),
              ErrorKind::PermissionDenied => {
                format!("Failed to read {}: permission denied", run.file_pos)
              }
              _ => format!("Failed to open {}: {}", run.file_pos, err),
            })
          }
        };

        basic_run::basic_run(input, run.file_pos.clone(), dir);
      }
      cli::Subcommands::Minimise(minimise) => {
        // Get the file contents and minimise it
        let input = read_file(minimise.file.clone());
        let output = mini::minimise(input, PathBuf::from(minimise.file).display().to_string());

        // Save it
        write_file(&minimise.out, output);
      }
      cli::Subcommands::Bundle(bundle) => {
        // Get file contents
        let input = read_file(bundle.file.clone());

        // Check if it is converting to an exe
        if bundle.exe {
          bundler::bundle_executable(
            input,
            PathBuf::from(bundle.file.clone()).display().to_string(),
            bundle.out.clone(),
            bundle.clone(),
          );

          return;
        }

        // Otherwise just bundle it normally
        let output = bundler::bundle(input, PathBuf::from(bundle.file).display().to_string());

        // Save it
        write_file(&bundle.out, output);
      }
    }

    return;
  }
}

fn read_file(path: String) -> String {
  match std::fs::read_to_string(path.clone()) {
    Ok(ok) => ok,
    Err(err) => {
      die(match err.kind() {
        ErrorKind::NotFound => format!("File {} does not exist", path),
        ErrorKind::PermissionDenied => format!("Failed to read {}: permission denied", path),
        _ => format!("Failed to open {}: {}", path, err),
      });
      return "".to_string();
    }
  }
}

fn write_file(path: &str, contents: String) -> () {
  match fs::write(path, contents) {
    Ok(_) => (),
    Err(err) => {
      die(match err.kind() {
        ErrorKind::NotFound => format!("File {} does not exist", path),
        ErrorKind::PermissionDenied => format!("Failed to read {}: permission denied", path),
        _ => format!("Failed to open {}: {}", path, err),
      });
    }
  }
}

fn die(err: String) {
  println!(
    "{}Fatal Error: {}{}",
    util::colors::fg_red(),
    err,
    util::colors::reset()
  );
  std::process::exit(1);
}
