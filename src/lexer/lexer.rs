use crate::{
  errors::{lexer_error, ZephyrError},
  lexer::token::{AdditiveTokenType, MultiplicativeTokenType, UnaryOperator},
};
use lazy_static::lazy_static;
use once_cell::sync::Lazy;
use std::collections::HashMap;

use super::{
  location::Location,
  token::{ComparisonTokenType, LogicalTokenType, Token, TokenType},
};

lazy_static! {
  static ref OPERATORS: HashMap<&'static str, &'static TokenType> = {
    let mut operators: HashMap<&'static str, &'static TokenType> = HashMap::new();

    macro_rules! tok {
      ($name:literal, $t:expr) => {
        operators.insert(&$name, &$t);
      }
    }

    // Arithmetic Operators
    tok!("+", TokenType::AdditiveOperator(AdditiveTokenType::Plus));
    tok!("-", TokenType::AdditiveOperator(AdditiveTokenType::Minus));
    tok!("*", TokenType::MultiplicativeOperator(MultiplicativeTokenType::Multiply));
    tok!("/", TokenType::MultiplicativeOperator(MultiplicativeTokenType::Divide));
    tok!("//", TokenType::MultiplicativeOperator(MultiplicativeTokenType::IntegerDivide));
    tok!("%", TokenType::MultiplicativeOperator(MultiplicativeTokenType::Modulo));

    // Comparison operators
    tok!("==", TokenType::ComparisonTokenType(ComparisonTokenType::Equals));
    tok!("!=", TokenType::ComparisonTokenType(ComparisonTokenType::NotEquals));
    tok!(">", TokenType::ComparisonTokenType(ComparisonTokenType::GreaterThan));
    tok!(">=", TokenType::ComparisonTokenType(ComparisonTokenType::GreaterThanOrEquals));
    tok!("<", TokenType::ComparisonTokenType(ComparisonTokenType::LessThan));
    tok!("<=", TokenType::ComparisonTokenType(ComparisonTokenType::LessThanOrEquals));

    // Assignment operators
    tok!("=", TokenType::NormalAssignmentOperator);

    // Logical operators
    tok!("&&", TokenType::LogicalOperator(LogicalTokenType::And));
    tok!("||", TokenType::LogicalOperator(LogicalTokenType::Or));

    // Unary Operator
    tok!("!", TokenType::UnaryOperator(UnaryOperator::Not));
    tok!("&", TokenType::UnaryOperator(UnaryOperator::Reference));
    tok!("~", TokenType::UnaryOperator(UnaryOperator::Dereference));

    // Keywords
    tok!("is", TokenType::Is);
    tok!("in", TokenType::In);
    tok!("let", TokenType::Let);
    tok!("if", TokenType::If);
    tok!("else", TokenType::Else);
    tok!("break", TokenType::Break);
    tok!("continue", TokenType::Continue);
    tok!("for", TokenType::For);
    tok!("while", TokenType::While);
    tok!("until", TokenType::Until);
    tok!("loop", TokenType::Loop);
    tok!("where", TokenType::Where);
    tok!("typeof", TokenType::Typeof);
    tok!("func", TokenType::Function);
    tok!("pure", TokenType::Pure);

    // Basic Syntax
    tok!("(", TokenType::OpenParen);
    tok!(")", TokenType::CloseParen);
    tok!("{", TokenType::OpenBrace);
    tok!("}", TokenType::CloseBrace);
    tok!("[", TokenType::OpenSquare);
    tok!("]", TokenType::CloseSquare);
    tok!(",", TokenType::Comma);
    tok!(".", TokenType::Dot);
    tok!(":", TokenType::Colon);
    tok!("?", TokenType::QuestionMark);
    tok!("#", TokenType::BlockPrefix);
    tok!(";", TokenType::Semicolon);

    operators
  };
}

static mut LOCATION_CONTENTS: Lazy<HashMap<u128, String>> = Lazy::new(|| {
  let mut hash = HashMap::new();
  hash.insert(0 as u128, "No Location Contents".to_string());
  hash
});
static mut CURRENT_CONTENTS: u128 = 1;

pub fn get_token(_t: &TokenType) -> String {
  OPERATORS
    .iter()
    .find_map(|(key, &val)| if val == _t { Some(key) } else { None })
    .unwrap()
    .to_string()
}

pub fn get_location_contents(id: u128) -> String {
  unsafe { String::from(LOCATION_CONTENTS.get(&id).unwrap()) }
}

