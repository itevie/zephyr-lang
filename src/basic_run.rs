use std::path::PathBuf;

use crate::{lexer, parser, runtime::interpreter::Interpreter};

pub fn basic_run(input: String, file_name: String, dir: PathBuf) -> () {
  let mut interpreter = Interpreter::new(dir.display().to_string());

  let result = match lexer::lexer::lex(input, file_name.clone()) {
    Ok(val) => val,
    Err(err) => {
      println!("{}", err.visualise(false));
      return;
    }
  };

  let mut parser = parser::parser::Parser::new(result);
  let ast = match parser.produce_ast() {
    Ok(val) => val,
    Err(err) => {
      println!("{}", err.visualise(false));
      return;
    }
  };

  let value = interpreter.evaluate(parser::nodes::Expression::Program(ast));
  match value {
    Err(err) => println!("{}", err.visualise(false)),
    Ok(_) => return,
  }
}
