use std::collections::HashMap;
use std::mem::{discriminant, Discriminant};

use super::nodes::{
  self, ArithmeticExpression, AssignmentExpression, ComparisonExpression, Expression,
  FunctionLiteral, Identifier, InExpression, LogicalExpression, TryExpression, TypeofStatement,
  VariableDeclaration,
};
use crate::lexer::location::Location;
use crate::lexer::token::{DualTokenType, MultiplicativeTokenType, UnaryOperator};
use crate::{
  errors::ZephyrError,
  lexer::token::{Token, TokenType},
};

macro_rules! parser_section {
  ($name:tt, $self:ident, $body:expr) => {
    pub fn $name(&mut $self) -> Result<nodes::Expression, ZephyrError> { $body }
  }
}

macro_rules! expect_any_ident {
  ($what:expr) => {
    matches!($what, TokenType::Identifier) || matches!($what, TokenType::PredicateIdentifier)
  };
}

pub struct Parser {
  pub tokens: Vec<Token>,
}

impl Parser {
  pub fn new(tokens: Vec<Token>) -> Parser {
    Parser { tokens }
  }

  pub fn at(&self) -> Token {
    if !self.tokens.is_empty() {
      self.tokens[0].clone()
    } else {
      panic!("Tokens is empty");
    }
  }

  pub fn eat(&mut self) -> Token {
    let token = self.at();
    self.tokens.remove(0);
    token
  }

  pub fn expect(
    &mut self,
    t: Discriminant<TokenType>,
    mut e: ZephyrError,
  ) -> Result<Token, ZephyrError> {
    let exists = discriminant(&self.at().token_type) == t;
    if !exists {
      e.location = self.at().location;
      return Err(e);
    }
    Ok(self.eat())
  }

  pub fn expect_one_of(
    &mut self,
    t: Vec<Discriminant<TokenType>>,
    mut e: ZephyrError,
  ) -> Result<Token, ZephyrError> {
    let mut exists = false;

    for i in t {
      if discriminant(&self.at().token_type) == i {
        exists = true;
      }
    }

    if !exists {
      e.location = self.at().location;
      return Err(e);
    }
    Ok(self.eat())
  }

  pub fn does_not_need_semicolon(&mut self, t: nodes::Expression) -> bool {
    match t {
      nodes::Expression::FunctionLiteral(_) => true,
      nodes::Expression::VariableDeclaration(_) => true,
      nodes::Expression::ForLoop(_) => true,
      nodes::Expression::WhileExpression(_) => true,
      nodes::Expression::Block(_) => true,
      nodes::Expression::Program(_) => true,
      nodes::Expression::IfExpression(_) => true,
      nodes::Expression::ExportStatement(_) => true,
      nodes::Expression::TryExpression(_) => true,
      _ => false,
    }
  }

  pub fn create_identifier(&self, val: Token) -> Result<Identifier, ZephyrError> {
    match val.token_type {
      TokenType::Identifier => Ok(Identifier {
        symbol: val.value,
        location: val.location,
      }),
      TokenType::PredicateIdentifier => Ok(Identifier {
        symbol: val.value,
        location: val.location,
      }),
      TokenType::SpecialIdentifier => Ok(Identifier {
        symbol: val.value,
        location: val.location,
      }),
      TokenType::String => Ok(Identifier {
        symbol: val.value,
        location: val.location,
      }),
      _ => Err(ZephyrError::parser(
        "Expected an identifier".to_string(),
        self.at().location,
      )),
    }
  }

  pub fn call_with_inferred_first(
    &mut self,
    first: nodes::Expression,
    ident: nodes::Identifier,
  ) -> Result<nodes::Expression, ZephyrError> {
    let identifier = &nodes::Expression::Identifier(ident);

    // Create the call expr
    Ok(nodes::Expression::CallExpression(
      if matches!(self.at().token_type, TokenType::OpenParen) {
        let mut call_expr = match self.parse_call_expression(Some(identifier.clone()))? {
          nodes::Expression::CallExpression(expr) => expr,
          _ => unreachable!(),
        };

        call_expr.arguments.insert(0, Box::from(first));
        call_expr
      } else {
        nodes::CallExpression {
          left: Box::from(identifier.clone()),
          location: identifier.get_location(),
          arguments: vec![Box::from(first)],
        }
      },
    ))
  }

  pub fn parse_argument_list(&mut self) -> Result<Vec<Box<nodes::Expression>>, ZephyrError> {
    // Check for the (
    self.expect(
      discriminant(&TokenType::OpenParen),
      ZephyrError::parser("Expected open parenthesis".to_string(), self.at().location),
    )?;

    // Collect arguments
    let mut arguments: Vec<Box<nodes::Expression>> = vec![];
    while !matches!(self.at().token_type, TokenType::CloseParen) {
      arguments.push(Box::new(self.parse_expression()?));
      if !matches!(self.at().token_type, TokenType::Comma) {
        break;
      } else {
        self.eat()
      };
    }

    // Check for the )
    self.expect(
      discriminant(&TokenType::CloseParen),
      ZephyrError::parser("Expected close parenthesis".to_string(), self.at().location),
    )?;

    Ok(arguments)
  }

