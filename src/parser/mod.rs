use std::{
    collections::HashMap,
    mem::{discriminant, Discriminant},
};

use nodes::{InterruptType, MatchCase, MatchCaseType, Node, TaggedSymbol};

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::{self, Token, TokenType, NO_LOCATION},
};

type NR = Result<Node, ZephyrError>;

pub mod nodes;
pub struct Parser {
    pub tokens: Vec<Token>,
    pub _file_name: String,
}

impl Parser {
    pub fn at(&self) -> &Token {
        self.tokens
            .get(0)
            .unwrap_or_else(|| panic!("Tokens is empty, but it is not supposed to be"))
    }

    pub fn eat(&mut self) -> Token {
        self.tokens.remove(0)
    }

    pub fn expect(
        &mut self,
        t: Discriminant<TokenType>,
        mut e: ZephyrError,
    ) -> Result<Token, ZephyrError> {
        if discriminant(&self.at().t) != t {
            e.location = Some(self.at().location.clone());
            return Err(e);
        }
        Ok(self.eat())
    }

    pub fn make_symbol(token: Token) -> nodes::Symbol {
        nodes::Symbol {
            value: token.value,
            location: token.location,
        }
    }

    pub fn new(tokens: Vec<Token>, file_name: String) -> Self {
        Parser {
            tokens,
            _file_name: file_name,
        }
    }

    pub fn produce_ast(&mut self) -> NR {
        self.block(true)
    }

    pub fn block(&mut self, no_brace: bool) -> NR {
        let mut nodes: Vec<Box<Node>> = vec![];

        let mut uses_arrow = false;
        let open_token = if !no_brace {
            if !matches!(self.at().t, TokenType::OpenBrace)
                && !matches!(self.at().t, TokenType::Arrow)
            {
                return Err(ZephyrError {
                    code: ErrorCode::UnexpectedToken,
                    message: format!("Expected {{, but got {}", self.at().value),
                    location: Some(self.at().location.clone()),
                });
            }

            if matches!(self.at().t, TokenType::Arrow) {
                uses_arrow = true;
            }

            Some(self.eat())
        } else {
            None
        };

        if uses_arrow {
            nodes.push(Box::from(self.statement()?));
        } else {
            while self.tokens.len() > 0
                && !matches!(self.at().t, TokenType::CloseBrace)
                && !matches!(self.at().t, TokenType::EOF)
            {
                nodes.push(Box::from(self.statement()?));

                if discriminant(&TokenType::Semicolon) == discriminant(&self.at().t) {
                    self.eat();
                }
            }
        }

        if !no_brace {
            if !uses_arrow {
                self.expect(
                    discriminant(&TokenType::CloseBrace),
                    ZephyrError {
                        code: ErrorCode::UnexpectedToken,
                        message: format!("Expected }}, but got {}", self.at().value),
                        location: Some(self.at().location.clone()),
                    },
                )?;
            }
        }

        Ok(Node::Block(nodes::Block {
            location: if let Some(t) = open_token {
                t.location
            } else if let Some(t) = nodes.get(0) {
                t.location().clone()
            } else {
                NO_LOCATION.clone()
            },
            nodes,
        }))
    }

    pub fn statement(&mut self) -> NR {
        match self.at().t {
            TokenType::Let | TokenType::Const => self.declare(),
            TokenType::Function => self.function(true),
            TokenType::Debug => Ok(Node::DebugNode(nodes::DebugNode {
                location: self.eat().location.clone(),
                node: Box::from(self.expression()?),
            })),
            TokenType::Export => self.export(),
            TokenType::While => self.while_stmt(),
            TokenType::Continue => Ok(Node::Interrupt(nodes::Interrupt {
                location: self.eat().location,
                t: InterruptType::Continue,
            })),
            TokenType::Break => Ok(Node::Interrupt(nodes::Interrupt {
                location: self.eat().location,
                t: InterruptType::Break,
            })),
            TokenType::Return => {
                let token = self.eat();
                let value = if let TokenType::Semicolon = self.at().t {
                    None
                } else {
                    Some(Box::from(self.expression()?))
                };

                Ok(Node::Interrupt(nodes::Interrupt {
                    location: self.eat().location,
                    t: InterruptType::Return(value),
                }))
            }
            _ => self.expression(),
        }
    }

    pub fn expression(&mut self) -> NR {
        self.assign()
    }

