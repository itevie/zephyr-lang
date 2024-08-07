use std::{
  collections::HashMap,
  fs::{self, OpenOptions},
  io::{ErrorKind, Write},
  path::PathBuf,
  time::Duration,
};

use pbr::ProgressBar;

use crate::{
  cli,
  lexer::{
    self,
    location::Location,
    token::{Token, TokenType},
  },
  mini::compress_tokens,
  parser::{nodes::Expression, parser},
  util,
};

#[derive(Debug)]
pub struct ExtractionResult {
  pub imports: Vec<(String, String)>,
  pub contents: String,
}

pub fn extract(file_name: String) -> ExtractionResult {
  util::debug(
    &format!("Preparing {} to be bundled...", file_name),
    "bundler",
  );
  let input = match std::fs::read_to_string(file_name.clone()) {
    Ok(ok) => ok,
    Err(err) => {
      crate::util::die(match err.kind() {
        ErrorKind::NotFound => format!("File {} does not exist", file_name),
        ErrorKind::PermissionDenied => format!("Failed to read {}: permission denied", file_name),
        _ => format!("Failed to open {}: {}", file_name, err),
      });
      panic!();
    }
  };

  let proper_file_name = fs::canonicalize(file_name).unwrap();

  // Get the tokens
  let tokens: Vec<Token> = match lexer::lexer::lex(input, proper_file_name.display().to_string()) {
    Ok(val) => val,
    Err(err) => {
      crate::util::die(format!("\n{}", err.visualise(false)));
      panic!();
    }
  }
  .iter()
  .map(|v| Token {
    token_type: v.token_type,
    value: if v.token_type == TokenType::String {
      v.value.clone().replace('\n', "\\\\n")
    } else {
      v.value.clone()
    },
    location: v.location,
  })
  .collect();

  // Parse to check if it's valid
  match parser::Parser::new(tokens.clone()).produce_ast(None) {
    Ok(_) => (),
    Err(err) => {
      crate::util::die(format!("\n{}", err.visualise(false)));
      panic!();
    }
  }

  let mut imports: Vec<(String, String)> = vec![];
  let mut processed: Vec<String> = vec![];
  let mut i: usize = 0;
  let mut new_tokens: Vec<Token> = vec![];
  // Loop through them, trying to find `from`
  while i != tokens.len() - 1 {
    let tok = &tokens[i];
    if tok.token_type == TokenType::From {
      util::debug(&format!("Found from: {:#?}", tokens[i]), "bundler");

      // Parse a import using the parser
      let old = tokens[i..].len();
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
      let path_whole = fs::canonicalize(path.clone()).unwrap();

      // Check if it exists
      if !path.exists() {
        crate::util::die(format!("The path {} does not exist", path.display()));
        panic!();
      }

      // Check how many tokens were removed
      let removed = old - parser.tokens.len();
      util::debug(
        &format!(
          "The import took {} tokens, it imported {}",
          removed,
          path_whole.display()
        ),
        "bundler",
      );
      i += removed;

      // Add the import to new_tokens
      for i in import.import {
        new_tokens.push(Token {
          token_type: TokenType::Identifier,
          value: format!(
            "let {} = (__imports[`{}`]())[`{}`];",
            i.1.symbol,
            path_whole.display(),
            i.0.symbol
          ),
          location: Location::no_location(),
        });
      }

      if !processed.contains(&path_whole.display().to_string()) {
        // Extract the import
        let data = extract(path_whole.display().to_string());

        // Check if it had any imports
        for i in data.imports {
          if !processed.contains(&i.0) {
            imports.push((i.0, i.1));
          }
        }

        imports.push((path_whole.display().to_string(), data.contents));
        processed.push(path_whole.display().to_string());
      }
    } else if tok.token_type == TokenType::Export {
      util::debug("Found export token.. attempting to convert", "bundler");

      // Try to parse the export
      let old = tokens[i..].len();
      let mut parser = parser::Parser::new(tokens[i..].to_vec());
      let export = match parser.parse_export_statement() {
        Ok(ok) => match ok {
          Expression::ExportStatement(imp) => imp,
          _ => unreachable!(),
        },
        Err(err) => {
          println!("{}", err.visualise(false));
          panic!();
        }
      };

      // We need to add semicolon at end of this
      let removed = old - parser.tokens.len();
      for _ in 1..removed {
        i += 1;
        new_tokens.push(tokens[i].clone());
      }
      new_tokens.push(Token {
        value: String::from(";"),
        token_type: TokenType::Semicolon,
        location: Location::no_location(),
      });

      // Check what was exported
      match *export.to_export {
        Expression::VariableDeclaration(dec) => new_tokens.push(Token {
          value: format!(
            "__mod_exports[`{}`] = {}",
            dec.identifier.symbol, dec.identifier.symbol
          ),
          token_type: TokenType::Identifier,
          location: Location::no_location(),
        }),
        _ => panic!("This cannot be used to export"),
      };
      new_tokens.push(Token {
        value: String::from(";"),
        token_type: TokenType::Semicolon,
        location: Location::no_location(),
      });
    } else if tok.token_type == TokenType::String {
      new_tokens.push(Token {
        value: tok.value.replace('\n', "\\\\n"),
        token_type: TokenType::String,
        location: tok.location,
      });
    } else {
      new_tokens.push(tok.clone());
    }
    i += 1;
  }

  ExtractionResult {
    imports,
    contents: compress_tokens(new_tokens.clone()),
  }
}