  pub fn parse_block(&mut self) -> Result<nodes::Block, ZephyrError> {
    let tok = self.expect(
      discriminant(&TokenType::OpenBrace),
      ZephyrError::parser(
        "Expected open of block brace".to_string(),
        self.at().location,
      ),
    )?;

    let expressions = self.parse_inner_block(true)?;

    self.expect(
      discriminant(&TokenType::CloseBrace),
      ZephyrError::parser(
        "Expected close of block brace".to_string(),
        self.at().location,
      ),
    )?;

    Ok(nodes::Block {
      nodes: expressions,
      location: tok.location,
    })
  }

  pub fn parse_inner_block(
    &mut self,
    is_block_type: bool,
  ) -> Result<Vec<Box<nodes::Expression>>, ZephyrError> {
    let mut expressions: Vec<Box<nodes::Expression>> = vec![];
    self.at();

    while !matches!(self.at().token_type, TokenType::EOF)
      && !matches!(self.at().token_type, TokenType::CloseBrace)
    {
      // Ignore random semi colons
      if matches!(self.at().token_type, TokenType::Semicolon) {
        self.eat();
        continue;
      }

      let expression = &self.parse_statement()?;
      expressions.push(Box::from(expression.clone()));

      // Check if needs semicolon
      if !self.does_not_need_semicolon(expression.clone()) {
        self.expect(
          discriminant(&TokenType::Semicolon),
          ZephyrError::parser("Expected semicolon".to_string(), self.at().location),
        )?;
      }
    }

    if !is_block_type {
      self.expect(
        discriminant(&TokenType::EOF),
        ZephyrError::parser("Expected end of file".to_string(), self.at().location),
      )?;
    }

    Ok(expressions)
  }

  pub fn produce_ast(&mut self, file_name: Option<String>) -> Result<nodes::Program, ZephyrError> {
    Ok(nodes::Program {
      nodes: self.parse_inner_block(false)?,
      file: file_name,
    })
  }

  // ----- Statements -----

  parser_section! {parse_statement, self, {
    match self.at().token_type {
      TokenType::Let => self.parse_variable_declaration(),
      TokenType::Function => self.parse_function_declaration(),
      TokenType::Export => self.parse_export_statement(),
      TokenType::From => self.parse_import_statement(),
      TokenType::Import => self.parse_import_statement(),
      TokenType::Enum => self.parse_enum_declaration(),
      TokenType::Assert => Ok(nodes::Expression::AssertStatement(nodes::AssertStatement {
        location: self.eat().location,
        value: Box::from(self.parse_expression()?),
      })),
      TokenType::Throw => {
        let tok = self.eat();
        Ok(nodes::Expression::ThrowStatement(nodes::ThrowStatement {
          location: tok.location,
          what: Box::from(self.parse_expression()?),
          is_rethrow: false,
        }))
      }
      TokenType::Break => Ok(nodes::Expression::BreakStatement(nodes::BreakStatement {
        location: self.eat().location,
        name: if matches!(self.at().token_type, TokenType::SpecialIdentifier) {
          let tok = self.eat();
          Some(self.create_identifier(tok)?)
        } else { None }
      })),
      TokenType::Continue => Ok(nodes::Expression::ContinueStatement(nodes::ContinueStatement {
        location: self.eat().location,
        name: if matches!(self.at().token_type, TokenType::SpecialIdentifier) {
          let tok = self.eat();
          Some(self.create_identifier(tok)?)
        } else { None }
      })),
      TokenType::Return => Ok(nodes::Expression::ReturnStatement(nodes::ReturnStatement {
        location: self.eat().location,
        value: if matches!(self.at().token_type, TokenType::Semicolon) {
          None
        } else {
          Some(Box::from(self.parse_expression()?))
        },
      })),
      _ => self.parse_expression()
    }
  }}

  parser_section! {parse_enum_declaration, self, {
    let enum_token = self.eat();

    // Expect name
    let name = self.expect(
      discriminant(&TokenType::Identifier),
      ZephyrError::parser("Expected enum name here".to_string(), self.at().location)
    )?;

    // Expect {
    self.expect(
      discriminant(&TokenType::OpenBrace),
      ZephyrError::parser("Expected open enum block here".to_string(), self.at().location)
    )?;

    let mut object = nodes::ObjectLiteral {
      items: HashMap::new(),
      location: enum_token.location,
    };
    let mut used_names: Vec<String> = vec![];
    let mut use_identifiers = false;

    while !matches!(self.at().token_type, TokenType::CloseBrace) {
      // Check if current is "use_strings"
      if matches!(self.at().token_type, TokenType::String) && !use_identifiers {
        let val = self.eat();

        if val.value == "use_strings" {
          use_identifiers = true;
          if matches!(self.at().token_type, TokenType::Comma) {
            self.eat();
            continue;
          } else {
            break;
          }
        }
      }

      let current_identifier = self.expect(
        discriminant(&TokenType::Identifier),
        ZephyrError::parser("Expected identifier here".to_string(), self.at().location)
      )?;

      // Check if already in enum
      if used_names.contains(&current_identifier.value) {
        return Err(ZephyrError::parser_with_ref(
          format!("Enum variant {} already used in enum", current_identifier.value),
          current_identifier.location,
          enum_token.location
        ));
      }

      // Add to object
      let value = Box::from(if !use_identifiers {
        nodes::Expression::NumericLiteral(nodes::NumericLiteral {
          value: used_names.len() as f64,
          location: current_identifier.location
        })
      } else {
        nodes::Expression::StringLiteral(nodes::StringLiteral {
          value: current_identifier.value.clone(),
          location: current_identifier.location
        })
      });
      object.items.insert(current_identifier.clone().value, value);
      used_names.push(current_identifier.clone().value);

      // Check for ,
      if matches!(self.at().token_type, TokenType::Comma) {
        self.eat();
      } else {
        break;
      }
    }

    // Expect {
    self.expect(
      discriminant(&TokenType::CloseBrace),
      ZephyrError::parser("Expected close enum block here".to_string(), self.at().location)
    )?;

    // Construct sugar
    Ok(nodes::Expression::VariableDeclaration(nodes::VariableDeclaration {
      identifier: self.create_identifier(name)?,
      location: enum_token.location,
      value: Box::from(nodes::Expression::ObjectLiteral(object))
    }))
  }}

