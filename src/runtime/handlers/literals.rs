use std::cell::Ref;

use crate::{
  errors,
  parser::nodes::{ArrayLiteral, Identifier, NumericLiteral, StringLiteral},
  runtime::{
    interpreter::interpret,
    scope::Scope,
    values::{Array, ArrayContainer, Number, RuntimeValue, StringValue},
  },
};

pub fn numeric_literal(
  literal: NumericLiteral,
  _: &Scope,
) -> Result<RuntimeValue, errors::ZephyrError> {
  Ok(RuntimeValue::Number(Number {
    value: literal.value,
  }))
}

pub fn identifier(
  ident: Identifier,
  scope: &mut Scope,
) -> Result<RuntimeValue, errors::ZephyrError> {
  let variable = scope.get_variable(&ident.symbol);
  match variable {
    Ok(ok) => match ok {
      RuntimeValue::Function(mut func) => super::special::block(*func.body, &mut (*func.scope)),
      _ => Ok(ok),
    },
    Err(err) => Err(err),
  }
}

pub fn string_literal(
  literal: StringLiteral,
  _: &Scope,
) -> Result<RuntimeValue, errors::ZephyrError> {
}

pub fn array_literal(
  literal: ArrayLiteral,
  scope: &mut Scope,
) -> Result<RuntimeValue, errors::ZephyrError> {
  // Create the array
  let mut array = Array { items: vec![] };

  for i in literal.items {
    array.items.push(Box::new(interpret(*i, scope)?))
  }

  // Add to memory
  let address = scope
    .memory
    .borrow_mut()
    .add_value(RuntimeValue::Array(array));

  // Finish
  Ok(RuntimeValue::ArrayContainer(ArrayContainer {
    location: address,
  }))
}
