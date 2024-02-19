use std::{collections::HashMap, fs, io::ErrorKind, path::PathBuf};

use crate::{
  die,
  lexer::{
    self,
    location::Location,
    token::{Token, TokenType},
  },
  mini::compress_tokens,
  parser::{nodes::Expression, parser},
};

#[derive(Debug)]
pub struct ExtractionResult {
  pub imports: Vec<(String, String)>,
}

pub fn extract(file_name: String) -> ExtractionResult {
  crate::debug(
    &format!("Preparing {} to be bundled...", file_name.clone()),
    "bundler",
  );
  let input = match std::fs::read_to_string(file_name.clone()) {
    Ok(ok) => ok,
    Err(err) => {
      die(match err.kind() {
        ErrorKind::NotFound => format!("File {} does not exist", file_name),
        ErrorKind::PermissionDenied => format!("Failed to read {}: permission denied", file_name),
        _ => format!("Failed to open {}: {}", file_name, err),
      });
      panic!();
    }
  };

  let proper_file_name = fs::canonicalize(file_name.clone()).unwrap();

  // Get the tokens
  let tokens = match lexer::lexer::lex(input.clone(), proper_file_name.display().to_string()) {
    Ok(val) => val,
    Err(err) => {
      println!("{}", err.visualise(false));
      panic!();
    }
  };

  let mut imports: Vec<(String, String)> = vec![];
  let mut i: usize = 0;
  let mut new_tokens: Vec<Token> = vec![];
  // Loop through them, trying to find `from`
  while i != tokens.len() - 1 {
    let tok = &tokens[i];
    if tok.token_type == TokenType::From {
      crate::debug(&format!("Found from: {:#?}", i), "bundler");

      // Parse a import using the parser
      let old = tokens[i..].len();
      println!("{:#?}", tokens[i..].to_vec());
      let mut parser = parser::Parser::new(tokens[i..].to_vec());
      let import = match parser.parse_import_statement() {
        Ok(ok) => match ok {
          Expression::ImportStatement(imp) => imp,
          _ => unreachable!(),
        },
        Err(err) => {
          println!("{}", err.visualise(false));
          panic!();
        }
      };

      // Check if file exists
      let mut path = proper_file_name.clone();
      path.pop();
      path.push(import.from.value);

      // Check if it exists
      if !path.exists() {
        die(format!(
          "The path {} does not exist",
          path.display().to_string()
        ));
        panic!();
      }

      let path_whole = fs::canonicalize(path).unwrap();
      if !imports.contains(&path_whole.display().to_string()) {
        imports.push(path_whole.display().to_string());
      }

      // Check how many tokens were removed
      let removed = old - parser.tokens.len();
      crate::debug(
        &format!(
          "The import took {} tokens, it imported {}",
          removed,
          path_whole.display().to_string()
        ),
        "bundler",
      );
      i += removed;

      // Add the import to new_tokens
      for i in import.import {
        new_tokens.push(Token {
          token_type: TokenType::Identifier,
          value: format!(
            "let {} = (__imports[\"{}\"]())[\"{}\"];",
            i.1.symbol,
            path_whole.display().to_string(),
            i.0.symbol
          ),
          location: Location::no_location(),
        });
      }
    } else {
      new_tokens.push(tok.clone());
    }
    i += 1;
  }

  ExtractionResult {
    imports,
    contents: compress_tokens(new_tokens),
  }
}

pub fn bundle(input: String, file_name: String) -> String {
  // Get the tokens
  let _result = match lexer::lexer::lex(input.clone(), file_name.clone()) {
    Ok(val) => val,
    Err(err) => {
      println!("{}", err.visualise(false));
      panic!();
    }
  };

  let mut files: HashMap<String, String> = HashMap::new();

  let data = extract(file_name.clone());
  println!("{:#?}", data);

  // Add index
  files.insert(file_name.clone(), input.clone());

  // Construct import map
  let mut result = String::from("let __import_cache = .{};\nlet __imports = .{");

  // Loop through found files
  for i in files {
    result.push_str(&format!("\"{}\": func {{", i.0));

    result.push_str(&format!(
      "if !(\"{}\" in __import_cache) {{ {} __import_cache[\"{}\"] = .{{}}; }} return __import_cache[\"{}\"];",
      i.0, i.1, i.0, i.0
    ));

    result.push_str("},");
  }

  // Close __imports
  result.push_str("};\n");

  // Call the index
  result.push_str(&format!("(__imports[\"{}\"])();\n", file_name.clone()));

  result.clone()
}
