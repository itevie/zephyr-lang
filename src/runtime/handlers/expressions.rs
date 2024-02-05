use crate::{errors::{self, runtime_error, ZephyrError}, lexer::{location::Location, token::{AdditiveTokenType, ComparisonTokenType, LogicalTokenType, MultiplicativeTokenType, TokenType, UnaryOperator}}, parser::nodes::{ArithmeticExpression, ComparisonExpression, Expression, LogicalExpression, MemberExpression, TypeofStatement, UnaryExpression}, runtime::{interpreter::interpret, memory::MemoryAddress, scope::Scope, values::{Boolean, Number, Reference, RuntimeValue, StringValue}}};
use crate::util;

use super::hrerr;

pub fn arithmetic_expression(expr: ArithmeticExpression, scope: &mut Scope) -> Result<RuntimeValue, errors::ZephyrError> {
  // Collect values
  let left = 
    match interpret(*expr.left, scope) {
      Ok(val) => val,
      Err(err) => return Err(err),
    };
  let right = 
    match interpret(*expr.right, scope) {
      Ok(val) => val,
      Err(err) => return Err(err),
  };

  // Check if both are numbers
  if util::varient_eq(&left, &right) && matches!(left, RuntimeValue::Number(_)) {
    // Convert to numbers
    let left_number = match left { RuntimeValue::Number(val) => val, _ => panic!("") };
    let right_number = match right { RuntimeValue::Number(val) => val, _ => panic!("") };
    let value: f64 = match expr.operator {
      TokenType::AdditiveOperator(AdditiveTokenType::Plus) => left_number.value + right_number.value,
      TokenType::AdditiveOperator(AdditiveTokenType::Minus)=> left_number.value - right_number.value,
      TokenType::MultiplicativeOperator(MultiplicativeTokenType::Multiply) => left_number.value * right_number.value,
      TokenType::MultiplicativeOperator(MultiplicativeTokenType::Divide) => left_number.value / right_number.value,
      TokenType::MultiplicativeOperator(MultiplicativeTokenType::IntegerDivide) => (left_number.value as i64 / right_number.value as i64) as f64,
      TokenType::MultiplicativeOperator(MultiplicativeTokenType::Modulo) => left_number.value % right_number.value,
      _ => unreachable!(),
    };

    return Ok(RuntimeValue::Number(Number {
      value,
    }));
  }

  // Try others
  let result: Option<RuntimeValue> = match left {
    RuntimeValue::StringValue(ref left_value) => {
      let right_value: Option<String> = match right {
        RuntimeValue::StringValue(ref string_value) => Some(String::from(&*string_value.value)),
        RuntimeValue::Number(ref number_value) => Some(number_value.value.to_string()),
        RuntimeValue::Boolean(ref bool_value) => Some(bool_value.value.to_string()),
        RuntimeValue::Null(_) => Some("null".to_string()),
        RuntimeValue::Reference(ref refer) => Some(refer.value.to_string()),
        _ => return Err(ZephyrError::runtime(format!("Cannot coerce a {} to a string", right.type_name()), Location::no_location()))
      };

      match right_value {
        Some(val) => Some(RuntimeValue::StringValue(StringValue { 
          value: String::from(&*left_value.value) + &*val
        })),
        None => None,
      }
    }
    _ => None,
  };

  match result {
    Some(val) => Ok(val),
    None => Err(errors::ZephyrError::runtime(
      format!("Cannot handle {} {:?} {}", left.type_name(), expr.operator, right.clone().type_name()),
      Location::no_location()
    ))
  }
}

pub fn logical_expression(expr: LogicalExpression, scope: &mut Scope) -> Result<RuntimeValue, errors::ZephyrError> {
  // Collect values
  let left = 
    match interpret(*expr.left, scope) {
      Ok(val) => val,
      Err(err) => return Err(err),
    };
  let right = 
    match interpret(*expr.right, scope) {
      Ok(val) => val,
      Err(err) => return Err(err),
    };
  
  Ok(RuntimeValue::Boolean(Boolean {
    value: match expr.operator {
      LogicalTokenType::And => left.is_truthy() && right.is_truthy(),
      LogicalTokenType::Or => left.is_truthy() || right.is_truthy(),
    }
  }))
}