  parser_section! {parse_function_declaration, self, {
    let func = self.parse_function_literal()?;

    // Expect it to have a name
    if func.identifier.is_none() {
      return Err(ZephyrError::parser(
        "A function must have a name when used as a statement".to_string(),
        self.at().location
      ));
    }

    Ok(nodes::Expression::VariableDeclaration(nodes::VariableDeclaration {
      identifier: func.identifier.clone().unwrap(),
      location: func.location,
      value: Box::from(nodes::Expression::FunctionLiteral(func))
    }))
  }}

  parser_section! {parse_variable_declaration, self, {
    let var_token = self.eat();

    // Get the name of the variable
    let name =self.expect(
      discriminant(&TokenType::Identifier),
      ZephyrError::parser("Expected variable name here".to_string(), self.at().location)
    )?;

    // Expect an =
    self.expect(
      discriminant(&TokenType::NormalAssignmentOperator),
      ZephyrError::parser("Expected normal assignment operator".to_string(), self.at().location),
    )?;

    // Get the value
    let value = self.parse_expression()?;

    Ok(nodes::Expression::VariableDeclaration(VariableDeclaration {
      location: var_token.location,
      identifier: match name.token_type {
        TokenType::Identifier => self.create_identifier(name)?,
        _ => unreachable!()
      },
      value: Box::from(value),
    }))
  }}

  parser_section! {parse_import_statement, self, {
    // Check if it is an import keyword started
    if matches!(self.at().token_type, TokenType::Import) {
      let import_tok = self.eat();

      let to_import = self.expect(
        discriminant(&TokenType::String),
        ZephyrError::parser(
          "Expected string literal to import".to_string(),
          self.at().location
        )
      )?;

      // Check if it has an as
      let _as = if matches!(self.at().token_type, TokenType::As) {
        let _tok = self.eat();
        let name = self.expect(
          discriminant(&TokenType::Identifier),
          ZephyrError::parser(
            "Expected string literal to import".to_string(),
            self.at().location
          )
        )?;
        self.create_identifier(name)?
      } else {
        self.create_identifier(to_import.clone())?
      };

      return Ok(nodes::Expression::ImportStatement(nodes::ImportStatement {
        from: nodes::StringLiteral {
          value: to_import.value,
          location: to_import.location
        },
        import: vec![(self.create_identifier(Token {
          value: "*".to_string(),
          token_type: TokenType::Identifier,
          location: Location::no_location(),
        })?, _as)],
        location: import_tok.location
      }));
    }

    let token = self.expect(
      discriminant(&TokenType::From),
      ZephyrError::parser(
        "Expected from token".to_string(),
        self.at().location
      )
    )?;

    let to_import = self.expect(
      discriminant(&TokenType::String),
      ZephyrError::parser(
        "Expected string literal to import".to_string(),
        self.at().location
      )
    )?;

    self.expect(
      discriminant(&TokenType::Import),
      ZephyrError::parser(
        "Expected import token".to_string(),
        self.at().location
      )
    )?;

    let mut import_as: Vec<(Identifier, Identifier)> = vec![];

    while matches!(self.at().token_type, TokenType::Identifier) {
      let tok = self.eat();
      let ident = self.create_identifier(tok)?;

      // Check if as
      let as_ = if matches!(self.at().token_type, TokenType::As) {
        self.eat();
        let tok = self.expect(
          discriminant(&TokenType::Identifier),
          ZephyrError::parser(
            "Expected identifier".to_string(),
            self.at().location
          )
        )?;
        self.create_identifier(tok)?
      } else {
        ident.clone()
      };

      import_as.push((ident, as_));

      if !matches!(self.at().token_type, TokenType::Comma) { break }
      else { self.eat(); }
    }

    Ok(nodes::Expression::ImportStatement(nodes::ImportStatement {
      from: nodes::StringLiteral {
        value: to_import.value,
        location: to_import.location
      },
      import: import_as,
      location: token.location,
    }))
  }}

  parser_section! {parse_export_statement, self, {
    let token = self.expect(
      discriminant(&TokenType::Export),
      ZephyrError::parser(
        "Expected export token".to_string(),
        self.at().location
      )
    )?;

    let statement = self.parse_statement()?;
    match statement {
      Expression::Identifier(_) => (),
      Expression::ObjectLiteral(_) => (),
      Expression::VariableDeclaration(_) => (),
      _ => return Err(ZephyrError::parser(
        "Cannot use this with an export".to_string(),
        statement.get_location()
      ))
    };

    Ok(nodes::Expression::ExportStatement(nodes::ExportStatement {
      to_export: Box::from(statement),
      location: token.location
    }))
  }}

  // ----- Expressions -----

  parser_section! {parse_expression, self, {
    self.parse_assignment_expression()
  }}

