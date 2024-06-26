use std::io::Write;

use crate::{lexer, parser, runtime::interpreter::Interpreter};

pub fn repl(options: crate::cli::Repl, directory: String) {
  let mut interpreter = Interpreter::new(directory.clone());
  let scope = interpreter.scope;

  loop {
    std::mem::swap(&mut interpreter.scope, &mut scope.clone());

    // Get input
    print!("> ");
    std::io::stdout().flush().unwrap();
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);

    if input.starts_with(".mem") {
      println!("{:#?}", crate::MEMORY.clone());
      continue;
    }
    if input.starts_with(".flush") {
      interpreter = Interpreter::new(directory.clone());
      continue;
    }
    if input.starts_with(".exit") {
      std::process::exit(0);
    }

    // Check if input has a ; at the end
    if !input.ends_with(';') {
      input.push(';');
    }

    // Lex
    let lex_timer = std::time::Instant::now();
    let result = match lexer::lexer::lex(input, "<repl>".to_string()) {
      Ok(val) => val,
      Err(err) => {
        println!("{}", err.visualise(false));
        continue;
      }
    };
    let lex_elapsed = lex_timer.elapsed();

    // Parse
    let parser_timer = std::time::Instant::now();
    let mut parser = parser::parser::Parser::new(result);
    let ast = match parser.produce_ast(Some("<repl>".to_string())) {
      Ok(val) => val,
      Err(err) => {
        println!("{}", err.visualise(false));
        continue;
      }
    };
    let parse_elapsed = parser_timer.elapsed();

    // Runtime
    let runtime_time = std::time::Instant::now();
    let value = interpreter.evaluate(parser::nodes::Expression::Program(ast));
    let runtime_elapsed = runtime_time.elapsed();

    // Compute time
    let time = &*format!(
      "\x1b[90m ~ {}\x1b[0m",
      &*format!(
        "(lexer: {}μs, parser: {}μs, runtime: {}μs, total: {}μs)",
        lex_elapsed.as_micros(),
        parse_elapsed.as_micros(),
        runtime_elapsed.as_micros(),
        lex_elapsed.as_micros() + parse_elapsed.as_micros() + runtime_elapsed.as_micros()
      )
    );

    match value {
      Err(err) => println!("{}", err.visualise(false)),
      Ok(val) => println!(
        "{}{}",
        val,
        if options.repl_time {
          "\n".to_owned() + time
        } else {
          "".to_owned()
        }
      ),
    }
  }
}
