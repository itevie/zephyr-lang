use crate::lexer;

pub fn bundle(input: String, file_name: String, _out_file: String) {
  // Get the tokens
  let result = match lexer::lexer::lex(input, file_name.clone()) {
    Ok(val) => val,
    Err(err) => {
      println!("{}", err.visualise(false));
      return;
    }
  };

  println!("Found {} tokens", result.len());
}