  parser_section! {parse_assignment_expression, self, {
    let left = self.parse_high_prescedence_unary_expression()?;

    // Check if it is =
    if matches!(self.at().token_type, TokenType::NormalAssignmentOperator) {
      let tok = self.eat();

      // Check if it is allowed
      match left {
        nodes::Expression::Identifier(_) => (),
        nodes::Expression::MemberExpression(_) => (),
        _ => return Err(ZephyrError::parser(
          format!("Cannot assign to a {:?}", left),
          left.get_location(),
        ))
      };

      let right = self.parse_expression()?;

      return Ok(nodes::Expression::AssignmentExpression(nodes::AssignmentExpression {
        left: Box::from(left),
        right: Box::from(right),
        location: tok.location
      }));
    }

    Ok(left)
  }}

  parser_section! {parse_high_prescedence_unary_expression, self, {
    if matches!(self.at().token_type, TokenType::Not) {
      let tok = self.eat();
      let expr = self.parse_ternary_expression()?;

      return Ok(nodes::Expression::UnaryExpression(nodes::UnaryExpression {
        location: tok.location,
        value: Box::from(expr),
        operator: tok.token_type
      }));
    }

    self.parse_ternary_expression()
  }}

  parser_section! {parse_ternary_expression, self, {
    let left = self.parse_logical_expression()?;

    // Check for ternary
    if matches!(self.at().token_type, TokenType::QuestionMark) {
      let tok = self.eat();
      let success = self.parse_expression()?;

      // Expect colon
      self.expect(
        discriminant(&TokenType::Colon),
        ZephyrError::parser(
          "Expected colon for ternary".to_string(),
          self.at().location
        )
      )?;

      let alternate = self.parse_expression()?;

      return Ok(nodes::Expression::IfExpression(nodes::IfExpression {
        success: Box::from(nodes::Block {
          nodes: vec![Box::from(success.clone())],
          location: success.get_location(),
        }),
        alternate: Some(Box::from(alternate)),
        test: Box::from(left),
        location: tok.location
      }));
    }

    Ok(left)
  }}

  parser_section! {parse_logical_expression, self, {
    let mut left = self.parse_comparison_operator()?;

    // Check if it is an comparison
    while !self.tokens.is_empty() && matches!(self.at().token_type, TokenType::LogicalOperator(_)) {
      let oper = self.eat();
      let right = self.parse_comparison_operator()?;

      left = nodes::Expression::LogicalExpression(LogicalExpression {
        left: Box::from(left),
        right: Box::from(right),
        location: oper.location,
        operator: match oper.token_type {
          TokenType::LogicalOperator(val) => val,
          _ => unreachable!()
        }
      });
    }

    Ok(left)
  }}

  parser_section! {parse_comparison_operator, self, {
    let mut left = self.parse_is_expression()?;

    // Check if it is an comparison
    while !self.tokens.is_empty() && matches!(self.at().token_type, TokenType::ComparisonTokenType(_)) {
      let oper = self.eat();
      let right = self.parse_is_expression()?;

      left = nodes::Expression::ComparisonOperator(ComparisonExpression {
        left: Box::from(left),
        right: Box::from(right),
        location: oper.location,
        operator: match oper.token_type {
          TokenType::ComparisonTokenType(val) => val,
          _ => unreachable!()
        }
      });
    }

    Ok(left)
  }}

  parser_section!(parse_is_expression, self, {
    let left = self.parse_typeof_statement()?;

    if matches!(self.at().token_type, TokenType::Is) {
      let _tok = self.eat();

      // Get the right
      let right_prematch = self.parse_typeof_statement()?;
      match right_prematch {
        nodes::Expression::Identifier(ident) => return self.call_with_inferred_first(left, ident),
        nodes::Expression::CallExpression(mut expr) => {
          expr.arguments.insert(0, Box::from(left));
          return Ok(nodes::Expression::CallExpression(expr));
        }
        _ => {
          return Err(ZephyrError::parser(
            format!("Cannot use {:?} with is expression", right_prematch),
            Location::no_location(),
          ))
        }
      };

      /*// Construct
      return Ok(nodes::Expression::IsExpression(nodes::IsExpression {
        left: Box::from(left),
        right: Box::from(right),
      }));*/
    }

    Ok(left)
  });

  parser_section! {parse_typeof_statement, self, {
    if matches!(self.at().token_type, TokenType::Typeof) {
      let typeof_token = self.eat();
      let value = self.parse_in_expression()?;

      return Ok(nodes::Expression::TypeofExpression(TypeofStatement {
        location: typeof_token.location,
        value: Box::from(value)
      }));
    }

    Ok(self.parse_in_expression())?
  }}

  parser_section! {parse_in_expression, self, {
    let left = self.parse_range_expression()?;

    if matches!(self.at().token_type, TokenType::In) {
      let token = self.eat();
      let right = self.parse_range_expression()?;

      return Ok(nodes::Expression::InExpression(InExpression {
        left: Box::from(left),
        right: Box::from(right),
        location: token.location
      }));
    }

    Ok(left)
  }}