pub fn comparison_expression(expr: ComparisonExpression, scope: &mut Scope) -> Result<RuntimeValue, errors::ZephyrError> {
  // Collect values
  let left = 
    match interpret(*expr.left, scope) {
      Ok(val) => val,
      Err(err) => return Err(err),
    };
  let right = 
    match interpret(*expr.right, scope) {
      Ok(val) => val,
      Err(err) => return Err(err),
    };

  // Check if they are the same type
  if !util::varient_eq(&left, &right) {
    return Ok(RuntimeValue::Boolean(Boolean { value: false }));
  }

  let mut result = false;

  // Numbers
  if matches!(left, RuntimeValue::Number(_)) {
    let left_number = match left { RuntimeValue::Number(num) => num.value, _ => unreachable!()};
    let right_number = match right { RuntimeValue::Number(num) => num.value, _ => unreachable!()};
  
    result = match expr.operator {
      ComparisonTokenType::Equals => left_number == right_number,
      ComparisonTokenType::GreaterThan => left_number > right_number,
      ComparisonTokenType::GreaterThanOrEquals => left_number >= right_number,
      ComparisonTokenType::LessThan => left_number < right_number,
      ComparisonTokenType::LessThanOrEquals => left_number <= right_number,
    };
  } 
  
  // Booleans
  else if matches!(left, RuntimeValue::Boolean(_)) {
    let left_bool = match left { RuntimeValue::Boolean(bool) => bool, _ => unreachable!()};
    let right_bool = match right { RuntimeValue::Boolean(bool) => bool, _ => unreachable!()};
  
    if left_bool.value == right_bool.value {
      result = true;
    }
  } 
  
  // Strings
  else if matches!(left, RuntimeValue::StringValue(_)) {
    let left_bool = match left { RuntimeValue::StringValue(string) => string, _ => unreachable!()};
    let right_bool = match right { RuntimeValue::StringValue(string) => string, _ => unreachable!()};
  
    if left_bool.value == right_bool.value {
      result = true;
    }
  }

  // Null
  else if matches!(left, RuntimeValue::Null(_)) {
    // This will always be true
    result = true;
  }

  return Ok(RuntimeValue::Boolean(Boolean { value: result }));
}

pub fn typeof_statement(stmt: TypeofStatement, scope: &mut Scope) -> Result<RuntimeValue, errors::ZephyrError> {
  Ok(RuntimeValue::StringValue(StringValue {
    value: match interpret(*stmt.value, scope) {
      Ok(val) => val.type_name().to_string(),
      Err(err) => return Err(err),
    }
  }))
}

pub fn member_expression(expr: MemberExpression, scope: &mut Scope) -> Result<RuntimeValue, errors::ZephyrError> {
  let value = interpret(*expr.left, scope)?;

  // Check if it is computed
  if expr.is_computed {
    // Get key
    let key = interpret(*expr.key, scope)?;

    match value {
      RuntimeValue::ArrayContainer(arr_ref) => {
        // Get the referenced array
        let arr = match scope.memory.borrow().get_value(arr_ref.location)? {
          RuntimeValue::Array(arr) => arr,
          _ => unreachable!(),
        };

        // Can only index via numbers
        let number = match key {
          RuntimeValue::Number(num) => num.value as usize,
          _ => return Err(errors::ZephyrError::runtime(
            format!("Can only index array with numbers, but got {}", key.type_name()),
            Location::no_location(),
          ))
        };

        // Check if out of bounds
        if arr.items.len() < number {
          return Err(runtime_error!(
            "Index out of bounds".to_string()
          ));
        }

        // Return
        return Ok(*(*&arr.items[number]).clone());
      },
      _ => return Err(errors::ZephyrError::runtime(
        format!("Cannot index a {}", value.type_name()),
        Location::no_location(),
      ))
    } 
  } else {
    unimplemented!();
  }
}

pub fn unary_expression(expr: UnaryExpression, scope: &mut Scope) -> Result<RuntimeValue, errors::ZephyrError> {
  let expr_value = *expr.value;
  let value = hrerr!(interpret(expr_value.clone(), scope));
  let operator = expr.operator;

  match operator {
    TokenType::UnaryOperator(UnaryOperator::Not) => Ok(RuntimeValue::Boolean(
      Boolean { value: !value.is_truthy() }
    )),
    TokenType::UnaryOperator(UnaryOperator::Dereference) => {
      match value {
        RuntimeValue::Reference(refer) => scope.memory.borrow().get_value(refer.value),
        RuntimeValue::Number(num) => scope.memory.borrow().get_value(num.value as MemoryAddress),
        RuntimeValue::ArrayContainer(arr) => scope.memory.borrow().get_value(arr.location),
        _ => Err(ZephyrError::runtime(format!("Cannot derference a {:?}", value.type_name()), Location::no_location())),
      }
    },
    TokenType::UnaryOperator(UnaryOperator::Reference) => Ok(RuntimeValue::Reference(
      Reference { value: match expr_value.clone() {
        Expression::Identifier(ident) => {
          match scope.get_variable_address(&ident.symbol) {
            Ok(val) => val,
            Err(err) => return Err(err),
          }
        },
        Expression::NumericLiteral(ident) => {
          ident.value as MemoryAddress
        }
        _ => return Err(ZephyrError::runtime(format!("Cannot reference this"), Location::no_location()))
      }}
    )),
    _ => unimplemented!()
  }
}