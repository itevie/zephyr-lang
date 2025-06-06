use std::collections::HashMap;

use crate::lexer::tokens::{self, Comparison, Location, TokenType};

#[derive(Debug, Clone)]
pub enum Node {
    Block(Block),
    ExportedBlock(ExportedBlock),

    Assign(Assign),
    Declare(Declare),
    Export(Export),
    For(For),
    If(If),
    Import(Import),
    Interrupt(Interrupt),
    Match(Match),
    WhileLoop(WhileLoop),

    Arithmetic(Arithmetic),
    Call(Call),
    Comp(Comp),
    Debug(DebugNode),
    Enum(Enum),
    Function(Function),
    Is(Is),
    Logical(Logical),
    Member(Member),
    Range(Range),
    Unary(Unary),

    Array(Array),
    Number(Number),
    Symbol(Symbol),
    Object(Object),
    ZString(ZString),
}

impl Node {
    pub fn location(&self) -> &Location {
        match self {
            Node::Block(v) => &v.location,
            Node::ExportedBlock(v) => &v.location,

            Node::Assign(v) => &v.location,
            Node::Declare(v) => &v.location,
            Node::Export(v) => &v.location,
            Node::For(v) => &v.location,
            Node::If(v) => &v.location,
            Node::Import(v) => &v.location,
            Node::Interrupt(v) => &v.location,
            Node::Match(v) => &v.location,
            Node::WhileLoop(v) => &v.location,

            Node::Arithmetic(v) => &v.location,
            Node::Call(v) => &v.location,
            Node::Comp(v) => &v.location,
            Node::Debug(v) => &v.location,
            Node::Enum(v) => &v.location,
            Node::Function(v) => &v.location,
            Node::Is(v) => &v.location,
            Node::Logical(v) => &v.location,
            Node::Member(v) => &v.location,
            Node::Range(v) => &v.location,
            Node::Unary(v) => &v.location,

            Node::Array(v) => &v.location,
            Node::Number(v) => &v.location,
            Node::Object(v) => &v.location,
            Node::Symbol(v) => &v.location,
            Node::ZString(v) => &v.location,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub nodes: Vec<Node>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ExportedBlock {
    pub nodes: Vec<Node>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Number {
    pub value: f64,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ZString {
    pub value: String,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Array {
    pub items: Vec<Node>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct TaggedSymbol {
    pub value: Box<Node>,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Object {
    pub items: HashMap<String, TaggedSymbol>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: Option<Symbol>,
    pub body: Block,
    pub args: Vec<Symbol>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub value: String,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Arithmetic {
    pub left: Box<Node>,
    pub right: Box<Node>,
    pub t: TokenType,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Comp {
    pub left: Box<Node>,
    pub right: Box<Node>,
    pub t: tokens::Comparison,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Logical {
    pub left: Box<Node>,
    pub right: Box<Node>,
    pub t: tokens::Logical,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Call {
    pub left: Box<Node>,
    pub args: Vec<Node>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Member {
    pub left: Box<Node>,
    pub right: Box<Node>,
    pub optional: bool,
    pub computed: bool,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub enum DeclareType {
    Symbol(Symbol),
    Array(Vec<Symbol>),
    Object(HashMap<String, String>),
}

#[derive(Debug, Clone)]
pub struct Declare {
    pub assignee: DeclareType,
    pub value: Option<Box<Node>>,
    pub location: Location,
    pub is_const: bool,
}

#[derive(Debug, Clone)]
pub struct Assign {
    pub assignee: Box<Node>,
    pub value: Box<Node>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct If {
    pub test: Box<Node>,
    pub succss: Box<Node>,
    pub alternate: Option<Box<Node>>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct WhileLoop {
    pub test: Box<Node>,
    pub body: Box<Node>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub enum MatchCaseType {
    MatchCase(MatchCase),
    Else(Box<Node>),
    Is(Box<Node>, IsType, Box<Node>),
}

#[derive(Debug, Clone)]
pub struct MatchCase {
    pub op: Comparison,
    pub value: Box<Node>,
    pub success: Box<Node>,
}

#[derive(Debug, Clone)]
pub struct Match {
    pub test: Box<Node>,
    pub cases: Vec<MatchCaseType>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct DebugNode {
    pub node: Box<Node>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub enum ExportType {
    Symbol(Symbol),
    Declaration(Declare),
    Object(Object),
}

#[derive(Debug, Clone)]
pub struct Export {
    pub export: ExportType,
    pub export_as: Option<String>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub enum InterruptType {
    Continue,
    Break,
    Return(Option<Box<Node>>),
}

#[derive(Debug, Clone)]
pub struct Interrupt {
    pub location: Location,
    pub t: InterruptType,
}

#[derive(Debug, Clone)]
pub enum ExposeType {
    Identifier(String),
    IdentifierAs(String, String),
    Star(),
    StarAs(String),
}

#[derive(Debug, Clone)]
pub struct Import {
    pub import: String,
    pub exposing: Vec<ExposeType>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct For {
    pub value_symbol: Option<Symbol>,
    pub index_symbol: Symbol,
    pub iterator: Box<Node>,
    pub block: Box<Node>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub enum UnaryType {
    Not,
    Plus,
    Minus,
    LengthOf,
    Increment,
    Decrement,
}

#[derive(Debug, Clone)]
pub struct Unary {
    pub t: UnaryType,
    pub value: Box<Node>,
    pub is_right: bool,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Range {
    pub start: Box<Node>,
    pub end: Box<Node>,
    pub inclusive_end: bool,
    pub step: Option<Box<Node>>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub enum IsType {
    Basic(Box<Node>),
    Comparison(tokens::Comparison, Box<Node>),
}

#[derive(Debug, Clone)]
pub struct Is {
    pub left: Box<Node>,
    pub right: IsType,
    pub r#as: Option<Symbol>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Enum {
    pub name: Symbol,
    pub values: Vec<(Symbol, String)>,
    pub location: Location,
}
