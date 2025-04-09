use std::{
    collections::HashMap,
    mem::{discriminant, Discriminant},
};

use either::Either::{Left, Right};
use nodes::{
    DeclareType, ExposeType, InterruptType, MatchCase, MatchCaseType, Node, TaggedSymbol, UnaryType,
};

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::{self, Token, TokenType, Unary, NO_LOCATION},
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
            .first()
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
        let mut nodes: Vec<Node> = vec![];

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
            nodes.push(self.statement()?);
        } else {
            while !self.tokens.is_empty()
                && !matches!(self.at().t, TokenType::CloseBrace)
                && !matches!(self.at().t, TokenType::Eof)
            {
                nodes.push(self.statement()?);

                if discriminant(&TokenType::Semicolon) == discriminant(&self.at().t) {
                    self.eat();
                }
            }
        }

        if !no_brace && !uses_arrow {
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
            } else if let Some(t) = nodes.first() {
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
            TokenType::Enum => self.enum_stmt(),
            TokenType::Function => self.function(true),
            TokenType::Debug => Ok(Node::Debug(nodes::DebugNode {
                location: self.eat().location.clone(),
                node: Box::from(self.expression()?),
            })),
            TokenType::Export => self.export(),
            TokenType::Import => self.import(),
            TokenType::While => self.while_stmt(),
            TokenType::For => self.for_loop(),
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

    pub fn enum_stmt(&mut self) -> NR {
        let token = self.eat();
        let name = Parser::make_symbol(self.expect(
            discriminant(&TokenType::Symbol),
            ZephyrError {
                message: "Expected symbol for enum name".to_string(),
                code: ErrorCode::UnexpectedToken,
                location: Some(self.at().location.clone()),
            },
        )?);

        self.expect(
            discriminant(&TokenType::OpenBrace),
            ZephyrError {
                message: "Expected open brace".to_string(),
                code: ErrorCode::UnexpectedToken,
                location: Some(self.at().location.clone()),
            },
        )?;

        let mut items: Vec<(nodes::Symbol, String)> = vec![];

        while !matches!(self.at().t, TokenType::CloseBrace) {
            let symbol = Parser::make_symbol(self.expect(
                discriminant(&TokenType::Symbol),
                ZephyrError {
                    message: "Expected symbol for enum item".to_string(),
                    code: ErrorCode::UnexpectedToken,
                    location: Some(self.at().location.clone()),
                },
            )?);

            let symbol_clone = symbol.clone();
            items.push((
                symbol,
                format!(
                    "{}.{}__{}",
                    name.value,
                    symbol_clone.value,
                    uuid::Uuid::new_v4()
                ),
            ));

            if matches!(self.at().t, TokenType::Comma) {
                self.eat();
            } else {
                break;
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

        Ok(Node::Enum(nodes::Enum {
            name,
            values: items,
            location: token.location,
        }))
    }

    /*pub fn when(&mut self) -> NR {
        let token = self.eat();
        let emitter = self.expression()?;

        self.expect(
            discriminant(&TokenType::Emits),
            ZephyrError {
                message: "Expected emits keyword".to_string(),
                code: ErrorCode::UnexpectedToken,
                location: Some(self.at().location.clone()),
            },
        )?;

        let message = self.expression()?;

        let once = if matches!(self.at().t, TokenType::Once) {
            self.eat();
            true
        } else {
            false
        };

        self.expect(
            discriminant(&TokenType::Do),
            ZephyrError {
                message: "Expected do keyword".to_string(),
                code: ErrorCode::UnexpectedToken,
                location: Some(self.at().location.clone()),
            },
        )?;

        let func = self.expression()?;

        Ok(Node::WhenClause(nodes::WhenClause {
            emitter: Box::from(emitter),
            message: Box::from(message),
            once,
            func: Box::from(func),
            location: token.location,
        }))
    }*/

    pub fn import(&mut self) -> NR {
        let token = self.eat();
        let import = self.expect(
            discriminant(&TokenType::String),
            ZephyrError {
                message: "Expected string containing import location".to_string(),
                code: ErrorCode::UnexpectedToken,
                location: Some(self.at().location.clone()),
            },
        )?;
        let mut expose: Vec<ExposeType> = vec![];

        if matches!(self.at().t, TokenType::Expose) {
            self.eat();
            loop {
                let mut expr = match self.at().t {
                    TokenType::Symbol => ExposeType::Identifier(self.eat().value),
                    TokenType::Multiplicative(tokens::Multiplicative::Multiply) => {
                        ExposeType::Star()
                    }
                    _ => {
                        return Err(ZephyrError {
                            message: "Cannot use this token here".to_string(),
                            code: ErrorCode::UnexpectedToken,
                            location: Some(self.at().location.clone()),
                        })
                    }
                };

                if matches!(self.at().t, TokenType::As) {
                    self.eat();
                    let ident = self.expect(
                        discriminant(&TokenType::Symbol),
                        ZephyrError {
                            message: "Expected identifier".to_string(),
                            code: ErrorCode::UnexpectedToken,
                            location: Some(self.at().location.clone()),
                        },
                    )?;

                    expr = match expr {
                        ExposeType::Identifier(i) => ExposeType::IdentifierAs(i, ident.value),
                        ExposeType::Star() => ExposeType::StarAs(ident.value),
                        _ => unreachable!(),
                    };
                }

                expose.push(expr);

                if matches!(self.at().t, TokenType::Comma) {
                    self.eat();
                    continue;
                } else {
                    break;
                }
            }
        }

        Ok(Node::Import(nodes::Import {
            import: import.value,
            exposing: expose,
            location: token.location,
        }))
    }

    pub fn export(&mut self) -> NR {
        let token = self.eat();
        let node = self.statement()?;

        let t = match node {
            Node::Symbol(v) => nodes::ExportType::Symbol(v),
            Node::Declare(v) => nodes::ExportType::Declaration(v),
            _ => {
                return Err(ZephyrError {
                    message: "Cannot export this statement".to_string(),
                    code: ErrorCode::UnexpectedToken,
                    location: Some(self.at().location.clone()),
                })
            }
        };

        let export_as = if matches!(self.at().t, TokenType::As) {
            self.eat();
            Some(
                self.expect(
                    discriminant(&TokenType::Symbol),
                    ZephyrError {
                        message: "Expected symbol".to_string(),
                        code: ErrorCode::UnexpectedToken,
                        location: Some(self.at().location.clone()),
                    },
                )?
                .value,
            )
        } else {
            None
        };

        Ok(Node::Export(nodes::Export {
            export: t,
            location: token.location,
            export_as,
        }))
    }

    pub fn declare(&mut self) -> NR {
        let token = self.eat();
        let is_const = !matches!(token.t, TokenType::Let);

        let symbol = match self.at().t {
            TokenType::Symbol => DeclareType::Symbol(Parser::make_symbol(self.eat())),
            TokenType::OpenSquare => {
                let start = self.eat();
                let mut names: Vec<nodes::Symbol> = vec![];

                loop {
                    let name = self.expect(
                        discriminant(&TokenType::Symbol),
                        ZephyrError {
                            message: "Expected symbol".to_string(),
                            code: ErrorCode::UnexpectedToken,
                            location: Some(self.at().location.clone()),
                        },
                    )?;

                    names.push(Parser::make_symbol(name));

                    if matches!(self.at().t, TokenType::Comma) {
                        self.eat();
                        continue;
                    } else {
                        break;
                    }
                }

                self.expect(
                    discriminant(&TokenType::CloseSquare),
                    ZephyrError {
                        message: "Expected close square paran".to_string(),
                        code: ErrorCode::UnexpectedToken,
                        location: Some(self.at().location.clone()),
                    },
                )?;

                DeclareType::Array(names)
            }
            _ => {
                return Err(ZephyrError {
                    message: "Cannot assign to this".to_string(),
                    code: ErrorCode::UnexpectedToken,
                    location: Some(self.at().location.clone()),
                })
            }
        };

        if let TokenType::Assign = self.at().t {
            let assign = self.eat();
            let value = self.expression()?;

            Ok(Node::Declare(nodes::Declare {
                assignee: symbol,
                location: assign.location,
                value: Some(Box::from(value)),
                is_const,
            }))
        } else {
            Ok(Node::Declare(nodes::Declare {
                assignee: symbol,
                location: token.location,
                value: None,
                is_const,
            }))
        }
    }

    pub fn for_loop(&mut self) -> NR {
        let token = self.eat();
        let index_symbol = Parser::make_symbol(self.expect(
            discriminant(&TokenType::Symbol),
            ZephyrError {
                message: "Expected symbol for the index".to_string(),
                code: ErrorCode::UnexpectedToken,
                location: Some(self.at().location.clone()),
            },
        )?);

        let value_symbol = if matches!(self.at().t, TokenType::Comma) {
            self.eat();
            Some(Parser::make_symbol(self.expect(
                discriminant(&TokenType::Symbol),
                ZephyrError {
                    message: "Expected symbol for the value".to_string(),
                    code: ErrorCode::UnexpectedToken,
                    location: Some(self.at().location.clone()),
                },
            )?))
        } else {
            None
        };

        self.expect(
            discriminant(&TokenType::In),
            ZephyrError {
                message: "Expected \"in\"".to_string(),
                code: ErrorCode::UnexpectedToken,
                location: Some(self.at().location.clone()),
            },
        )?;

        let iterator = self.expression()?;

        let block = self.block(false)?;

        Ok(Node::For(nodes::For {
            value_symbol,
            index_symbol,
            location: token.location,
            block: Box::from(block),
            iterator: Box::from(iterator),
        }))
    }

    pub fn function(&mut self, is_statement: bool) -> NR {
        let token = self.eat();

        let name = {
            if let TokenType::Symbol = self.at().t {
                Some(self.eat())
            } else if is_statement {
                return Err(ZephyrError {
                    code: ErrorCode::UnexpectedToken,
                    message: "Expected name for function".to_string(),
                    location: Some(self.at().location.clone()),
                });
            } else {
                None
            }
        };

        let mut arguments: Vec<nodes::Symbol> = vec![];

        if let TokenType::OpenParan = self.at().t {
            self.eat();
            while !matches!(self.at().t, TokenType::CloseParan) {
                arguments.push(Parser::make_symbol(self.expect(
                    discriminant(&TokenType::Symbol),
                    ZephyrError {
                        code: ErrorCode::UnexpectedToken,
                        message: "Expected symbol".to_string(),
                        location: Some(self.at().location.clone()),
                    },
                )?));

                if matches!(self.at().t, TokenType::Comma) {
                    self.eat();
                } else {
                    break;
                }
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
                assignee: nodes::DeclareType::Symbol(Parser::make_symbol(name.unwrap())),
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

    pub fn expression(&mut self) -> NR {
        self.assign()
    }

    pub fn assign(&mut self) -> NR {
        let left = self.is()?;

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

    pub fn is(&mut self) -> NR {
        let left = self.comparison()?;

        if matches!(self.at().t, TokenType::Is) {
            let token = self.eat();
            let right = self.expression()?;

            return Ok(Node::Is(nodes::Is {
                left: Box::from(left),
                right: nodes::IsType::Basic(Box::from(right)),
                r#as: None,
                location: token.location,
            }));
        }

        Ok(left)
    }

    pub fn comparison(&mut self) -> NR {
        let mut left = self.range()?;

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

    pub fn range(&mut self) -> NR {
        // expr as Some(expr) or .. as None
        let left = if matches!(self.at().t, TokenType::Range)
            || matches!(self.at().t, TokenType::RangeInclusive)
        {
            Right(self.eat())
        } else {
            Left(self.additive()?)
        };

        // ..
        // expr..
        // expr..=
        if matches!(left, Right(_))
            || matches!(self.at().t, TokenType::Range)
            || matches!(self.at().t, TokenType::RangeInclusive)
        {
            // Right = alread taken
            // Left = need to take
            let mut range_token = match left {
                Right(ref v) => v.clone(),
                Left(_) => self.eat(),
            };

            // .. = 0..
            // expr..(=) = expr..(=)
            let actual_left = match left {
                Right(ref v) => Node::Number(nodes::Number {
                    value: 0f64,
                    location: v.location.clone(),
                }),
                Left(ref v) => v.clone(),
            };

            // ..(=)(identifier | number | open_paran | unary)
            // expr = ..(=)expr
            let actual_right = if matches!(self.at().t, TokenType::Symbol)
                || matches!(self.at().t, TokenType::Number)
                || matches!(self.at().t, TokenType::Unary(_))
                || matches!(self.at().t, TokenType::OpenParan)
            {
                self.additive()?
            } else {
                range_token.t = TokenType::RangeInclusive;
                Node::Number(nodes::Number {
                    value: -1f64,
                    location: range_token.location.clone(),
                })
            };

            // range:step
            let step = if matches!(self.at().t, TokenType::Colon) {
                self.eat();
                Some(self.additive()?)
            } else {
                None
            };

            // done
            return Ok(Node::Range(nodes::Range {
                start: Box::from(actual_left),
                end: Box::from(actual_right),
                inclusive_end: match range_token.t {
                    TokenType::Range => false,
                    TokenType::RangeInclusive => true,
                    _ => unreachable!(),
                },
                step: step.map(Box::from),
                location: range_token.location.clone(),
            }));
        }

        Ok(left.left().unwrap())
    }

    pub fn additive(&mut self) -> NR {
        let mut left = self.multiplicative()?;

        while let TokenType::Additive(_) = &self.at().t {
            let token = self.eat();
            let right = self.additive()?;
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
        let mut left = self.unary()?;

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

    pub fn unary(&mut self) -> NR {
        if matches!(self.at().t, TokenType::Unary(_))
            || matches!(self.at().t, TokenType::Additive(tokens::Additive::Minus))
            || matches!(self.at().t, TokenType::Additive(tokens::Additive::Plus))
        {
            let token = self.eat();
            let t = match token.t {
                TokenType::Unary(Unary::Length) => UnaryType::LengthOf,
                TokenType::Unary(Unary::Not) => UnaryType::Not,
                TokenType::Unary(Unary::Decrement) => UnaryType::Increment,
                TokenType::Unary(Unary::Increment) => UnaryType::Increment,
                TokenType::Additive(tokens::Additive::Plus) => UnaryType::Plus,
                TokenType::Additive(tokens::Additive::Minus) => UnaryType::Minus,
                _ => unreachable!(),
            };

            let value = self.unary()?;

            return Ok(Node::Unary(nodes::Unary {
                t,
                value: Box::from(value),
                is_right: false,
                location: token.location,
            }));
        }

        let value = self.call_member_new()?;

        if matches!(self.at().t, TokenType::Unary(_)) {
            let token = self.eat();
            let t = match token.t {
                TokenType::Unary(Unary::Decrement) => UnaryType::Increment,
                TokenType::Unary(Unary::Increment) => UnaryType::Increment,
                _ => unreachable!(),
            };

            let value = self.call_member_new()?;

            return Ok(Node::Unary(nodes::Unary {
                t,
                value: Box::from(value),
                is_right: true,
                location: token.location,
            }));
        }

        Ok(value)
    }

    // Possiblities:
    // symbol.symbol2         => MEMBER: left = symbol, right = symbol2
    // symbol[symbol2]        => MEMBER: left = symbol, right = symbol2, computed = true
    // symbol?.symbol2        => MEMBER: left = symbol, right = symbol2, optional = true
    // symbol?.[symbol2]      => MEMBER: left = symbol, right = symbol2, optional = true, computed = true
    // symbol()               => CALL: left = symbol, args = []
    // symbol().symbol2       => MEMBER: left = (CALL: left = symbol, args = []), right = symbol2
    // symbol()?.symbol2      => MEMBER: left = (CALL: left = symbol, args = []), right = symbol2, optional = true
    // symbol()[symbol2]      => MEMBER: left = (CALL: left = symbol, args = []), right = symbol2, computed = true
    // symbol()?.[symbol2]    => MEMBER: left = (CALL: left = symbol, args = []), right = symbol2, optional = true, computed = true
    // [any of the above]!
    // [any of the above].^

    pub fn call_member_new(&mut self) -> NR {
        let mut left = self.literal()?;

        let check = |x: Token| -> bool {
            matches!(x.t, TokenType::Dot)
                || matches!(x.t, TokenType::OpenSquare)
                || matches!(x.t, TokenType::DotOptional)
                || matches!(x.t, TokenType::OpenParan)
                || matches!(x.t, TokenType::OpenSquare)
        };

        while check(self.at().clone()) {
            // Check for left()
            if matches!(self.at().t, TokenType::OpenParan) {
                let token = self.eat();
                let mut arguments: Vec<Node> = vec![];

                // Collect arguments
                while !matches!(self.at().t, TokenType::CloseParan)
                    && !matches!(self.at().t, TokenType::Eof)
                {
                    arguments.push(self.expression()?);
                    if let TokenType::Comma = self.at().t {
                        self.eat();
                    } else {
                        break;
                    }
                }

                // Expect )
                self.expect(
                    discriminant(&TokenType::CloseParan),
                    ZephyrError {
                        code: ErrorCode::UnexpectedToken,
                        message: "Expected closing paran for call".to_string(),
                        location: Some(token.location.clone()),
                    },
                )?;

                // left = left(args)
                left = Node::Call(nodes::Call {
                    left: Box::from(left),
                    args: arguments,
                    location: token.location,
                });
            } else {
                // Member
                let (optional, optional_token) = if matches!(self.at().t, TokenType::DotOptional) {
                    (true, Some(self.eat()))
                } else {
                    (false, None)
                };

                let (computed, token) = if matches!(self.at().t, TokenType::OpenSquare) {
                    (true, self.eat())
                } else {
                    (
                        false,
                        if optional {
                            optional_token.unwrap()
                        } else {
                            self.eat()
                        },
                    )
                };

                let right = if computed {
                    self.expression()?
                } else {
                    match self.at().t {
                        TokenType::Symbol => Node::Symbol(Parser::make_symbol(self.eat())),
                        _ => {
                            return Err(ZephyrError {
                                message: "Expected symbol for member expression".to_string(),
                                code: ErrorCode::UnexpectedToken,
                                location: Some(self.at().location.clone()),
                            })
                        }
                    }
                };

                if computed {
                    self.expect(
                        discriminant(&TokenType::CloseSquare),
                        ZephyrError {
                            code: ErrorCode::UnexpectedToken,
                            message: "Expected closing of computed key".to_string(),
                            location: Some(self.at().location.clone()),
                        },
                    )?;
                }

                left = Node::Member(nodes::Member {
                    left: Box::from(left),
                    right: Box::from(right),
                    optional,
                    computed,
                    location: token.location,
                });
            }

            // Check for !
            if matches!(self.at().t, TokenType::Unary(Unary::Not)) {
                let token = self.eat();
                left = Node::EncapsulateError(nodes::EncapsulateError {
                    left: Box::from(left),
                    location: token.location,
                });
            }

            // Check for .^
            if matches!(self.at().t, TokenType::ShortCircuit) {
                let token = self.eat();
                left = Node::PropogateError(nodes::PropogateError {
                    left: Box::from(left),
                    location: token.location,
                });
            }
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
                let mut items: Vec<Node> = vec![];

                while !matches!(self.at().t, TokenType::Eof)
                    && !matches!(self.at().t, TokenType::CloseSquare)
                {
                    items.push(self.expression()?);
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

                while !matches!(self.at().t, TokenType::Eof)
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