pub fn lex(contents: String) -> Result<Vec<Token>, ZephyrError> {
  let id = unsafe { CURRENT_CONTENTS };
  unsafe {
    CURRENT_CONTENTS += 1;
    LOCATION_CONTENTS.insert(id, contents.clone());
  };

  let mut operator_keys: Vec<&&str> = OPERATORS.keys().clone().collect();
  operator_keys.sort_by(|a, b| b.len().cmp(&a.len()));

  let string_only_operators: Vec<&&str> = OPERATORS
    .keys()
    .filter(|x| x.chars().all(char::is_alphabetic))
    .collect();
  //for i in &operator_keys { println!("{}", i); }

  let mut chars: Vec<char> = contents.chars().collect();
  let mut tokens: Vec<Token> = vec![];

  let mut current_char: u32 = 0;
  let mut current_line: u32 = 0;

  while chars.len() != 0 {
    let mut location = Location {
      char_start: current_char,
      char_end: current_char,
      line: current_line,
      location_contents: id,
    };

    let mut token_value: Option<String> = None;
    let mut token_type: Option<TokenType> = None;

    let mut eat = |chars: &mut Vec<char>| -> String {
      let char = chars[0];
      chars.remove(0);
      current_char += 1;

      char.to_string()
    };

    let mut set_token = |value, t| {
      token_value = Some(value);
      token_type = Some(t);
    };

    // Check for whitespace
    if chars[0] == ' ' || chars[0] == '\t' || chars[0] == '\r' {
      eat(&mut chars);
      continue;
    }
    // Check for newline
    else if chars[0] == '\n' {
      eat(&mut chars);
      current_char = 0;
      current_line += 1;
      continue;
    }
    // Check for // comment
    else if chars[0] == '/' && chars.len() >= 2 && chars[1] == '/' {
      // Repeat until \n or eof
      while chars.len() > 0 && chars[0] != '\n' {
        eat(&mut chars);
      }
      continue;
    }
    // Check for /* comment
    else if chars[0] == '/' && chars.len() >= 2 && chars[1] == '*' {
      let mut closed = false;

      while chars.len() > 0 {
        if chars.len() >= 2 && chars[0] == '*' && chars[1] == '/' {
          eat(&mut chars);
          eat(&mut chars);
          closed = true;
          break;
        }
        eat(&mut chars);
      }
      // Check if it was closed
      if !closed {
        return Err(ZephyrError::lexer(
          "Multi-line comment not closed".to_string(),
          Location::no_location(),
        ));
      }
      continue;
    }
    // Check for string literal
    else if chars[0] == '"' {
      // Remove quote mark
      eat(&mut chars);

      let mut value = String::from("");

      // Repeat until end of quote, found new line or EOF
      while chars[0] != '"' && chars[0] != '\n' && chars.len() > 0 {
        value.push_str(&eat(&mut chars));
      }

      // Make sure current character is a "
      if chars[0] != '"' {
        return Err(lexer_error!("Unexpected ending of string".to_string()));
      }

      eat(&mut chars);

      set_token(value, TokenType::String);
    }
    // Check if current char is a number
    else if chars[0].is_numeric() {
      let mut value: String = eat(&mut chars);

      // Loop until not a number
      while chars.len() > 0 && chars[0].is_numeric() {
        value.push_str(&eat(&mut chars));
      }

      // Set token
      set_token(value, TokenType::Number);
    }
    // Check if the current char is alpha
    else if chars[0].is_alphabetic() || chars[0] == '_' {
      let mut value: String = eat(&mut chars);

      while chars.len() > 0 && (chars[0].is_alphanumeric() || chars[0] == '_') {
        value.push_str(&eat(&mut chars));
      }

      // Check for ?
      if chars.len() > 0 && chars[0] == '?' {
        value.push_str(&eat(&mut chars));
        set_token(value, TokenType::PredicateIdentifier);
      } else {
        // Check if it is an operator
        if string_only_operators.contains(&&&*value) {
          let oper = *OPERATORS.get(&*value).unwrap();
          set_token(value, *oper);
        }
        // Set token
        else {
          set_token(value, TokenType::Identifier)
        };
      };
    }
    // No token was found
    else {
      // Check for symbol operators
      let mut found: bool = false;
      for key in &operator_keys {
        let operator_type = OPERATORS[*key];

        // Check if length is ok
        if chars.len() < key.len() {
          continue;
        };

        // Lookahead
        let lookahead: String = chars[0..key.len()].to_vec().iter().collect();

        // Check if it is same
        if lookahead == **key {
          found = true;
          set_token(lookahead, *operator_type);
          for _ in 0..key.len() {
            eat(&mut chars);
          }
          break;
        }
      }

      // Check if it was found
      if !found {
        return Err(lexer_error!(format!("Unexpected token: {}", chars[0])));
      }
    }

    // Update the location
    location.char_end = current_char;

    // Add the token to the list
    tokens.push(Token {
      value: token_value.unwrap_or("".to_string()),
      token_type: token_type.unwrap(),
      location,
    });
  }

  tokens.push(Token {
    value: "".to_string(),
    token_type: TokenType::EOF,
    location: Location {
      location_contents: id,
      char_start: current_char,
      char_end: current_char,
      line: current_line,
    },
  });

  Ok(tokens)
}
