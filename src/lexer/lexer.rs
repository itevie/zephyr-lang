use crate::{
  errors::ZephyrError,
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
  static ref RAW_PREFIX: &'static str = "r#";
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
    tok!("|>", TokenType::MultiplicativeOperator(MultiplicativeTokenType::Pipe));
    //tok!("!!?", TokenType::MultiplicativeOperator(MultiplicativeTokenType::Banginterrobang));

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
    tok!("rethrow", TokenType::Rethrow);
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

/*pub fn lex(contents: String, file_name: String) -> Result<Vec<Token>, ZephyrError> {
  contents = contents.replace('\r', "");

  // Get the ID for this one
  let id = {
    let mut current_id = CURRENT_CONTENTS.lock().unwrap();
    *current_id += 1;
    *current_id
  };

  // Insert
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
      eat(&mut chars);
      current_char += 1;

      char.to_string()
    };

    let mut set_token = |value, t| {
      token_value = Some(value);
      token_type = Some(t);
    };

    match chars[0] {

    }
  }

  Ok(tokens)
}*/

macro_rules! eat {
  ($a:ident, $b:ident) => {{
    $a += 1;
    $b.remove(0)
  }};
}

pub fn lex(mut contents: String, file_name: String) -> Result<Vec<Token>, ZephyrError> {
  // Remove stupid \r's
  contents = contents.replace('\r', "");
  let id = {
    let mut current_id = CURRENT_CONTENTS.lock().unwrap();
    *current_id += 1;
    *current_id
  };

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

    let mut set_token = |value, t| {
      token_value = Some(value);
      token_type = Some(t);
    };

    match chars[0] {
      // Whitespace
      ' ' | '\t' | '\r' => {
        eat!(current_char, chars);
        continue;
      }
      // Newlines
      '\n' => {
        chars.remove(0);
        current_char = 0;
        current_line += 1;
        continue;
      }
      // Comments - singleline
      '/' if chars.len() >= 2 && chars[1] == '/' => {
        while !chars.is_empty() && chars[0] != '\n' {
          eat!(current_char, chars);
        }
        continue;
      }
      // Comments - multiline
      '/' if chars.len() >= 2 && chars[1] == '*' => {
        while chars.len() >= 2 && (chars[0] != '*' || chars[1] != '/') {
          if chars[0] == '\n' {
            current_line += 1;
            current_char = 0;
            chars.remove(0);
          } else {
            eat!(current_char, chars);
          }
        }

        // Expect that there was a */
        if chars.len() < 3 && !(chars[0] != '*' && chars[0] != '/') {
          return Err(ZephyrError::lexer(
            "Multi-line comment not closed".to_string(),
            Location::no_location(),
          ));
        }

        eat!(current_char, chars);
        eat!(current_char, chars);
        continue;
      }
      // Strings
      '"' => {
        // Remove quote mark
        eat!(current_char, chars);

        let mut value = String::from("");

        // Repeat until end of quote, new line or EOF
        while !chars.is_empty() && chars[0] != '"' && chars[0] != '\n' {
          value.push_str(&match eat!(current_char, chars) {
            // Check for escaping
            '\\' => {
              // Check if there is a character to escape
              if chars.is_empty() {
                return Err(ZephyrError::lexer(
                  "Expected a character to escape".to_string(),
                  location,
                ));
              }

              match eat!(current_char, chars) {
                // Basic ones
                'n' => "\n".to_string(),
                'r' => "\r".to_string(),
                't' => "\t".to_string(),
                '"' => "\"".to_string(),
                '\\' => "\\".to_string(),
                // Hex sequences
                'x' => {
                  // Expect 2 more
                  if chars.len() < 2 {
                    return Err(ZephyrError::lexer(
                      "Invalid hexadecimal escape sequence".to_string(),
                      location,
                    ));
                  }

                  let hex_digits = chars.drain(0..2).collect::<String>();
                  if let Ok(v) = u8::from_str_radix(&hex_digits, 16) {
                    (v as char).to_string()
                  } else {
                    "\\x".to_string() + &hex_digits.as_str()
                  }
                }
                v => {
                  return Err(ZephyrError::lexer(
                    format!("Cannot escape the given character: {}", v),
                    location,
                  ))
                }
              }
            }
            v => v.to_string(),
          });
        }

        // Make sure the current character is a quote
        if chars.is_empty() || chars[0] != '"' {
          return Err(ZephyrError::lexer(
            "Unexpected ending of string".to_string(),
            location,
          ));
        }

        eat!(current_char, chars);

        set_token(value, TokenType::String);
      }
      // Numbers
      _ if chars[0].is_numeric() => {
        let mut value = String::new();
        let mut allowed_chars = "0123456789".chars().collect::<Vec<char>>();
        let bases = ['b', 'x', 'o'];
        let mut base: Option<char> = None;
        let mut used_float = false;

        // Check if it uses special base

        if chars.len() >= 2 && chars[0] == '0' && bases.contains(&chars[1]) {
          base = Some(chars[1]);

          allowed_chars = match chars[1] {
            'x' => "ABCDEFabcdef0123456789",
            'o' => "01234567",
            'b' => "01",
            _ => unreachable!(),
          }
          .chars()
          .collect::<Vec<char>>();

          eat!(current_char, chars);
          eat!(current_char, chars);
        }

        while !chars.is_empty() {
          value.push(match chars[0] {
            v if allowed_chars.contains(&v) => eat!(current_char, chars),
            '.' => {
              // Check if char AFTER is a number (for range operator mainly)
              if chars.len() > 2 && !chars[1].is_numeric() {
                break;
              }

              // Check if a bse is being used
              if let Some(_) = base {
                return Err(ZephyrError::lexer(
                  "Cannot use decimals while in different base".to_string(),
                  location,
                ));
              }

              // Check if already used
              if used_float {
                return Err(ZephyrError::lexer(
                  "Decimal point already used in this numeric literal".to_string(),
                  location,
                ));
              }

              used_float = true;
              eat!(current_char, chars)
            }
            _ => {
              break;
            }
          });
        }

        // Check if a base was applied
        if let Some(base) = base {
          value = match match base {
            'x' => i64::from_str_radix(&value, 16),
            'o' => i64::from_str_radix(&value, 8),
            'b' => i64::from_str_radix(&value, 2),
            _ => unreachable!(),
          } {
            Ok(ok) => ok.to_string(),
            Err(err) => {
              return Err(ZephyrError::lexer(
                format!(
                  "Failed to parse numeric literal using base {}: {}",
                  base, err
                ),
                location,
              ));
            }
          }
        }

        set_token(value, TokenType::Number);
      }
      // Identifiers
      _ if chars[0].is_alphabetic() || chars[0] == '_' || chars[0] == '@' => {
        let is_special = chars[0] == '@';
        let mut value: String = eat!(current_char, chars).to_string();

        while !chars.is_empty()
          && (chars[0].is_alphanumeric() || chars[0] == '_' || chars[0] == '!')
        {
          value.push(eat!(current_char, chars));
        }

        // Check for ?
        if !chars.is_empty() && chars[0] == '?' {
          value.push(eat!(current_char, chars));
          set_token(value, TokenType::PredicateIdentifier);
        } else {
          // Check if it is an operator
          if STRING_ONLY_OPERATORS.contains(&&&*value) {
            let oper = *OPERATORS.get(&*value).unwrap();
            set_token(value, *oper);
          } else {
            set_token(
              value,
              if is_special {
                TokenType::SpecialIdentifier
              } else {
                TokenType::Identifier
              },
            )
          }
        }
      }
      _ => {
        let op = OPERATOR_KEYS
          .iter()
          .find(|&&op| chars.starts_with(&op.chars().collect::<Vec<_>>()));

        if let Some(&op) = op {
          let len = op.len();
          let value = chars.drain(0..len).collect::<String>();
          current_char += len as u32;
          set_token(value, OPERATORS[op].clone());
        } else {
          return Err(ZephyrError::lexer(
            format!("Unexpected token: {}", chars[0]),
            location,
          ));
        }
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
