use std::{
    collections::HashMap,
    mem::{discriminant, Discriminant},
};

use nodes::{Node, Symbol};

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::{Token, TokenType, NO_LOCATION},
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

        let open_token = if !no_brace {
            Some(self.expect(
                discriminant(&TokenType::OpenBrace),
                ZephyrError {
                    code: ErrorCode::UnexpectedToken,
                    message: format!("Expected {{, but got {}", self.at().value),
                    location: Some(self.at().location.clone()),
                },
            )?)
        } else {
            None
        };

        while self.tokens.len() > 0
            && !matches!(self.at().t, TokenType::CloseBrace)
            && !matches!(self.at().t, TokenType::EOF)
        {
            nodes.push(Box::from(self.statement()?));

            if discriminant(&TokenType::Semicolon) == discriminant(&self.at().t) {
                self.eat();
            }
        }

        if !no_brace {
            self.expect(
                discriminant(&TokenType::CloseBrace),
                ZephyrError {
                    code: ErrorCode::UnexpectedToken,
                    message: format!("Expected }}, but got {}", self.at().value),
                    location: Some(self.at().location.clone()),
                },
            )?;
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
            _ => self.expression(),
        }
    }

    pub fn expression(&mut self) -> NR {
        self.assign()
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

        let args: Vec<Symbol> = if let TokenType::OpenParan = self.at().t {
            self.eat();

            let mut args: Vec<Symbol> = vec![];

            while !matches!(self.at().t, TokenType::EOF)
                && !matches!(self.at().t, TokenType::CloseParan)
            {
                args.push(Parser::make_symbol(self.expect(
                    discriminant(&TokenType::Symbol),
                    ZephyrError {
                        code: ErrorCode::UnexpectedToken,
                        message: "Expected identifier".to_string(),
                        location: Some(self.at().location.clone()),
                    },
                )?));

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
                    message: "Expected closing of argument list".to_string(),
                    location: Some(self.at().location.clone()),
                },
            )?;

            args
        } else {
            vec![]
        };

        let block = self.block(false)?;

        let function = Node::Function(nodes::Function {
            name: name.as_ref().map(|x| Parser::make_symbol(x.clone())),
            body: match block {
                Node::Block(v) => v,
                _ => unreachable!(),
            },
            args,
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
        let mut left = self.call()?;

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

                let mut items: HashMap<String, Box<Node>> = HashMap::new();

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

                    items.insert(identifier.value, Box::from(value));

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
