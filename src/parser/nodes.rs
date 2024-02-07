use std::collections::HashMap;

use crate::lexer::{
  location::Location,
  token::{ComparisonTokenType, LogicalTokenType, TokenType},
};

#[derive(Debug, Clone)]
pub enum Expression {
  // ----- Literals -----
  NumericLiteral(NumericLiteral),
  StringLiteral(StringLiteral),
  Identifier(Identifier),
  ArrayLiteral(ArrayLiteral),
  ObjectLiteral(ObjectLiteral),

  // ----- Expressions -----
  ArithmeticOperator(ArithmeticExpression),
  ComparisonOperator(ComparisonExpression),
  LogicalExpression(LogicalExpression),
  UnaryExpression(UnaryExpression),
  MemberExpression(MemberExpression),
  CallExpression(CallExpression),
  IsExpression(IsExpression),
  AssignmentExpression(AssignmentExpression),
  TernaryExpression(TernaryExpression),
  BreakStatement(BreakStatement),
  ContinueStatement(ContinueStatement),
  TryExpression(TryExpression),
  SpreadExpression(SpreadExpression),

  // ----- Statement like expressions -----
  ForLoop(ForLoop),
  IfExpression(IfExpression),
  WhileExpression(WhileExpression),

  // ----- Statements -----
  VariableDeclaration(VariableDeclaration),
  FunctionLiteral(FunctionLiteral),
  TypeofExpression(TypeofStatement),

  // ----- Special -----
  Program(Program),
  Block(Block),
  None,
}

impl Expression {
  pub fn get_location(&self) -> Location {
    match self {
      Expression::NumericLiteral(x) => x.location.clone(),
      Expression::ArithmeticOperator(x) => x.location.clone(),
      Expression::StringLiteral(x) => x.location.clone(),
      Expression::Identifier(x) => x.location.clone(),
      Expression::ArrayLiteral(x) => x.location.clone(),
      Expression::ComparisonOperator(x) => x.location.clone(),
      Expression::LogicalExpression(x) => x.location.clone(),
      Expression::UnaryExpression(x) => x.location.clone(),
      Expression::MemberExpression(x) => x.location.clone(),
      Expression::CallExpression(x) => x.location.clone(),
      Expression::IsExpression(x) => x.location.clone(),
      Expression::VariableDeclaration(x) => x.location.clone(),
      Expression::FunctionLiteral(x) => x.location.clone(),
      Expression::TypeofExpression(x) => x.location.clone(),
      Expression::Program(_) => panic!("Cannot get location of program"),
      Expression::Block(x) => x.location.clone(),
      Expression::None => Location::no_location(),
      Expression::ObjectLiteral(x) => x.location.clone(),
      Expression::ForLoop(x) => x.location.clone(),
      Expression::AssignmentExpression(x) => x.location.clone(),
      Expression::TernaryExpression(x) => x.location.clone(),
      Expression::IfExpression(x) => x.location.clone(),
      Expression::WhileExpression(x) => x.location.clone(),
      Expression::BreakStatement(x) => x.location.clone(),
      Expression::ContinueStatement(x) => x.location.clone(),
      Expression::TryExpression(x) => x.location.clone(),
      Expression::SpreadExpression(x) => x.location.clone(),
    }
  }
}

// ----- Special -----
#[derive(Debug, Clone)]
pub struct Program {
  pub nodes: Vec<Box<Expression>>,
}

#[derive(Debug, Clone)]
pub struct Block {
  pub nodes: Vec<Box<Expression>>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct WhereClause {
  pub tests: Vec<Box<Expression>>,
}

// ----- Statements -----
#[derive(Debug, Clone)]
pub struct VariableDeclaration {
  pub identifier: Identifier,
  pub value: Box<Expression>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct TypeofStatement {
  pub value: Box<Expression>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct BreakStatement {
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ContinueStatement {
  pub location: Location,
}

// ----- Expressions -----
#[derive(Debug, Clone)]
pub struct MemberExpression {
  pub left: Box<Expression>,
  pub key: Box<Expression>,
  pub is_computed: bool,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct CallExpression {
  pub left: Box<Expression>,
  pub arguments: Vec<Box<Expression>>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct IsExpression {
  pub left: Box<Expression>,
  pub right: Box<Expression>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ArithmeticExpression {
  pub left: Box<Expression>,
  pub right: Box<Expression>,
  pub operator: TokenType,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ComparisonExpression {
  pub left: Box<Expression>,
  pub right: Box<Expression>,
  pub operator: ComparisonTokenType,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct LogicalExpression {
  pub left: Box<Expression>,
  pub right: Box<Expression>,
  pub operator: LogicalTokenType,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct UnaryExpression {
  pub value: Box<Expression>,
  pub operator: TokenType,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ForLoop {
  pub value_to_iter: Box<Expression>,
  pub identifier: Identifier,
  pub body: Block,
  pub location: Location,
  pub none: Option<Box<Block>>,
}

#[derive(Debug, Clone)]
pub struct AssignmentExpression {
  pub left: Box<Expression>,
  pub right: Box<Expression>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct SpreadExpression {
  pub expression: Box<Expression>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct TernaryExpression {
  pub test: Box<Expression>,
  pub success: Box<Expression>,
  pub alternate: Box<Expression>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct IfExpression {
  pub test: Box<Expression>,
  pub success: Box<Block>,
  pub alternate: Option<Box<Expression>>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct WhileExpression {
  pub test: Box<Expression>,
  pub body: Box<Block>,
  pub none: Option<Box<Block>>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct TryExpression {
  pub main: Box<Block>,
  pub catch: Option<Box<Block>>,
  pub catch_identifier: Option<Identifier>,
  pub finally: Option<Box<Block>>,
  pub location: Location,
}

// ----- Literals -----
#[derive(Debug, Clone)]
pub struct NumericLiteral {
  pub value: f64,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Identifier {
  pub symbol: String,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct StringLiteral {
  pub value: String,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ArrayLiteral {
  pub items: Vec<Box<Expression>>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ObjectLiteral {
  pub items: HashMap<String, Box<Expression>>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct FunctionLiteral {
  pub identifier: Option<Identifier>,
  pub body: Box<Block>,
  pub where_clauses: Box<WhereClause>,
  pub arguments: Vec<Identifier>,
  pub is_pure: bool,
  pub location: Location,
}