  parser_section! {parse_range_expression, self, {
    let from = self.parse_additive_expression()?;

    if matches!(self.at().token_type, TokenType::Range) || matches!(self.at().token_type, TokenType::RangeUninclusive) {
      let uninclusive = matches!(self.at().token_type, TokenType::RangeUninclusive);
      let token = self.eat();

      let to = self.parse_additive_expression()?;

      let step = if matches!(self.at().token_type, TokenType::Step) {
        self.eat();
        Some(Box::from(self.parse_additive_expression()?))
      } else {
        None
      };

      return Ok(nodes::Expression::RangeExpression(nodes::RangeExpression {
        from: Box::from(from),
        to: Box::from(to),
        step,
        uninclusive,
        location: token.location,
      }));
    }

    Ok(from)
  }}

  parser_section! {parse_additive_expression, self, {
    let mut left = self.parse_multiplicative_expression()?;

    // Check if dual additive
    while !self.tokens.is_empty() && matches!(self.at().token_type, TokenType::DualOperator(DualTokenType::Additive(_))) {
      let oper = self.eat();
      let right = self.parse_unary_expression()?;

      left = nodes::Expression::AssignmentExpression(AssignmentExpression {
        left: Box::from(left.clone()),
        right: Box::from(nodes::Expression::ArithmeticOperator(ArithmeticExpression {
          left: Box::from(left.clone()),
          right: Box::from(right),
          location: oper.location,
          operator: match oper.token_type {
            TokenType::DualOperator(DualTokenType::Additive(add)) => TokenType::AdditiveOperator(add),
            _ => unreachable!()
          }
        })),
        location: oper.location,
      });
    }

    // Check if it is an additive
    while !self.tokens.is_empty() && matches!(self.at().token_type, TokenType::AdditiveOperator(_)) {
      let oper = self.eat();
      let right = self.parse_multiplicative_expression()?;

      left = nodes::Expression::ArithmeticOperator(ArithmeticExpression {
        left: Box::from(left),
        right: Box::from(right),
        location: oper.location,
        operator: match oper.token_type {
          TokenType::AdditiveOperator(val) => TokenType::AdditiveOperator(val),
          _ => unreachable!()
        }
      });
    }

    Ok(left)
  }}

  parser_section! {parse_multiplicative_expression, self, {
    let mut left = self.parse_unary_expression()?;

    // Check if dual additive
    while !self.tokens.is_empty() && matches!(self.at().token_type, TokenType::DualOperator(DualTokenType::Multiplicative(_))) {
      let oper = self.eat();
      let right = self.parse_unary_expression()?;

      left = nodes::Expression::AssignmentExpression(AssignmentExpression {
        left: Box::from(left.clone()),
        right: Box::from(nodes::Expression::ArithmeticOperator(ArithmeticExpression {
          left: Box::from(left.clone()),
          right: Box::from(right),
          location: oper.location,
          operator: match oper.token_type {
            TokenType::DualOperator(DualTokenType::Multiplicative(add)) => TokenType::MultiplicativeOperator(add),
            _ => unreachable!()
          }
        })),
        location: oper.location,
      });
    }

    // Check if it is an additive
    while !self.tokens.is_empty() && matches!(self.at().token_type, TokenType::MultiplicativeOperator(_)) {
      let oper = self.eat();
      let right = self.parse_unary_expression()?;

      if matches!(oper.token_type, TokenType::MultiplicativeOperator(MultiplicativeTokenType::Banginterrobang)) {
        left = nodes::Expression::TryExpression(TryExpression {
          main: Box::from(nodes::Block {
            nodes: vec![Box::from(left.clone())],
            location: left.get_location(),
          }),
          catch_identifier: None,
          finally: None,
          catch: Some(Box::from(nodes::Block {
            nodes: vec![Box::from(right.clone())],
            location: right.get_location(),
          })),
          location: oper.location
        })
      } else {
        left = nodes::Expression::ArithmeticOperator(ArithmeticExpression {
          left: Box::from(left),
          right: Box::from(right),
          location: oper.location,
          operator: match oper.token_type {
            TokenType::MultiplicativeOperator(val) => TokenType::MultiplicativeOperator(val),
            _ => unreachable!()
          }
        });
      }
    }

    Ok(left)
  }}

  parser_section! {parse_unary_expression, self, {
    // Check for unary left
    if matches!(self.at().token_type, TokenType::UnaryOperator(_)) || matches!(self.at().token_type, TokenType::AdditiveOperator(_)) {
      let tok = self.eat();

      // Get the right hand value
      let expr = self.parse_call_expression(None)?;

      return Ok(nodes::Expression::UnaryExpression(nodes::UnaryExpression {
        value: Box::from(expr),
        location: tok.location,
        operator: match tok.token_type {
          TokenType::UnaryOperator(val) => TokenType::UnaryOperator(val),
          TokenType::AdditiveOperator(val) => TokenType::AdditiveOperator(val),
          _ => unreachable!(),
        }
      }));
    }

    // Get a value
    let expr = self.parse_call_expression(None)?;

    // Check if it is a unary right expression
    if matches!(self.at().token_type, TokenType::UnaryOperator(_)) {
      let tok = self.eat();

      // Check if it can be used as a postfix
      if !matches!(tok.token_type, TokenType::UnaryOperator(UnaryOperator::Increment) | TokenType::UnaryOperator(UnaryOperator::Decrement)) {
        return Err(ZephyrError::lexer(
          format!("Cannot use {} as a postfix operator", tok.token_type),
          tok.location
        ));
      }

      // ay-ok
      return Ok(nodes::Expression::UnaryRightExpression(nodes::UnaryRightExpression {
        value: Box::from(expr),
        location: tok.location,
        operator: match tok.token_type {
          TokenType::UnaryOperator(val) => TokenType::UnaryOperator(val),
          _ => unreachable!(),
        }
      }));
    }

    Ok(expr)
  }}