    pub fn export(&mut self) -> NR {
        let token = self.eat();
        let node = self.statement()?;

        let t = match node {
            Node::Symbol(v) => nodes::ExportType::Symbol(v),
            _ => todo!(),
        };

        Ok(Node::Export(nodes::Export {
            export: t,
            location: token.location,
        }))
    }

    pub fn declare(&mut self) -> NR {
        let token = self.eat();
        let is_const = !matches!(token.t, TokenType::Let);

        let symbol = self.expect(
            discriminant(&TokenType::Symbol),
            ZephyrError {
                code: ErrorCode::UnexpectedToken,
                message: "Expected name of variable".to_string(),
                location: Some(self.at().location.clone()),
            },
        )?;

        if let TokenType::Assign = self.at().t {
            let assign = self.eat();
            let value = self.expression()?;

            Ok(Node::Declare(nodes::Declare {
                symbol: Parser::make_symbol(symbol),
                location: assign.location,
                value: Some(Box::from(value)),
                is_const,
            }))
        } else {
            Ok(Node::Declare(nodes::Declare {
                symbol: Parser::make_symbol(symbol),
                location: token.location,
                value: None,
                is_const,
            }))
        }
    }

    pub fn function(&mut self, is_statement: bool) -> NR {
        let token = self.eat();

        let name = {
            if let TokenType::Symbol = self.at().t {
                Some(self.eat())
            } else {
                if is_statement {
                    return Err(ZephyrError {
                        code: ErrorCode::UnexpectedToken,
                        message: "Expected name for function".to_string(),
                        location: Some(self.at().location.clone()),
                    });
                } else {
                    None
                }
            }
        };

        let mut arguments: Vec<nodes::Symbol> = vec![];

        if let TokenType::OpenParan = self.at().t {
            self.eat();
            while !matches!(self.at().t, TokenType::EOF)
                && !matches!(self.at().t, TokenType::CloseParan)
            {
                arguments.push(Parser::make_symbol(self.expect(
                    discriminant(&TokenType::Symbol),
                    ZephyrError {
                        code: ErrorCode::UnexpectedToken,
                        message: "Expected symbol".to_string(),
                        location: Some(self.at().location.clone()),
                    },
                )?));
            }

            self.expect(
                discriminant(&TokenType::CloseParan),
                ZephyrError {
                    message: "Expected closing paren".to_string(),
                    code: ErrorCode::UnexpectedToken,
                    location: Some(self.at().location.clone()),
                },
            )?;
        }

        let block = self.block(false)?;

        let function = Node::Function(nodes::Function {
            name: name.as_ref().map(|x| Parser::make_symbol(x.clone())),
            body: match block {
                Node::Block(v) => v,
                _ => unreachable!(),
            },
            args: arguments,

            location: token.location.clone(),
        });

        if is_statement {
            Ok(Node::Declare(nodes::Declare {
                symbol: Parser::make_symbol(name.unwrap()),
                value: Some(Box::from(function)),
                location: token.location,
                is_const: false,
            }))
        } else {
            Ok(function)
        }
    }

    pub fn while_stmt(&mut self) -> NR {
        let token = self.eat();

        let test = self.expression()?;

        let block = self.block(false)?;

        Ok(Node::WhileLoop(nodes::WhileLoop {
            test: Box::from(test),
            body: Box::from(block),
            location: token.location,
        }))
    }

    pub fn if_stmt(&mut self) -> NR {
        let token = self.eat();

        let test = self.expression()?;
        let success = self.block(false)?;

        let alternate = if let TokenType::Else = self.at().t {
            self.eat();
            if let TokenType::If = self.at().t {
                Some(Box::from(self.if_stmt()?))
            } else {
                Some(Box::from(self.block(false)?))
            }
        } else {
            None
        };

        Ok(Node::If(nodes::If {
            test: Box::from(test),
            succss: Box::from(success),
            alternate,
            location: token.location,
        }))
    }

