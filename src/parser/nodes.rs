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
  UnaryRightExpression(UnaryRightExpression),
  MemberExpression(MemberExpression),
  CallExpression(CallExpression),
  IsExpression(IsExpression),
  InExpression(InExpression),
  AssignmentExpression(AssignmentExpression),
  TernaryExpression(TernaryExpression),
  TryExpression(TryExpression),
  SpreadExpression(SpreadExpression),
  RangeExpression(RangeExpression),
  PipeExpression(PipeExpression),

  // ----- Statement like expressions -----
  ForLoop(ForLoop),
  IfExpression(IfExpression),
  WhileExpression(WhileExpression),

  // ----- Statements -----
  VariableDeclaration(VariableDeclaration),
  FunctionLiteral(FunctionLiteral),
  TypeofExpression(TypeofStatement),
  ExportStatement(ExportStatement),
  ImportStatement(ImportStatement),
  BreakStatement(BreakStatement),
  ContinueStatement(ContinueStatement),
  ReturnStatement(ReturnStatement),
  AssertStatement(AssertStatement),
  ThrowStatement(ThrowStatement),

  // ----- Special -----
  Program(Program),
  Block(Block),
  None,
}

impl Expression {
  pub fn get_location(&self) -> Location {
    match self {
      Expression::NumericLiteral(x) => x.location,
      Expression::ArithmeticOperator(x) => x.location,
      Expression::StringLiteral(x) => x.location,
      Expression::Identifier(x) => x.location,
      Expression::ArrayLiteral(x) => x.location,
      Expression::ComparisonOperator(x) => x.location,
      Expression::LogicalExpression(x) => x.location,
      Expression::UnaryExpression(x) => x.location,
      Expression::UnaryRightExpression(x) => x.location,
      Expression::MemberExpression(x) => x.location,
      Expression::CallExpression(x) => x.location,
      Expression::IsExpression(x) => x.location,
      Expression::InExpression(x) => x.location,
      Expression::VariableDeclaration(x) => x.location,
      Expression::FunctionLiteral(x) => x.location,
      Expression::TypeofExpression(x) => x.location,
      Expression::Block(x) => x.location,
      Expression::ObjectLiteral(x) => x.location,
      Expression::ForLoop(x) => x.location,
      Expression::AssignmentExpression(x) => x.location,
      Expression::TernaryExpression(x) => x.location,
      Expression::IfExpression(x) => x.location,
      Expression::WhileExpression(x) => x.location,
      Expression::BreakStatement(x) => x.location,
      Expression::ContinueStatement(x) => x.location,
      Expression::ReturnStatement(x) => x.location,
      Expression::TryExpression(x) => x.location,
      Expression::SpreadExpression(x) => x.location,
      Expression::ExportStatement(x) => x.location,
      Expression::ImportStatement(x) => x.location,
      Expression::RangeExpression(x) => x.location,
      Expression::AssertStatement(x) => x.location,
      Expression::ThrowStatement(x) => x.location,
      Expression::PipeExpression(x) => x.location,
      Expression::Program(_) => panic!("Cannot get location of program"),
      Expression::None => Location::no_location(),
    }
  }
}

// ----- Special -----
#[derive(Debug, Clone)]
pub struct Program {
  pub nodes: Vec<Box<Expression>>,
  pub file: Option<String>,
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
  pub name: Option<Identifier>,
}

#[derive(Debug, Clone)]
pub struct ContinueStatement {
  pub location: Location,
  pub name: Option<Identifier>,
}

#[derive(Debug, Clone)]
pub struct ReturnStatement {
  pub value: Option<Box<Expression>>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct AssertStatement {
  pub value: Box<Expression>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ExportStatement {
  pub to_export: Box<Expression>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ImportStatement {
  pub from: StringLiteral,
  // Vec<(what, as)>
  pub import: Vec<(Identifier, Identifier)>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ThrowStatement {
  pub location: Location,
  pub what: Box<Expression>,
  pub is_rethrow: bool,
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
pub struct PipeExpression {
  pub left: Box<Expression>,
  pub right: Box<Expression>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct RangeExpression {
  pub from: Box<Expression>,
  pub to: Box<Expression>,
  pub uninclusive: bool,
  pub step: Option<Box<Expression>>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct IsExpression {
  pub left: Box<Expression>,
  pub right: Box<Expression>,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct InExpression {
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
pub struct UnaryRightExpression {
  pub value: Box<Expression>,
  pub operator: TokenType,
  pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ForLoop {
  pub value_to_iter: Box<Expression>,
  pub index_identifier: Identifier,
  pub value_identifier: Option<Identifier>,
  pub body: Block,
  pub location: Location,
  pub name: Option<Identifier>,
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
  pub name: Option<Identifier>,
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
