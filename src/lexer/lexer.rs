use crate::{
  errors::{lexer_error, ZephyrError},
  lexer::token::{AdditiveTokenType, DualTokenType, MultiplicativeTokenType, UnaryOperator},
};
use lazy_static::lazy_static;
use once_cell::sync::Lazy;
use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};

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
    tok!("??", TokenType::MultiplicativeOperator(MultiplicativeTokenType::Coalesce));

    // Dual operators
    tok!("+=", TokenType::DualOperator(DualTokenType::Additive(AdditiveTokenType::Plus)));
    tok!("-=", TokenType::DualOperator(DualTokenType::Additive(AdditiveTokenType::Minus)));
    tok!("*=", TokenType::DualOperator(DualTokenType::Multiplicative(MultiplicativeTokenType::Multiply)));
    tok!("/=", TokenType::DualOperator(DualTokenType::Multiplicative(MultiplicativeTokenType::Divide)));
    tok!("//=", TokenType::DualOperator(DualTokenType::Multiplicative(MultiplicativeTokenType::IntegerDivide)));
    tok!("%=", TokenType::DualOperator(DualTokenType::Multiplicative(MultiplicativeTokenType::Modulo)));
    tok!("??=", TokenType::DualOperator(DualTokenType::Multiplicative(MultiplicativeTokenType::Coalesce)));

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
    tok!("++", TokenType::UnaryOperator(UnaryOperator::Increment));
    tok!("--", TokenType::UnaryOperator(UnaryOperator::Decrement));

    // Keywords
    tok!("is", TokenType::Is);
    tok!("in", TokenType::In);
    tok!("not", TokenType::Not);
    tok!("let", TokenType::Let);
    tok!("try", TokenType::Try);
    tok!("catch", TokenType::Catch);
    tok!("finally", TokenType::Finally);
    tok!("throw", TokenType::Throw);
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
    tok!("export", TokenType::Export);
    tok!("import", TokenType::Import);
    tok!("from", TokenType::From);
    tok!("as", TokenType::As);
    tok!("enum", TokenType::Enum);
    tok!("func", TokenType::Function);
    //tok!("pure", TokenType::Pure);
    tok!("step", TokenType::Step);
    tok!("return", TokenType::Return);
    tok!("assert", TokenType::Assert);

    // Basic Syntax
    tok!("(", TokenType::OpenParen);
    tok!(")", TokenType::CloseParen);
    tok!("{", TokenType::OpenBrace);
    tok!("}", TokenType::CloseBrace);
    tok!("[", TokenType::OpenSquare);
    tok!("]", TokenType::CloseSquare);
    tok!(",", TokenType::Comma);
    tok!(".", TokenType::Dot);
    tok!("...", TokenType::Spread);
    tok!("..", TokenType::Range);
    tok!(".<", TokenType::RangeUninclusive);
    tok!(":", TokenType::Colon);
    tok!("?", TokenType::QuestionMark);
    tok!(";", TokenType::Semicolon);
    tok!("$", TokenType::UnaryOperator(UnaryOperator::LengthOf));

    operators
  };
}

#[derive(Clone)]
pub struct LocationContents {
  pub contents: String,
  pub file_name: String,
}

static LOCATION_CONTENTS: Lazy<Arc<Mutex<HashMap<u128, LocationContents>>>> = Lazy::new(|| {
  let mut hash = HashMap::new();
  hash.insert(
    0_u128,
    LocationContents {
      contents: "No Location Contents".to_string(),
      file_name: "<unknown>".to_string(),
    },
  );
  Arc::from(Mutex::from(hash))
});
static CURRENT_CONTENTS: Lazy<Arc<Mutex<u128>>> = Lazy::new(|| Arc::from(Mutex::from(0)));

pub fn get_token(_t: &TokenType) -> String {
  OPERATORS
    .iter()
    .find_map(|(key, &val)| if val == _t { Some(key) } else { None })
    .unwrap()
    .to_string()
}