  pub fn parse_call_expression(
    &mut self,
    set_left: Option<nodes::Expression>,
  ) -> Result<nodes::Expression, ZephyrError> {
    let left = if let Some(sl) = set_left {
      sl
    } else {
      self.parser_member_expression(None)?
    };

    // Check if it is a call
    if matches!(self.at().token_type, TokenType::OpenParen) {
      let location = self.at().location;
      let arguments = self.parse_argument_list()?;
      let expr = nodes::CallExpression {
        left: Box::new(left),
        arguments,
        location,
      };

      return if matches!(self.at().token_type, TokenType::Dot)
        || matches!(self.at().token_type, TokenType::OpenSquare)
      {
        self.parser_member_expression(Some(nodes::Expression::CallExpression(expr)))
      } else {
        self.parse_call_expression(Some(nodes::Expression::CallExpression(expr)))
      };
    }

    Ok(left)
  }

  pub fn parser_member_expression(
    &mut self,
    set_left: Option<nodes::Expression>,
  ) -> Result<nodes::Expression, ZephyrError> {
    let mut left = if let Some(sl) = set_left {
      sl
    } else {
      self.parse_primary_expression()?
    };
    let left_location = left.get_location();

    while !matches!(self.at().token_type, TokenType::EOF)
      && (matches!(self.at().token_type, TokenType::OpenSquare)
        || matches!(self.at().token_type, TokenType::Dot))
    {
      // Check if it is computed
      if matches!(self.at().token_type, TokenType::OpenSquare) {
        self.eat();
        let expr = self.parse_expression()?;
        let expr_location = expr.get_location();
        self.expect(
          discriminant(&TokenType::CloseSquare),
          ZephyrError::parser(
            "Expected closing square for index".to_string(),
            Location::no_location(),
          ),
        )?;

        left = nodes::Expression::MemberExpression(nodes::MemberExpression {
          left: Box::from(left),
          key: Box::from(expr),
          is_computed: true,
          location: left_location.combine_with(expr_location),
        });
      } else if matches!(self.at().token_type, TokenType::Dot) {
        self.eat();
        let ident_location;
        let ident = if expect_any_ident!(self.at().token_type) {
          let tok = self.eat();
          ident_location = tok.clone().location;
          self.create_identifier(tok)?
        } else {
          return Err(ZephyrError::parser(
            "Expected an identifier".to_string(),
            self.at().location,
          ));
        };

        left = nodes::Expression::MemberExpression(nodes::MemberExpression {
          left: Box::from(left),
          key: Box::from(nodes::Expression::Identifier(ident)),
          is_computed: false,
          location: left_location.combine_with(ident_location),
        });

        if matches!(self.at().token_type, TokenType::OpenParen) {
          left = self.parse_call_expression(Some(left))?;
        }
      } else {
        unreachable!();
      }
    }

    Ok(left)
  }

  // ----- Literals -----

  parser_section! {parse_primary_expression, self, {
    Ok(match self.at().token_type {
      TokenType::Number => nodes::Expression::NumericLiteral({
        let tok = self.eat();
        nodes::NumericLiteral {
          value: tok.value.parse::<f64>().unwrap(),
          location: tok.location,
        }}),
      TokenType::String => nodes::Expression::StringLiteral({
        let tok = self.eat();
        nodes::StringLiteral {
          value: tok.value,
          location: tok.location,
        }}),
      TokenType::Identifier => nodes::Expression::Identifier({
        let tok = self.eat();
        nodes::Identifier {
          symbol: tok.value,
          location: tok.location,
        }}),
      TokenType::PredicateIdentifier => nodes::Expression::Identifier({
        let tok = self.eat();
        nodes::Identifier {
          symbol: tok.value,
          location: tok.location,
        }}),
      TokenType::For => self.parse_for_expression()?,
      TokenType::Function => nodes::Expression::FunctionLiteral(self.parse_function_literal()?),
      TokenType::If => self.parse_if_expression()?,
      TokenType::While => self.parse_while_expression()?,
      TokenType::Loop => self.parse_while_expression()?,
      TokenType::Until => self.parse_while_expression()?,
      TokenType::Try => self.parse_try_expression()?,
      TokenType::OpenParen => {
        self.eat();
        let value = self.parse_expression()?;
        self.expect(
          discriminant(&TokenType::CloseParen),
          ZephyrError::parser("Expected closing paren".to_string(), self.at().location
        ))?;
        value
      },
      TokenType::Dot => {
        self.parse_object_literal()?
      },
      TokenType::OpenSquare => {
        let array_tok = self.eat();
        let mut items: Vec<Box<nodes::Expression>> = vec![];

        while !matches!(self.at().token_type, TokenType::EOF) &&
          !matches!(self.at().token_type, TokenType::CloseSquare) {
            items.push(Box::new(self.parse_expression()?));

          // Check for comma
          if !matches!(self.at().token_type, TokenType::Comma) {
            break;
          }

          // Remove comma
          self.eat();
        }

        let _ = self.expect(
          discriminant(&TokenType::CloseSquare),
          ZephyrError::runtime(
            "Expected closing of array".to_string(),
            Location::no_location(),
          )
        )?;

        return Ok(nodes::Expression::ArrayLiteral(nodes::ArrayLiteral {
          location: array_tok.location,
          items
        }));
      },
      TokenType::OpenBrace => {
        //self.eat();
        let block = self.parse_block()?;
        nodes::Expression::Block(block)
      },
      _ => return Err(ZephyrError::parser(format!("Cannot handle this token {:?}", self.at().token_type), self.at().location))
    })
  }}