    pub fn match_stmt(&mut self) -> NR {
        let token = self.eat();

        let test = self.expression()?;

        self.expect(
            discriminant(&TokenType::OpenBrace),
            ZephyrError {
                message: "Expected open brace".to_string(),
                code: ErrorCode::UnexpectedToken,
                location: Some(self.at().location.clone()),
            },
        )?;

        let mut cases: Vec<MatchCaseType> = vec![];

        while !matches!(self.at().t, TokenType::CloseBrace) {
            let case: MatchCaseType = if let TokenType::Else = self.at().t.clone() {
                let token = self.eat();
                let block = self.block(false)?;

                MatchCaseType::Else(Box::from(block))
            } else if let TokenType::Comparison(c) = self.at().t.clone() {
                let token = self.eat();

                let value = self.expression()?;
                let block = self.block(false)?;

                MatchCaseType::MatchCase(MatchCase {
                    op: c,
                    value: Box::from(value),
                    success: Box::from(block),
                })
            } else {
                MatchCaseType::MatchCase(MatchCase {
                    op: tokens::Comparison::Eq,
                    value: Box::from(self.expression()?),
                    success: Box::from(self.block(false)?),
                })
            };

            cases.push(case.clone());

            if !matches!(self.at().t, TokenType::Comma) || matches!(case, MatchCaseType::Else(_)) {
                break;
            } else {
                self.eat();
            }
        }

        self.expect(
            discriminant(&TokenType::CloseBrace),
            ZephyrError {
                message: "Expected close brace".to_string(),
                code: ErrorCode::UnexpectedToken,
                location: Some(self.at().location.clone()),
            },
        )?;

        Ok(Node::Match(nodes::Match {
            cases,
            test: Box::from(test),
            location: token.location,
        }))
    }

    pub fn assign(&mut self) -> NR {
        let left = self.additive()?;

        if let TokenType::Assign = self.at().t {
            let token = self.eat();
            let value = self.expression()?;

            return Ok(Node::Assign(nodes::Assign {
                assignee: Box::from(left),
                value: Box::from(value),
                location: token.location,
            }));
        }

        Ok(left)
    }

    pub fn additive(&mut self) -> NR {
        let mut left = self.multiplicative()?;

        while let TokenType::Additive(_) = &self.at().t {
            let token = self.eat();
            let right = self.expression()?;
            left = Node::Arithmetic(nodes::Arithmetic {
                left: Box::from(left),
                right: Box::from(right),
                t: token.t,
                location: token.location,
            })
        }

        Ok(left)
    }

    pub fn multiplicative(&mut self) -> NR {
        let mut left = self.comparison()?;

        while let TokenType::Multiplicative(_) = self.at().t {
            let token = self.eat();
            let right = self.expression()?;
            left = Node::Arithmetic(nodes::Arithmetic {
                left: Box::from(left),
                right: Box::from(right),
                t: token.t,
                location: token.location,
            })
        }

        Ok(left)
    }

    pub fn comparison(&mut self) -> NR {
        let mut left = self.call()?;

        while let TokenType::Comparison(v) = self.at().t.clone() {
            let token = self.eat();
            let right = self.expression()?;
            left = Node::Comp(nodes::Comp {
                left: Box::from(left),
                right: Box::from(right),
                t: v,
                location: token.location,
            })
        }

        Ok(left)
    }

    pub fn call(&mut self) -> NR {
        let mut left = self.member()?;

        while let TokenType::OpenParan = self.at().t {
            let token = self.eat();
            let mut arguments: Vec<Box<Node>> = vec![];

            while !matches!(self.at().t, TokenType::CloseParan)
                && !matches!(self.at().t, TokenType::EOF)
            {
                arguments.push(Box::from(self.expression()?));
                if let TokenType::Comma = self.at().t {
                    self.eat();
                    continue;
                } else {
                    break;
                }
            }

            self.expect(
                discriminant(&TokenType::CloseParan),
                ZephyrError {
                    code: ErrorCode::UnexpectedToken,
                    message: "Expected end of argument list".to_string(),
                    location: Some(token.location.clone()),
                },
            )?;

            left = Node::Call(nodes::Call {
                left: Box::from(left),
                args: arguments,
                location: token.location,
            });
        }

        Ok(left)
    }

    pub fn member(&mut self) -> NR {
        let mut left = self.literal()?;

        while matches!(self.at().t, TokenType::Dot)
            || matches!(self.at().t, TokenType::OpenSquare)
            || (matches!(self.at().t, TokenType::QuestionMark)
                && matches!(self.tokens[1].t, TokenType::Dot))
        {
            let (token, optional, computed) = if let TokenType::QuestionMark = self.at().t {
                self.eat();
                let token = self.eat();
                if let TokenType::OpenSquare = self.at().t {
                    (self.eat(), true, true)
                } else {
                    (token, true, false)
                }
            } else {
                let token = self.eat();
                (
                    token.clone(),
                    false,
                    matches!(token.t, TokenType::OpenSquare),
                )
            };

            let right = if computed {
                self.expression()?
            } else {
                self.call()?
            };

            if computed {
                self.expect(
                    discriminant(&TokenType::CloseSquare),
                    ZephyrError {
                        code: ErrorCode::UnexpectedToken,
                        message: "Expected closing of computed key".to_string(),
                        location: Some(token.location.clone()),
                    },
                )?;
            }

            left = Node::Member(nodes::Member {
                left: Box::from(left),
                right: Box::from(right),
                optional,
                computed,
                location: token.location,
            })
        }

        Ok(left)
    }

