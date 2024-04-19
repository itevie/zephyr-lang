use std::path::PathBuf;

use crate::{
  lexer, parser,
  runtime::interpreter::{self, Interpreter},
};

pub fn basic_run(input: String, file_name: String, dir: PathBuf) {
  let builder = std::thread::Builder::new()
    .name("zephyr_runner".into())
    .stack_size(crate::ARGS.stack_size);
  crate::verbose(
    &format!(
      "Running file {} with {}b stack",
      file_name.clone(),
      crate::ARGS.stack_size
    ),
    "basic_run",
  );

  let handler = builder
    .spawn(move || {
      let mut interpreter = Interpreter::new(dir.display().to_string());
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
    })
    .unwrap();

  handler.join().unwrap();

  // Check if it should split out outputs
  if crate::ARGS.node_evaluation_times {
    let data = interpreter::NODE_EVALUATION_TIMES.lock().unwrap().clone();
    let mut vec_data: Vec<(String, f64)> = vec![];

    for (key, value) in data.iter() {
      let mut total = 0;
      for i in value.clone() {
        total += i;
      }
      vec_data.push((key.clone(), total as f64 / value.len() as f64));
    }

    vec_data.sort_by(|b, a| a.1.partial_cmp(&b.1).unwrap());

    println!(" ----- NODE EVALUATION TIMES ----- ");
    for (key, value) in vec_data {
      println!("{}: {:.3}ms", key, value);
    }
  }
}