  parser_section! {parse_try_expression, self, {
    let token = self.expect(
      discriminant(&TokenType::Try),
      ZephyrError::parser(
        "Expected try token".to_string(),
        self.at().location
      )
    )?;

    // Get try block
    let block = self.parse_block()?;

    // Get catch block
    let mut catch_ident: Option<Identifier> = None;
    let catch = if matches!(self.at().token_type, TokenType::Catch) {
      self.eat();

      // Check if it defines a ident
      if matches!(self.at().token_type, TokenType::Identifier) {
        let tok = self.eat();
        catch_ident = Some(self.create_identifier(tok)?);
      }

        Some(Box::from(self.parse_block()?))
    } else { None };

    let finally = if matches!(self.at().token_type, TokenType::Finally) {
      self.eat();
      Some(Box::from(self.parse_block()?))
    } else { None };

    Ok(nodes::Expression::TryExpression(nodes::TryExpression {
      main: Box::from(block),
      catch_identifier: catch_ident,
      catch,
      location: token.location,
      finally
    }))
  }}

  parser_section! {parse_while_expression, self, {
    let token = self.expect_one_of(
      vec![
        discriminant(&TokenType::While),
        discriminant(&TokenType::Loop),
        discriminant(&TokenType::Until)
      ],
      ZephyrError::parser(
        "Expected while token".to_string(),
        self.at().location,
      )
    )?;
    let token_location = token.location;

    // Expect expression
    let expression = match token.token_type {
      TokenType::While => self.parse_expression()?,
      TokenType::Until => {
        let expr = &self.parse_expression()?;
        nodes::Expression::UnaryExpression(nodes::UnaryExpression {
          value: Box::from(expr.clone()),
          operator: TokenType::UnaryOperator(UnaryOperator::Not),
          location: expr.clone().get_location(),
        })
      },
      TokenType::Loop => nodes::Expression::NumericLiteral(nodes::NumericLiteral {
        value: 1.0,
        location: token_location
      }),
      _ => unreachable!()
    };

    // Check for name
    let name = if matches!(self.at().token_type, TokenType::SpecialIdentifier) {
      let tok = self.eat();
      Some(self.create_identifier(tok)?)

    } else {
      None
    };

    // Expect body of while
    let body = self.parse_block()?;

    let else_none = if matches!(self.at().token_type, TokenType::Else) {
      self.eat();
      Some(Box::from(self.parse_block()?))
    } else {
      None
    };

    // Done
    Ok(nodes::Expression::WhileExpression(nodes::WhileExpression {
      test: Box::from(expression),
      body: Box::from(body),
      location: token_location,
      none: else_none,
      name
    }))
  }}

  pub fn parse_if_expression(&mut self) -> Result<nodes::Expression, ZephyrError> {
    let token = self.expect(
      discriminant(&TokenType::If),
      ZephyrError::parser("Expected if token".to_string(), self.at().location),
    )?;

    // Get the test
    let test = self.parse_expression()?;

    // Get the success block
    let success = self.parse_block()?;

    // Check for alternate
    let alternate = if matches!(self.at().token_type, TokenType::Else) {
      self.eat();
      Some(Box::from(match self.at().token_type {
        TokenType::If => self.parse_if_expression()?,
        TokenType::OpenBrace => nodes::Expression::Block(self.parse_block()?),
        _ => {
          return Err(ZephyrError::parser(
            "Cannot use this with else, expected if or a block".to_string(),
            self.at().location,
          ))
        }
      }))
    } else {
      None
    };

    Ok(nodes::Expression::IfExpression(nodes::IfExpression {
      test: Box::from(test),
      success: Box::from(success),
      alternate,
      location: token.location,
    }))
  }

  pub fn parse_for_expression(&mut self) -> Result<nodes::Expression, ZephyrError> {
    let token = self.expect(
      discriminant(&TokenType::For),
      ZephyrError::parser("Expected for token".to_string(), self.at().location),
    )?;

    // Expect identifier
    let ident = self.expect(
      discriminant(&TokenType::Identifier),
      ZephyrError::parser("Expected identifier".to_string(), self.at().location),
    )?;

    // Check if there is a comma (for the value identifier)
    let mut value_ident: Option<Identifier> = None;
    if matches!(self.at().token_type, TokenType::Comma) {
      self.eat();
      let vident = self.expect(
        discriminant(&TokenType::Identifier),
        ZephyrError::parser("Expected an identifier".to_string(), self.at().location),
      )?;
      value_ident = Some(self.create_identifier(vident)?);
    }

    // Expect in token
    self.expect(
      discriminant(&TokenType::In),
      ZephyrError::parser("Expected in token".to_string(), self.at().location),
    )?;

    // Expect expression
    let expr = self.parse_expression()?;

    // Check for name
    let name = if matches!(self.at().token_type, TokenType::SpecialIdentifier) {
      let tok = self.eat();
      Some(self.create_identifier(tok)?)
    } else {
      None
    };

    let block = self.parse_block()?;

    let else_none = if matches!(self.at().token_type, TokenType::Else) {
      self.eat();
      Some(Box::from(self.parse_block()?))
    } else {
      None
    };

    // Done
    Ok(nodes::Expression::ForLoop(nodes::ForLoop {
      index_identifier: self.create_identifier(ident)?,
      value_identifier: value_ident,
      value_to_iter: Box::from(expr),
      body: block,
      location: token.location,
      none: else_none,
      name,
    }))
  }

