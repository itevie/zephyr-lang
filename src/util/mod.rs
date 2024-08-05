pub mod colors;
pub mod fs;
pub mod json;
pub mod path_resolver;

pub fn varient_eq<T>(a: &T, b: &T) -> bool {
  std::mem::discriminant(a) == std::mem::discriminant(b)
}

pub fn die(err: String) {
  println!(
    "{}Fatal Error: {}{}",
    colors::fg_red(),
    err,
    colors::reset()
  );
  std::process::exit(1);
}

pub fn debug(contents: &str, what: &str) {
  if crate::ARGS.debug || crate::ARGS.verbose {
    println!(
      "[DEBUG:{} THREAD: {:?}]: {}",
      what,
      std::thread::current().id(),
      contents
    );
  }
}

pub fn verbose(contents: &str, what: &str) {
  if crate::ARGS.verbose {
    println!("[VERBOSE:{}]: {}", what, contents);
  }
}
