use std::collections::HashMap;

use crate::lexer::tokens::{self, Comparison, Location, TokenType};

#[derive(Debug, Clone)]
pub enum Node {
    Block(Block),
    ExportedBlock(ExportedBlock),

    Number(Number),
    ZString(ZString),
    Symbol(Symbol),
    Array(Array),
    Object(Object),
    Function(Function),

    If(If),
    Match(Match),

    WhileLoop(WhileLoop),
    Interrupt(Interrupt),

    Import(Import),
    Export(Export),

    Arithmetic(Arithmetic),
    Comp(Comp),
    Declare(Declare),
    Assign(Assign),
    Call(Call),
    Member(Member),

    DebugNode(DebugNode),
}

impl Node {
    pub fn location(&self) -> &Location {
        match self {
            Node::Block(v) => &v.location,
            Node::ExportedBlock(v) => &v.location,

            Node::Number(v) => &v.location,
            Node::ZString(v) => &v.location,
            Node::Symbol(v) => &v.location,
            Node::Function(v) => &v.location,
            Node::Array(v) => &v.location,
            Node::Object(v) => &v.location,

            Node::If(v) => &v.location,
            Node::Match(v) => &v.location,

            Node::WhileLoop(v) => &v.location,
            Node::Interrupt(v) => &v.location,

            Node::Import(v) => &v.location,
            Node::Export(v) => &v.location,

            Node::Arithmetic(v) => &v.location,
            Node::Comp(v) => &v.location,
            Node::Declare(v) => &v.location,
            Node::Assign(v) => &v.location,
            Node::Call(v) => &v.location,
            Node::Member(v) => &v.location,
            Node::DebugNode(v) => &v.location,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub nodes: Vec<Box<Node>>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ExportedBlock {
    pub nodes: Vec<Box<Node>>,
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
    pub items: Vec<Box<Node>>,
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
pub struct Call {
    pub left: Box<Node>,
    pub args: Vec<Box<Node>>,
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
