use crate::lexer::{
  self,
  token::{Token, TokenType},
};

pub fn minimise(input: String, file_name: String) -> String {
  // Get the tokens
  let result = match lexer::lexer::lex(input, file_name) {
    Ok(val) => val,
    Err(err) => {
      println!("{}", err.visualise(false));
      panic!();
    }
  };

  compress_tokens(result)
}

pub fn compress_tokens(tokens: Vec<Token>) -> String {
  let mut result_string = String::new();

  for i in tokens {
    let value = match i.token_type {
      TokenType::String => format!("\"{}\"", i.value).replace('\\', "\\\\"),
      _ => i.value,
    };
    let needs_space_after = match i.token_type {
      TokenType::Let => true,
      TokenType::In => true,
      TokenType::If => true,
      TokenType::Is => true,
      TokenType::Catch => true,
      TokenType::Not => true,
      TokenType::While => true,
      TokenType::Throw => true,
      TokenType::Until => true,
      TokenType::Loop => true,
      TokenType::Import => true,
      TokenType::Enum => true,
      TokenType::Export => true,
      TokenType::Function => true,
      TokenType::Return => true,
      TokenType::Else => true,
      TokenType::For => true,
      TokenType::Identifier => true,
      _ => false,
    };
    result_string.push_str(&(value + if needs_space_after { " " } else { "" }));
  }

  result_string
}