  pub fn parse_object_literal(&mut self) -> Result<nodes::Expression, ZephyrError> {
    // Expect .
    self.expect(
      discriminant(&TokenType::Dot),
      ZephyrError::parser(
        "Expected dot for obejct literal".to_string(),
        self.at().location,
      ),
    )?;

    // Expect "{"
    let object_token = self.expect(
      discriminant(&TokenType::OpenBrace),
      ZephyrError::parser(
        "Expected open brace for object".to_string(),
        self.at().location,
      ),
    )?;

    // Collect all the items
    let mut items: HashMap<String, Box<nodes::Expression>> = HashMap::new();
    while !matches!(self.at().token_type, TokenType::CloseBrace) {
      // Expect key
      let tok = self.expect_one_of(
        vec![
          discriminant(&TokenType::Identifier),
          discriminant(&TokenType::String),
        ],
        ZephyrError::parser(
          "Expected identifier as key for object item".to_string(),
          self.at().location,
        ),
      )?;
      let key = self.create_identifier(tok)?;

      // Check if the value is inferred
      let value = if !matches!(self.at().token_type, TokenType::Colon) {
        nodes::Expression::Identifier(key.clone())
      } else {
        self.eat();
        self.parse_expression()?
      };

      // Done
      items.insert(key.symbol.clone(), Box::from(value));

      // Check for ,
      if !matches!(self.at().token_type, TokenType::Comma) {
        break;
      } else {
        self.eat();
      }
    }

    self.expect(
      discriminant(&TokenType::CloseBrace),
      ZephyrError::parser(
        "Expected close brace for object".to_string(),
        self.at().location,
      ),
    )?;

    Ok(nodes::Expression::ObjectLiteral(nodes::ObjectLiteral {
      items,
      location: object_token.location,
    }))
  }

  pub fn parse_function_literal(&mut self) -> Result<nodes::FunctionLiteral, ZephyrError> {
    // Expect "function"
    let func_tok = self.expect(
      discriminant(&TokenType::Function),
      ZephyrError::parser("Expected function keyword".to_string(), self.at().location),
    )?;

    let is_pure = if matches!(self.at().token_type, TokenType::Pure) {
      self.eat();
      true
    } else {
      false
    };

    // Expect identifier
    let function_name = if expect_any_ident!(self.at().token_type) {
      let tok = self.eat();
      Some(self.create_identifier(tok)?)
    } else {
      None
    };

    let mut where_clauses: Vec<Box<nodes::Expression>> = vec![];

    // Collect arguments
    let arguments: Vec<Identifier> = if matches!(self.at().token_type, TokenType::OpenParen) {
      self.eat();
      let mut temp_arguments = vec![];
      while !matches!(self.at().token_type, TokenType::CloseParen) {
        let tok = self.expect(
          discriminant(&TokenType::Identifier),
          ZephyrError::parser(
            "Expected identifier as argument".to_string(),
            self.at().location,
          ),
        )?;
        let arg_ident = self.create_identifier(tok)?;
        temp_arguments.push(arg_ident.clone());

        // Check for :
        if matches!(self.at().token_type, TokenType::Colon) {
          self.eat();
          // a: test? = where test?(a)
          let test = self.parse_expression()?;
          where_clauses.push(match test {
            nodes::Expression::Identifier(ident) => {
              // Expect it to be predicate
              if !ident.symbol.ends_with('?') {
                return Err(ZephyrError::parser(
                  "Expected predicate identifier".to_string(),
                  ident.location,
                ));
              }

              Box::from(nodes::Expression::CallExpression(nodes::CallExpression {
                location: arg_ident.location,
                left: Box::from(nodes::Expression::Identifier(ident)),
                arguments: vec![Box::from(nodes::Expression::Identifier(arg_ident))],
              }))
            }
            nodes::Expression::CallExpression(mut expr) => {
              expr
                .arguments
                .insert(0, Box::from(nodes::Expression::Identifier(arg_ident)));
              Box::from(nodes::Expression::CallExpression(expr))
            }
            _ => {
              return Err(ZephyrError::parser(
                format!("Cannot use {:?} with where test", test),
                Location::no_location(),
              ))
            }
          });
        }

        // Check for comma
        if !matches!(self.at().token_type, TokenType::Comma) {
          break;
        } else {
          self.eat();
        }
      }
      self.eat();
      temp_arguments
    } else {
      vec![]
    };

    // Check for where
    if matches!(self.at().token_type, TokenType::Where) {
      self.eat();
      where_clauses.push(Box::from(self.parse_expression()?));
      while matches!(self.at().token_type, TokenType::Comma) {
        self.eat();
        where_clauses.push(Box::from(self.parse_expression()?));
      }
    }

    // Get the block
    let block = self.parse_block()?;

    Ok(FunctionLiteral {
      location: func_tok.location,
      identifier: function_name,
      body: Box::new(block),
      arguments,
      where_clauses: Box::from(nodes::WhereClause {
        tests: where_clauses,
      }),
      is_pure,
    })
  }
}
