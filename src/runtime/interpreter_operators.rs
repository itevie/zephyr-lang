use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::{self, TokenType},
    parser::nodes::{self, UnaryType},
};

use super::{
    values::{self, RuntimeValue},
    Interpreter, R,
};

impl Interpreter {
    pub fn run_arithmetic(&mut self, expr: nodes::Arithmetic) -> R {
        let left = self.run(*expr.left)?;
        let right = self.run(*expr.right)?;

        if let (RuntimeValue::Number(left_number), RuntimeValue::Number(right_number)) =
            (&left, &right)
        {
            return Ok(values::Number::new(match expr.t {
                TokenType::Additive(tokens::Additive::Plus) => {
                    left_number.value + right_number.value
                }
                TokenType::Additive(tokens::Additive::Minus) => {
                    left_number.value - right_number.value
                }
                TokenType::Multiplicative(tokens::Multiplicative::Divide) => {
                    left_number.value / right_number.value
                }
                TokenType::Multiplicative(tokens::Multiplicative::Multiply) => {
                    left_number.value * right_number.value
                }
                TokenType::Multiplicative(tokens::Multiplicative::Exponent) => {
                    left_number.value.powf(right_number.value)
                }
                TokenType::Multiplicative(tokens::Multiplicative::Modulo) => {
                    left_number.value % right_number.value
                }
                _ => unreachable!(),
            }));
        }

        let result = match left {
            // string ? *
            RuntimeValue::ZString(ref left_value) => match expr.t {
                // string + *
                TokenType::Additive(tokens::Additive::Plus) => Some(values::ZString::new(
                    left_value.value.clone() + &right.to_string()?,
                )),
                _ => None,
            },
            _ => None,
        };

        match result {
            Some(ok) => Ok(ok),
            None => Err(ZephyrError {
                code: ErrorCode::InvalidOperation,
                message: format!(
                    "Cannot handle {} {:?} {}",
                    left.type_name(),
                    expr.t,
                    right.type_name()
                ),
                location: Some(expr.location),
            }),
        }
    }

    pub fn run_comp(&mut self, expr: nodes::Comp) -> R {
        let left = self.run(*expr.left)?;
        let right = self.run(*expr.right)?;

        Ok(values::Boolean::new(left.compare_with(
            right,
            expr.t,
            Some(expr.location),
        )?))
    }

    pub fn run_unary(&mut self, expr: nodes::Unary) -> R {
        let left = self.run(*expr.value)?;

        if !expr.is_right {
            match expr.t {
                UnaryType::LengthOf => Ok(values::Number::new(left.iter()?.len() as f64)),
                UnaryType::Not => Ok(values::Boolean::new(!left.is_truthy())),
                _ => unreachable!(),
            }
        } else {
            match expr.t {
                _ => unreachable!(),
            }
        }
    }
}
