pub fn new(o: crate::Subcommands) {
  let options = match o { crate::Subcommands::New(op) => op, _ => unreachable!() };
  println!("{:?}", options);
}