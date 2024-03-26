use std::{
  collections::HashMap,
  fs::{self, File},
  io::{ErrorKind, Write},
  path::PathBuf,
  sync::{Arc, Mutex},
  time::Duration,
};

use basic_run::basic_run;
use once_cell::sync::Lazy;
use runtime::memory::Memory;
use std::sync::atomic::{AtomicUsize, Ordering};
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
    long = "args",
    short = "a",
    default_value = "",
    value_name = "ZEPHYR_ARGS",
    help = "The arguments to pass along to the Zephyr program"
  )]
  pub args: Vec<String>,

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

  #[structopt(long, help = "Whether or not to log special verbose logs")]
  pub verbose: bool,

  #[structopt(
    long = "repl-time",
    help = "Whether or not REPL mode should display how long an operation took"
  )]
  pub repl_time: bool,

  #[structopt(
    long = "stack-size",
    value_name = "STACK_SIZE",
    help = "The maximum stack size that the interpreter can use in bytes",
    default_value = "33554432"
  )]
  pub stack_size: usize,

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

  // ----- Subcommands -----
  #[structopt(subcommand)]
  pub subcommand: Option<Subcommands>,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Subcommands {
  New(NewPackage),
  Test(TestPackage),
  Run(RunFile)
}

#[derive(Debug, StructOpt, Clone)]
pub struct RunFile {
  #[structopt(
    value_name = "PATH",
    empty_values = false,
    conflicts_with = "file-flag"
  )]
  pub file_pos: String,

  #[structopt(raw(true))]
  pub args: Vec<String>
}

#[derive(Debug, StructOpt, Clone)]
pub struct NewPackage {
  #[structopt(value_name = "PACKAGE-NAME", empty_values = false)]
  name_pos: String,
}

#[derive(Debug, StructOpt, Clone)]
pub struct TestPackage {
  #[structopt(value_name = "PATH-NAME", empty_values = false)]
  name_pos: String,

  #[structopt(
    long = "pattern",
    empty_values = false,
    short = "p",
    help = "The pattern to match with files",
    value_name = "PATTERN",
    default_value = "*.test.zr"
  )]
  pub pattern: String,
}

static MEMORY: Lazy<Arc<Mutex<Memory>>> = Lazy::new(|| Arc::from(Mutex::from(Memory::new())));
static SCOPES: Lazy<Arc<Mutex<HashMap<u128, Arc<Mutex<runtime::scope::Scope>>>>>> =
  Lazy::new(|| Arc::from(Mutex::from(HashMap::new())));
static ARGS: Lazy<Args> = Lazy::new(Args::from_args);
static ZEPHYR_ARGS: Lazy<Arc<RwLock<Vec<String>>>> = Lazy::new(|| Arc::from(RwLock::from(vec![])));

static GLOBAL_THREAD_COUNT: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(0));

pub fn debug(contents: &str, what: &str) {
  if ARGS.debug || ARGS.verbose {
    println!("[DEBUG:{}]: {}", what, contents);
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
      Subcommands::New(new) => package_manager::new(new, dir),
      Subcommands::Test(new) => tester::test(new),
      Subcommands::Run(run) => {
        let input = match std::fs::read_to_string(run.file_pos.clone()) {
          Ok(ok) => ok,
          Err(err) => {
            return die(match err.kind() {
              ErrorKind::NotFound => format!("File {} does not exist", run.file_pos),
              ErrorKind::PermissionDenied => format!("Failed to read {}: permission denied", run.file_pos),
              _ => format!("Failed to open {}: {}", run.file_pos, err),
            })
          }
        };

        basic_run::basic_run(input, run.file_pos.clone(), dir);
      }
    }

    return;
  }

  // By now it should just be special things or REPL mode
  if matches!(args.file_flag, None) {
    repl::repl(args, dir.display().to_string());
  } else {
    let file_name = args.file_flag.unwrap().clone();

    // Now the user just wants to do special things ig
    let mut input = match std::fs::read_to_string(file_name.clone()) {
      Ok(ok) => ok,
      Err(err) => {
        return die(match err.kind() {
          ErrorKind::NotFound => format!("File {} does not exist", file_name),
          ErrorKind::PermissionDenied => format!("Failed to read {}: permission denied", file_name),
          _ => format!("Failed to open {}: {}", file_name, err),
        })
      }
    };

    let proper_file_name = fs::canonicalize(file_name.clone()).unwrap();

    // Check if should have out file
    let should_out = if args.bundle || args.minimise || args.bundle_executable {
      if !matches!(args.out_file, Some(_)) {
        return die(
          "The --bundle or --minimise flags were used, but no --out was provided".to_string(),
        );
      }
      true
    } else {
      false
    };

    // Check if should bundle executable
    if args.bundle_executable {
      bundler::bundle_executable(
        input,
        proper_file_name.display().to_string(),
        ARGS.clone().out_file.unwrap(),
      );
      return;
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
      return;
    }

    basic_run::basic_run(input, file_name.clone(), dir);
  }

  while GLOBAL_THREAD_COUNT.load(Ordering::SeqCst) != 0 {
    std::thread::sleep(Duration::from_millis(1));
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