pub fn get_location_contents(id: u128) -> LocationContents {
  LOCATION_CONTENTS.lock().unwrap().get(&id).unwrap().clone()
}

static OPERATOR_KEYS: Lazy<Vec<&&str>> = Lazy::new(|| {
  let mut keys: Vec<&&str> = OPERATORS.keys().clone().collect();
  keys.sort_by(|a, b| b.len().cmp(&a.len()));
  keys
});

static STRING_ONLY_OPERATORS: Lazy<Vec<&&str>> = Lazy::new(|| {
  OPERATORS
    .keys()
    .filter(|x| x.chars().all(char::is_alphabetic))
    .collect()
});

pub fn lex(temp_contents: String, file_name: String) -> Result<Vec<Token>, ZephyrError> {
  // Remove stupid \r's
  let contents = temp_contents.replace("\r", "");

  // Increment the current contents ID
  *CURRENT_CONTENTS.lock().unwrap() += 1;
  let id = { *CURRENT_CONTENTS.lock().unwrap() };
  LOCATION_CONTENTS.lock().unwrap().insert(
    id,
    LocationContents {
      contents: contents.clone(),
      file_name,
    },
  );

  let mut chars: Vec<char> = contents.chars().collect();
  let mut tokens: Vec<Token> = vec![];

  let mut current_char: u32 = 0;
  let mut current_line: u32 = 0;

  while !chars.is_empty() {
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
      while !chars.is_empty() && chars[0] != '\n' {
        eat(&mut chars);
      }
      continue;
    }
    // Check for /* comment
    else if chars[0] == '/' && chars.len() >= 2 && chars[1] == '*' {
      let mut closed = false;

      while !chars.is_empty() {
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
    // Check for raw string literals
    else if chars[0] == '`' {
      // Remove quote mark
      eat(&mut chars);

      let mut value = String::from("");

      while !chars.is_empty() && chars[0] != '`' && chars[0] != '\n' {
        value.push_str(&eat(&mut chars));
      }

      // Make sure current character is a `
      if chars[0] != '`' {
        return Err(ZephyrError::lexer(
          "Unexpected end of string".to_string(),
          location,
        ));
      }

      eat(&mut chars);

      set_token(value, TokenType::String);
    }
    // Check for string literal
    else if chars[0] == '"' {
      // Remove quote mark
      eat(&mut chars);

      let mut value = String::from("");

      // Repeat until end of quote, found new line or EOF
      let start = location.clone();
      while chars[0] != '"' && chars[0] != '\n' && !chars.is_empty() {
        let char = eat(&mut chars);

        // Check if escape
        match char.as_str() {
          // It is escaping
          "\\" => {
            // Check if there is a character to escape
            if !chars.is_empty() {
              let next_char = eat(&mut chars);
              match next_char.as_str() {
                // Basic ones
                "n" => value.push('\n'),
                "r" => value.push('\r'),
                "t" => value.push('\t'),
                "\"" => value.push('\"'),
                "\\" => value.push('\\'),
                "\n" => value.push('\n'),
                "\r" => (),
                // Hex sequences, like x1b[31m, god knows how this work
                // chatgpt did it
                "x" => {
                  // Expect 2 more
                  if chars.len() < 2 {
                    return Err(ZephyrError::lexer(
                      "Invalid hexadecimal escape sequence".to_string(),
                      location,
                    ));
                  }

                  let hex_digits: String = eat(&mut chars) + &eat(&mut chars);
                  if let Ok(v) = u8::from_str_radix(&hex_digits, 16) {
                    value.push_str(String::from(v as char).as_str())
                  } else {
                    value.push_str(("\\x".to_string() + &hex_digits).as_str())
                  }
                }
                // Cannot escape given character
                _ => {
                  return Err(ZephyrError::lexer(
                    format!("Cannot escape the given character: {}", next_char),
                    location,
                  ))
                }
              };
            } else {
              return Err(ZephyrError::lexer(
                "Expected character to escape".to_string(),
                location,
              ));
            }
          }
          // It is not, so just push the val
          _ => {
            value.push_str(&char);
          }
        };
      }

      // Make sure current character is a "
      if chars[0] != '"' {
        return Err(ZephyrError::lexer_with_ref(
          "Unexpected ending of string".to_string(),
          location,
          start,
        ));
      }

      eat(&mut chars);

      set_token(value, TokenType::String);
    }
    // Check if current char is a number
    else if chars[0].is_numeric() {
      let mut value: String = eat(&mut chars);

      // Loop until not a number
      let mut allowed_chars = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
        .iter()
        .map(|v| v.to_string().chars().next().unwrap())
        .collect::<Vec<char>>();
      while !chars.is_empty()
        && (allowed_chars.contains(&chars[0])
          || (value.len() == 1 && (chars[0] == 'x' || chars[0] == 'o' || chars[0] == 'b')))
      {
        let c = &eat(&mut chars);
        value.push_str(c);

        // Check if should modify allowed
        match c.as_str() {
          "x" => allowed_chars = "abcdef0123456789".to_string().chars().collect(),
          "o" => allowed_chars = "01234567".to_string().chars().collect(),
          "b" => allowed_chars = vec!['0', '1'],
          _ => (),
        }
      }

      // Check if the first number is 0, if it is then try other bases
      if value.starts_with('0') && value.len() > 1 {
        // Check the base
        let base = value.chars().nth(1).unwrap();
        let mut actual_number_chars = value.chars();
        actual_number_chars.next();
        actual_number_chars.next();
        let actual_number = actual_number_chars.as_str();
        match base {
          'x' => {
            value = match i64::from_str_radix(actual_number, 16) {
              Ok(ok) => ok,
              Err(err) => {
                return Err(ZephyrError::lexer(
                  format!("Failed to parse hexadecimal string: {}", err),
                  location,
                ))
              }
            }
            .to_string()
          }
          'o' => {
            value = match i64::from_str_radix(actual_number, 16) {
              Ok(ok) => ok,
              Err(err) => {
                return Err(ZephyrError::lexer(
                  format!("Failed to parse octal string: {}", err),
                  location,
                ))
              }
            }
            .to_string()
          }
          'b' => {
            value = match i64::from_str_radix(actual_number, 16) {
              Ok(ok) => ok,
              Err(err) => {
                return Err(ZephyrError::lexer(
                  format!("Failed to parse binary string: {}", err),
                  location,
                ))
              }
            }
            .to_string()
          }
          _ => unimplemented!(),
        }
      }

      // Set token
      set_token(value, TokenType::Number);
    }
    // Check if the current char is alpha
    else if chars[0].is_alphabetic() || chars[0] == '_' || chars[0] == '@' {
      let is_special = chars[0] == '@';

      let mut value: String = eat(&mut chars);

      while !chars.is_empty() && (chars[0].is_alphanumeric() || chars[0] == '_') {
        value.push_str(&eat(&mut chars));
      }

      // Check for ?
      if !chars.is_empty() && chars[0] == '?' {
        value.push_str(&eat(&mut chars));
        set_token(value, TokenType::PredicateIdentifier);
      } else {
        // Check if it is an operator
        if STRING_ONLY_OPERATORS.contains(&&&*value) {
          let oper = *OPERATORS.get(&*value).unwrap();
          set_token(value, *oper);
        }
        // Set token
        else {
          set_token(
            value,
            if is_special {
              TokenType::SpecialIdentifier
            } else {
              TokenType::Identifier
            },
          )
        };
      };
    }
    // No token was found
    else {
      // Check for symbol operators
      let mut found: bool = false;
      for key in &OPERATOR_KEYS.clone() {
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
