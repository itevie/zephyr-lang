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
      let result = match lexer::lexer::lex(input, file_name.clone()) {
        Ok(val) => val,
        Err(err) => {
          return crate::die(err.visualise(false));
        }
      };

      let mut parser = parser::parser::Parser::new(result);
      let ast = match parser.produce_ast(Some(file_name.clone())) {
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
  if crate::ARGS.node_evaluation_times || crate::ARGS.function_evaluation_times {
    let data = interpreter::NODE_EVALUATION_TIMES.lock().unwrap().clone();
    let mut vec_data: Vec<(String, f64, String)> = vec![];

    for (key, value) in data.iter() {
      let mut total = 0;
      for i in value.clone() {
        total += i;
      }

      let time = total as f64 / value.len() as f64;

      // Check if it is a function
      if key.starts_with("#f#") {
        let parts: Vec<&str> = key.split("##").collect();
        vec_data.push((parts[0].replace("#f#", "func "), time, parts[1].to_string()));
      } else {
        vec_data.push((format!("node {}", key.clone()), time, "".to_string()));
      }
    }

    vec_data.sort_by(|b, a| a.1.partial_cmp(&b.1).unwrap());

    // Get biggest key
    let mut biggest_key = 0;
    for key in data.keys() {
      if key.len() > biggest_key {
        biggest_key = key.len();
      }
    }

    println!(" ----- NODE EVALUATION TIMES ----- ");
    println!("{0: <50} | {1: <10} | {2: <50} ", "What", "Time", "Context");
    for (key, value, context) in vec_data {
      if value == 0.0f64 && crate::ARGS.node_evaluation_skip_zeros {
        continue;
      }

      //let is_danger_time = value > 500f64;
      let ms = format!("{:.3}ms", value);

      let text = format!("{0: <50} | {1: <10} | {2: <50}", key, ms, context);

      /*if is_danger_time {
        println!(
          "{} {} {}",
          util::colors::fg_red(),
          text,
          util::colors::reset()
        );
      } else {*/
      println!("{}", text);
      //}
    }
  }
}
