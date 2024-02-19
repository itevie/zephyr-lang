use crate::lexer;

pub fn bundle(input: String, file_name: String) -> String {
  // Get the tokens
  let _result = match lexer::lexer::lex(input.clone(), file_name.clone()) {
    Ok(val) => val,
    Err(err) => {
      println!("{}", err.visualise(false));
      panic!();
    }
  };

  input.clone()
}
