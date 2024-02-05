use core::fmt;

use super::location::Location;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenType {
  // ----- Basic Syntax -----
  OpenParen,
  CloseParen,
  OpenBrace,
  CloseBrace,
  OpenSquare,
  CloseSquare,

  BlockPrefix,
  Comma,
  Colon,
  QuestionMark,
  Dot,
  Semicolon,
  EOF,

  // ----- Literals -----
  Number,
  Identifier,
  PredicateIdentifier,
  String,

  // ----- Operators -----
  NormalAssignmentOperator,
  AdditiveOperator(AdditiveTokenType),
  MultiplicativeOperator(MultiplicativeTokenType),
  ComparisonTokenType(ComparisonTokenType),
  LogicalOperator(LogicalTokenType),
  AssignmentOperator,
  UnaryOperator(UnaryOperator),
  UnaryRightOperator(UnaryRightOperator),

  // ----- Keywords -----
  Is,
  In,
  Typeof,

  For,
  While,
  Until,
  Loop,

  Let,

  Function,
  Pure,
  Where,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AdditiveTokenType {
  Plus,
  Minus,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MultiplicativeTokenType {
  Multiply,
  Divide,
  IntegerDivide,
  Modulo,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ComparisonTokenType {
  Equals,
  NotEquals,
  GreaterThan,
  GreaterThanOrEquals,
  LessThan,
  LessThanOrEquals,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LogicalTokenType {
  Or,
  And,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnaryOperator {
  Not,
  Reference,
  Dereference,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnaryRightOperator {}

#[derive(Clone, Debug)]
pub struct Token {
  pub value: String,
  pub token_type: TokenType,
  pub location: Location,
}

impl fmt::Display for TokenType {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", super::lexer::get_token(&self))
  }
}

impl fmt::Display for ComparisonTokenType {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(
      f,
      "{}",
      super::lexer::get_token(&TokenType::ComparisonTokenType(*self))
    )
  }
}