    pub fn literal(&mut self) -> NR {
        match self.at().t {
            TokenType::Number => {
                let pre_value = self.eat();
                let value = match pre_value.value.parse::<f64>() {
                    Ok(ok) => ok,
                    Err(err) => {
                        return Err(ZephyrError {
                            code: ErrorCode::InvalidNumber,
                            message: format!("Failed to parse number: {}", err.to_string()),
                            location: Some(pre_value.location),
                        })
                    }
                };
                Ok(Node::Number(nodes::Number {
                    value,
                    location: pre_value.location,
                }))
            }
            TokenType::String => {
                let token = self.eat();
                Ok(Node::ZString(nodes::ZString {
                    value: token.value,
                    location: token.location,
                }))
            }
            TokenType::Symbol => {
                let token = self.eat();
                Ok(Node::Symbol(nodes::Symbol {
                    value: token.value,
                    location: token.location,
                }))
            }
            TokenType::Function => self.function(false),
            TokenType::If => self.if_stmt(),
            TokenType::Match => self.match_stmt(),
            TokenType::OpenSquare => {
                let token = self.eat();
                let mut items: Vec<Box<Node>> = vec![];

                while !matches!(self.at().t, TokenType::EOF)
                    && !matches!(self.at().t, TokenType::CloseSquare)
                {
                    items.push(Box::from(self.expression()?));
                    if let TokenType::Comma = self.at().t {
                        self.eat();
                        continue;
                    } else {
                        break;
                    }
                }

                self.expect(
                    discriminant(&TokenType::CloseSquare),
                    ZephyrError {
                        code: ErrorCode::UnexpectedToken,
                        message: "Expected closing of array".to_string(),
                        location: Some(token.location.clone()),
                    },
                )?;

                Ok(Node::Array(nodes::Array {
                    items,
                    location: token.location.clone(),
                }))
            }
            TokenType::Dot => {
                let token = self.eat();
                self.expect(
                    discriminant(&TokenType::OpenBrace),
                    ZephyrError {
                        code: ErrorCode::UnexpectedToken,
                        message: "Expected start of object literal".to_string(),
                        location: Some(self.at().location.clone()),
                    },
                )?;

                let mut items: HashMap<String, TaggedSymbol> = HashMap::new();

                while !matches!(self.at().t, TokenType::EOF)
                    && !matches!(self.at().t, TokenType::CloseBrace)
                {
                    let identifier = Parser::make_symbol(self.expect(
                        discriminant(&TokenType::Symbol),
                        ZephyrError {
                            code: ErrorCode::UnexpectedToken,
                            message: "Expected identifier".to_string(),
                            location: Some(self.at().location.clone()),
                        },
                    )?);

                    let value = if let TokenType::Colon = self.at().t {
                        self.eat();
                        self.expression()?
                    } else {
                        Node::Symbol(identifier.clone())
                    };

                    items.insert(
                        identifier.value,
                        TaggedSymbol {
                            value: Box::from(value),
                            tags: HashMap::new(),
                        },
                    );

                    if let TokenType::Comma = self.at().t {
                        self.eat();
                        continue;
                    } else {
                        break;
                    }
                }

                self.expect(
                    discriminant(&TokenType::CloseBrace),
                    ZephyrError {
                        code: ErrorCode::UnexpectedToken,
                        message: "Expected closing of object literal".to_string(),
                        location: Some(self.at().location.clone()),
                    },
                )?;

                Ok(Node::Object(nodes::Object {
                    items,
                    location: token.location,
                }))
            }
            TokenType::OpenBrace => self.block(false),
            TokenType::OpenParan => {
                let token = self.eat();
                let value = self.expression()?;
                self.expect(
                    discriminant(&TokenType::CloseParan),
                    ZephyrError {
                        code: ErrorCode::UnexpectedToken,
                        message: format!("Expected closing of group, but got {}", self.at().value),
                        location: Some(token.location.clone()),
                    },
                )?;
                Ok(value)
            }
            _ => {
                return Err(ZephyrError {
                    code: ErrorCode::UnexpectedToken,
                    message: format!("Cannot handle the token {} here", self.at().value),
                    location: Some(self.at().location.clone()),
                })
            }
        }
    }
}
