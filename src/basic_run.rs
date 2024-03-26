use std::path::PathBuf;

use crate::{lexer, parser, runtime::interpreter::Interpreter};

pub fn basic_run(input: String, file_name: String, dir: PathBuf) {
  let builder = std::thread::Builder::new()
    .name("reductor".into())
    .stack_size(32 * 1024 * 1024);
  
  let handler = builder.spawn(move || {let mut interpreter = Interpreter::new(dir.display().to_string());
    let result = match lexer::lexer::lex(input, file_name) {
      Ok(val) => val,
      Err(err) => {
        return crate::die(err.visualise(false));
      }
    };
  
    let mut parser = parser::parser::Parser::new(result);
    let ast = match parser.produce_ast() {
      Ok(val) => val,
      Err(err) => {
        return crate::die(err.visualise(false));
      }
    };
  
    let value = interpreter.evaluate(parser::nodes::Expression::Program(ast));
    match value {
      Err(err) => crate::die(err.visualise(false)),
      Ok(_) => (),
    }
  }).unwrap();

  handler.join().unwrap();
}
