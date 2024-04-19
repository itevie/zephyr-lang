use structopt::StructOpt;

// ----- Base CLI options -----
#[derive(StructOpt, Debug, Clone)]
pub struct Args {
  // ----- Global Flags -----
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
    long,
    help = "Whether or not to display a list of ast nodes that took the longest to complete (only displayed with run verb)"
  )]
  pub node_evaluation_times: bool,

  #[structopt(
    long,
    help = "Whether or not to display a list of Zephyr functions that took the longest to complete (only displayed with run verb)"
  )]
  pub function_evaluation_times: bool,

  #[structopt(
    long,
    help = "Whether or not to skip times that took 0ms in node or function evaluation times (only displayed with run verb)"
  )]
  pub node_evaluation_skip_zeros: bool,

  #[structopt(
    long = "stack-size",
    value_name = "STACK_SIZE",
    help = "The maximum stack size that the interpreter can use in bytes",
    default_value = "33554432"
  )]
  pub stack_size: usize,

  // ----- Subcommands -----
  #[structopt(subcommand)]
  pub subcommand: Option<Subcommands>,
}

// ----- List of subcommands -----
#[derive(Debug, StructOpt, Clone)]
pub enum Subcommands {
  #[structopt(about = "Generate a new Zephyr project")]
  New(NewPackage),
  #[structopt(about = "Test a directory of Zephyr test files")]
  Test(TestPackage),
  #[structopt(about = "Execute a zephyr file")]
  Run(RunFile),
  #[structopt(about = "Minimise a Zephyr file (UNSTABLE)")]
  Minimise(MinimiseFile),
  #[structopt(about = "Bundle a Zephyr project into one file (UNSTABLE)")]
  Bundle(BundleFile),
  #[structopt(about = "Go into REPL mode")]
  Repl(Repl),
}

#[derive(Debug, StructOpt, Clone)]
pub struct Repl {
  #[structopt(
    long = "repl-time",
    help = "Whether or not REPL mode should display how long an operation took"
  )]
  pub repl_time: bool,
}

#[derive(Debug, StructOpt, Clone)]
pub struct MinimiseFile {
  #[structopt(value_name = "PATH", empty_values = false)]
  pub file: String,

  #[structopt(value_name = "OUT", empty_values = false)]
  pub out: String,
}

#[derive(Debug, StructOpt, Clone)]
pub struct BundleFile {
  #[structopt(value_name = "PATH", empty_values = false)]
  pub file: String,

  #[structopt(value_name = "OUT", empty_values = false)]
  pub out: String,

  #[structopt(
    long = "executable",
    short = "e",
    help = "Whether or not to bundle the file into an executable"
  )]
  pub exe: bool,

  #[structopt(long = "target", short = "t", help = "Rust build target")]
  pub target: Option<String>,
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
  pub args: Vec<String>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct NewPackage {
  #[structopt(value_name = "PACKAGE-NAME", empty_values = false)]
  pub name_pos: String,
}

#[derive(Debug, StructOpt, Clone)]
pub struct TestPackage {
  #[structopt(value_name = "PATH-NAME", empty_values = false)]
  pub name_pos: String,

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