pub fn bundle(input: String, file_name: String) -> String {
  // Get the tokens
  let _result = match lexer::lexer::lex(input.clone(), file_name.clone()) {
    Ok(val) => val,
    Err(err) => {
      crate::util::die(err.visualise(false));
      return "".to_string();
    }
  };

  let mut files: HashMap<String, String> = HashMap::new();

  let data = extract(file_name.clone());
  let mut imports = data.imports.clone();
  imports.push((
    fs::canonicalize(file_name.clone())
      .unwrap()
      .display()
      .to_string(),
    data.contents,
  ));

  // Add index
  files.insert(file_name.clone(), input);

  // Construct import map
  let mut result = String::from("let __import_cache = .{};\nlet __imports = .{\n");

  // Loop through found files
  for i in imports {
    result.push_str(&format!("`{}`: func {{\n", i.0));

    result.push_str(&format!(
      "if !(`{}` in __import_cache) {{ let __mod_exports = .{{}}; {} __import_cache[`{}`] = __mod_exports; }} return __import_cache[`{}`];",
      i.0, i.1, i.0, i.0
    ));

    result.push_str("\n},\n");
  }

  // Close __imports
  result.push_str("};\n");

  // Call the index
  result.push_str(&format!(
    "(__imports[`{}`])();\n",
    PathBuf::from(file_name).canonicalize().unwrap().display()
  ));

  result.clone()
}

// idk why i made this i was bored
pub fn bundle_executable(
  input: String,
  file_name: String,
  _out_file: String,
  options: cli::BundleFile,
) {
  let parts = 7;
  let mut pb = ProgressBar::new(parts);
  pb.show_speed = false;
  pb.show_time_left = false;

  // Bundle
  let contents = bundle(input, file_name);

  // Create temp directory
  pb.message("Creating temp directory ");
  if PathBuf::from("./bundler_executable_temp").exists() {
    fs::remove_dir_all("./bundler_executable_temp").unwrap();
  }
  match fs::create_dir("bundler_executable_temp") {
    Ok(_) => (),
    Err(err) => return crate::util::die(format!("Failed to create temporary directory: {}", err)),
  };
  pb.inc();

  // Git clone
  pb.message("Cloning Zephyr GitHub repository ");
  util::debug("Attempting to clone GitHub repository...", "bundler");
  match std::process::Command::new("git")
    .args([
      "clone",
      "https://github.com/itevie/zephyr-lang",
      "bundler_executable_temp",
    ])
    .output()
  {
    Ok(_) => (),
    Err(err) => return crate::util::die(format!("Failed to run git, is git installed?: {}", err)),
  };

  // Expect file to exist
  let index_rs = PathBuf::from("./bundler_executable_temp/src/main.rs");

  // Check if it exists
  if !index_rs.exists() {
    return crate::util::die("Failed to clone GitHub repository, is git installed?".to_string());
  }

  util::debug(
    &format!(
      "Git clone successful, index file is: {}",
      fs::canonicalize(index_rs.clone()).unwrap().display()
    ),
    "bundler",
  );
  pb.inc();

  util::debug("Waiting few seconds...", "bundler");
  std::thread::sleep(Duration::from_secs(3));
  pb.inc();

  // Read index
  pb.message("Building ");
  let mut index_contents = match fs::read_to_string(index_rs.clone()) {
    Ok(ok) => ok,
    Err(err) => return crate::util::die(format!("Failed to read index.rs: {}", err)),
  };
  index_contents = index_contents.replace(
    "let bundled_data = \"\";",
    &format!("let bundled_data = r#\"{}\"#;", contents.replace('\n', "")),
  );
  pb.inc();

  // Write the file
  let mut f = match OpenOptions::new().write(true).open(index_rs) {
    Ok(ok) => ok,
    Err(err) => return crate::util::die(format!("Failed to open index.rs: {}", err)),
  };
  match f.write_all(index_contents.as_bytes()) {
    Ok(_) => (),
    Err(err) => return crate::util::die(format!("Failed to write index.rs: {}", err)),
  };
  pb.inc();

  util::debug(
    "Successfully modified index.rs, now attempting to compile...",
    "bundler",
  );

  let target = options.target.map(|target| match target.as_str() {
    "windows" => "x86_64-pc-windows-gnu".to_string(),
    _ => target,
  });

  let mut args: Vec<String> = vec![
    "build".to_string(),
    "--release".to_string(),
    "--manifest-path=./bundler_executable_temp/Cargo.toml".to_string(),
  ];

  // Check if should add target
  if let Some(t) = target.clone() {
    args.push(format!("--target={}", t));
  }

  // Try to run cargo build
  match std::process::Command::new("cargo")
    .args(args)
    .stdout(std::process::Stdio::piped())
    .output()
  {
    Ok(_) => (),
    Err(err) => {
      return crate::util::die(format!("Failed to run cargo, is cargo installed?: {}", err))
    }
  }

  // Construct path
  let mut path = PathBuf::from("./bundler_executable_temp/target/");
  if let Some(t) = target.clone() {
    path.push(t.clone());
    path.push("release");

    if t.contains("windows") {
      path.push("rust-zephyr.exe");
    } else {
      path.push("rust-zephyr");
    }
  } else {
    path.push("release");
    if cfg!(windows) {
      path.push("rust-zephyr.exe");
    } else {
      path.push("rust-zephyr");
    }
  }

  // Check if it exists
  if !path.exists() {
    panic!(
      "{}",
      format!(
        "Failed to compile, is cargo installed? (the path {} was not found)",
        path.display()
      )
    );
  }
  pb.inc();

  // Copy
  util::debug("Copying outputted executable to out file...", "bundler");

  fs::copy(path, _out_file).unwrap();
  pb.inc();

  util::debug("Done, cleaning up", "bundler");
  fs::remove_dir_all("./bundler_executable_temp").unwrap();
  pb.finish_print("Done!");
}
