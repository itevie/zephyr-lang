use std::{collections::HashMap, rc::Rc};

use crate::{
  errors::{self, runtime_error, ZephyrError},
  lexer::{
    lexer::lex,
    location::Location,
    token::{
      AdditiveTokenType, ComparisonTokenType, LogicalTokenType, MultiplicativeTokenType, TokenType,
      UnaryOperator,
    },
  },
  parser::{
    nodes::{Block, Expression, MemberExpression},
    parser::Parser,
  },
  util,
};

use super::{
  memory::MemoryAddress,
  scope::Scope,
  values::{
    to_array, Array, ArrayContainer, Boolean, Function, Null, Number, Object, ObjectContainer,
    Reference, RuntimeValue, StringValue,
  },
};

#[path = "./handlers/mod.rs"]
pub mod handlers;

pub struct Interpreter {
  pub scope: Rc<Scope>,
  pub global_scope: Rc<Scope>,
}

impl Interpreter {
  pub fn new() -> Self {
    let libs: Vec<&str> = vec![
      include_str!("../lib/predicates.zr"),
      include_str!("../lib/array.zr"),
      include_str!("../lib/string.zr"),
    ];
    let scope = &Rc::new(Scope::new());

    // Load libs
    for i in libs {
      let lib_scope = scope.clone().create_child();
      let mut lib_interpreter = Interpreter {
        scope: lib_scope.clone(),
        global_scope: scope.clone(),
      };
      lib_interpreter
        .evaluate(Expression::Program(
          Parser::new(lex(String::from(i)).unwrap())
            .produce_ast()
            .unwrap(),
        ))
        .unwrap();

      // Add defined variables to global
      for (key, value) in lib_scope.clone().variables.borrow().iter() {
        scope.variables.borrow_mut().insert((key).clone(), *value);
      }
    }

    Interpreter {
      global_scope: scope.clone(),
      scope: scope.create_child(),
    }
  }

  pub fn evaluate_block(
    &mut self,
    block: Block,
    scope: Rc<Scope>,
  ) -> Result<RuntimeValue, ZephyrError> {
    let prev_scope = std::mem::replace(&mut self.scope, scope);
    let mut last_value: Option<RuntimeValue> = None;

    for expr in block.nodes {
      last_value = Some(match self.evaluate(*expr) {
        Ok(val) => val,
        Err(err) => return Err(err),
      });
    }

    let _ = std::mem::replace(&mut self.scope, prev_scope);

    match last_value {
      None => Ok(RuntimeValue::Null(Null {})),
      Some(val) => Ok(val),
    }
  }

  pub fn evaluate_member_expression(
    &mut self,
    expr: MemberExpression,
    assign: &Option<RuntimeValue>,
  ) -> Result<RuntimeValue, errors::ZephyrError> {
    let key_loc = expr.key.get_location();
    let value = self.evaluate(*expr.left)?;

    // Check if it is computed
    if expr.is_computed {
      // Get key
      let key = self.evaluate(*expr.key)?;

      match value {
        RuntimeValue::ArrayContainer(arr_ref) => {
          // Get the referenced array
          let mut arr = match unsafe { crate::MEMORY.get_value(arr_ref.location) }? {
            RuntimeValue::Array(arr) => arr,
            _ => unreachable!(),
          };

          // Can only index via numbers
          let number = match key {
            RuntimeValue::Number(num) => num.value as usize,
            _ => {
              return Err(errors::ZephyrError::runtime(
                format!(
                  "Can only index array with numbers, but got {}",
                  key.type_name()
                ),
                Location::no_location(),
              ))
            }
          };

          // Check if out of bounds
          if arr.items.len() <= number {
            // Check if it is assignment
            if let Some(_) = assign {
              // Only allow +1
              if arr.items.len() + 1 < number {
                return Err(ZephyrError::runtime(
                  "Index out of bounds".to_string(),
                  key_loc.clone(),
                ));
              } else {
                // Add dummy item so rust doesn't panic
                arr.items.push(Box::from(RuntimeValue::Null(Null {})));
              }
            } else {
              return Err(runtime_error!("Index out of bounds".to_string()));
            };
          }

          // Check if should assign
          if let Some(assign_to) = assign.clone() {
            arr.items[number] = Box::from(assign_to);

            // Update it

            return Ok(RuntimeValue::ArrayContainer(ArrayContainer {
              location: unsafe {
                crate::MEMORY.set_value(arr_ref.location, RuntimeValue::Array(arr))?
              },
            }));
          }

          // Return
          return Ok(*(*&arr.items[number]).clone());
        }
        RuntimeValue::ObjectContainer(obj_ref) => {
          let object = match unsafe { crate::MEMORY.get_value(obj_ref.location) }? {
            RuntimeValue::Object(obj) => obj,
            _ => unreachable!(),
          };

          let string_key = match key {
            RuntimeValue::StringValue(num) => num.value,
            _ => {
              return Err(errors::ZephyrError::runtime(
                format!(
                  "Can only index object with strings, but got {}",
                  key.type_name()
                ),
                key_loc.clone(),
              ))
            }
          };

          // Check if object has the item defined
          if object.items.contains_key(&string_key) {
            return Ok((*object.items.get(&string_key).unwrap()).clone());
          } else {
            return Err(errors::ZephyrError::runtime(
              format!("Object does not contain definition for key {}", string_key),
              key_loc.clone(),
            ));
          }
        }
        _ => {
          return Err(errors::ZephyrError::runtime(
            format!("Cannot index a {}", value.type_name()),
            Location::no_location(),
          ))
        }
      }
    } else {
      unimplemented!();
    }
  }

  pub fn evaluate(&mut self, _node: Expression) -> Result<RuntimeValue, errors::ZephyrError> {
    let node = &_node;
    let result = match node.clone() {
      /////////////////////////////////
      // ----- Special Things ----- //
      ///////////////////////////////
      Expression::Program(program) => {
        let mut last_value: Option<RuntimeValue> = None;

        for expr in program.nodes {
          last_value = Some(match self.evaluate(*expr) {
            Ok(val) => val,
            Err(err) => return Err(err),
          });
        }

        match last_value {
          None => Ok(RuntimeValue::Null(Null {})),
          Some(val) => Ok(val),
        }
      }

      Expression::Block(block) => self.evaluate_block(block, self.scope.create_child()),

      /////////////////////////////
      // ----- Statements ----- //
      ///////////////////////////
      Expression::VariableDeclaration(stmt) => {
        // Collect data
        let name = stmt.identifier.symbol;
        let value = match self.evaluate(*stmt.value) {
          Ok(val) => val,
          Err(err) => return Err(err),
        };

        // Set the variable
        match self.scope.declare_variable(&name, value) {
          Ok(_) => (),
          Err(err) => return Err(err),
        }

        Ok(RuntimeValue::Null(Null {}))
      }

      ///////////////////////////
      // ----- Literals ----- //
      /////////////////////////
      Expression::NumericLiteral(literal) => Ok(RuntimeValue::Number(Number {
        value: literal.value,
      })),
      Expression::Identifier(ident) => {
        let variable = self.scope.get_variable(&ident.symbol);
        match variable {
          Ok(ok) => Ok(ok),
          Err(err) => Err(err),
        }
      }
      Expression::StringLiteral(literal) => Ok(RuntimeValue::StringValue(StringValue {
        value: literal.value,
      })),
      Expression::ObjectLiteral(literal) => {
        let mut object = Object {
          items: HashMap::new(),
        };

        for (key, value) in literal.items {
          object.items.insert(key, self.evaluate(*value)?);
        }

        // Assign to memory
        let address = unsafe { crate::MEMORY.add_value(RuntimeValue::Object(object)) };

        // Finish
        Ok(RuntimeValue::ObjectContainer(ObjectContainer {
          location: address,
        }))
      }
      Expression::ArrayLiteral(literal) => {
        // Create the array
        let mut array = Array { items: vec![] };

        for i in literal.items {
          array.items.push(Box::new(self.evaluate(*i)?))
        }

        // Add to memory
        let address = unsafe { crate::MEMORY.add_value(RuntimeValue::Array(array)) };

        // Finish
        Ok(RuntimeValue::ArrayContainer(ArrayContainer {
          location: address,
        }))
      }
      Expression::FunctionLiteral(stmt) => {
        //let _name = stmt.identifier.symbol;
        let child_scope = self.scope.create_child();

        if stmt.is_pure {
          *child_scope.pure_functions_only.borrow_mut() = true;
        }

        let function = Function {
          scope: Rc::from(child_scope),
          body: Box::from(stmt.body),
          arguments: stmt.arguments,
          where_clause: stmt.where_clauses,
          name: match stmt.identifier {
            Some(ident) => Some(ident.symbol),
            None => None,
          },
          pure: stmt.is_pure,
        };

        Ok(RuntimeValue::Function(function))
      }

      //////////////////////////////
      // ----- Expressions ----- //
      ////////////////////////////
      Expression::TernaryExpression(expr) => {
        // Check if the test is truthy
        let truthy = (self.evaluate(*expr.test)?).is_truthy();
        if truthy {
          self.evaluate(*expr.success)
        } else {
          self.evaluate(*expr.alternate)
        }
      }
      Expression::CallExpression(expr) => {
        let expr2 = expr.clone();
        let callee = self.evaluate(*expr.left)?;

        match callee {
          RuntimeValue::NativeFunction(func) => {
            let caller_args = expr2.arguments.clone();
            let args = caller_args
              .iter()
              .map(|e| self.evaluate(*e.clone()))
              .collect::<Result<Vec<_>, _>>()?;

            (func.func)(&args)
          }
          RuntimeValue::Function(func) => {
            if *self.scope.pure_functions_only.borrow() {
              *self.scope.pure_functions_only.borrow_mut() = false;
              // Check if it is pure
              if !func.pure {
                return Err(ZephyrError::parser(
                  "Expected pure function here".to_string(),
                  expr2.clone().left.get_location(),
                ));
              }
            }
            // Get the scope
            let scope = if func.pure {
              self.global_scope.create_child()
            } else {
              func.scope.create_child()
            };
            let caller_args = expr2.arguments.clone();

            if func.pure {
              *scope.pure_functions_only.borrow_mut() = true;
            }

            // Declare the args
            for i in 0..func.arguments.len() {
              let assigned = if expr.arguments.len() >= func.arguments.len() {
                let arg = caller_args.get(i).unwrap().clone();
                self.evaluate(*arg)?
              } else {
                RuntimeValue::Null(Null {})
              };

              scope.declare_variable(&func.arguments.get(i).unwrap().clone().symbol, assigned)?;
            }

            // Check where clauses
            let prev = std::mem::replace(&mut self.scope, scope.clone());
            {
              *self.scope.pure_functions_only.borrow_mut() = true;
            }
            for clause in func.where_clause.tests {
              let res = self.evaluate((**&clause).clone())?;

              // Check if it succeeded
              if !res.is_truthy() {
                return Err(ZephyrError::runtime(
                  format!("Call failed to pass where clauses, received: {}", res),
                  clause.clone().get_location().clone(),
                ));
              }
            }
            {
              *self.scope.pure_functions_only.borrow_mut() = func.pure;
            }
            let _ = std::mem::replace(&mut self.scope, prev);

            let return_value = self.evaluate_block(*func.body, scope)?;

            // Check if it is predicate
            if let Some(f) = func.name {
              // Check endswith ?
              if f.ends_with("?") {
                // Check for boolean
                if !matches!(return_value, RuntimeValue::Boolean(_)) {
                  return Err(ZephyrError::runtime(
                    "Predicate function can only return booleans".to_string(),
                    Location::no_location(),
                  ));
                }
              }
            }

            Ok(return_value)
          }
          _ => Err(runtime_error!("Expected a function to call".to_string())),
        }
      }
      Expression::AssignmentExpression(expr) => {
        let right = self.evaluate(*expr.right)?;

        match *expr.left {
          Expression::MemberExpression(mem) => {
            self.evaluate_member_expression(mem, &Some(right.clone()))
          }
          Expression::Identifier(ident) => self.scope.modify_variable(&ident.symbol, right),
          _ => unimplemented!(),
        }
      }
      Expression::MemberExpression(expr) => self.evaluate_member_expression(expr, &None),
      Expression::ArithmeticOperator(expr) => {
        // Collect values
        let left = self.evaluate(*expr.left)?;
        let right = self.evaluate(*expr.right)?;

        // Check if both are numbers
        if util::varient_eq(&left, &right) && matches!(left, RuntimeValue::Number(_)) {
          // Convert to numbers
          let left_number = match left {
            RuntimeValue::Number(val) => val,
            _ => panic!(""),
          };
          let right_number = match right {
            RuntimeValue::Number(val) => val,
            _ => panic!(""),
          };
          let value: f64 = match expr.operator {
            TokenType::AdditiveOperator(AdditiveTokenType::Plus) => {
              left_number.value + right_number.value
            }
            TokenType::AdditiveOperator(AdditiveTokenType::Minus) => {
              left_number.value - right_number.value
            }
            TokenType::MultiplicativeOperator(MultiplicativeTokenType::Multiply) => {
              left_number.value * right_number.value
            }
            TokenType::MultiplicativeOperator(MultiplicativeTokenType::Divide) => {
              left_number.value / right_number.value
            }
            TokenType::MultiplicativeOperator(MultiplicativeTokenType::IntegerDivide) => {
              (left_number.value as i64 / right_number.value as i64) as f64
            }
            TokenType::MultiplicativeOperator(MultiplicativeTokenType::Modulo) => {
              left_number.value % right_number.value
            }
            _ => unreachable!(),
          };

          return Ok(RuntimeValue::Number(Number { value }));
        }

        // Try others
        let result: Option<RuntimeValue> = match expr.operator {
          TokenType::AdditiveOperator(AdditiveTokenType::Plus) => match left {
            RuntimeValue::StringValue(ref left_value) => {
              let right_value: Option<String> = match right {
                RuntimeValue::StringValue(ref string_value) => {
                  Some(String::from(&*string_value.value))
                }
                RuntimeValue::Number(ref number_value) => Some(number_value.value.to_string()),
                RuntimeValue::Boolean(ref bool_value) => Some(bool_value.value.to_string()),
                RuntimeValue::Null(_) => Some("null".to_string()),
                RuntimeValue::Reference(ref refer) => Some(refer.value.to_string()),
                _ => {
                  return Err(ZephyrError::runtime(
                    format!("Cannot coerce a {} to a string", right.type_name()),
                    Location::no_location(),
                  ))
                }
              };

              match right_value {
                Some(val) => Some(RuntimeValue::StringValue(StringValue {
                  value: String::from(&*left_value.value) + &*val,
                })),
                None => None,
              }
            }
            RuntimeValue::ArrayContainer(ref container) => {
              let mut array = match unsafe { crate::MEMORY.get_value(container.location)? } {
                RuntimeValue::Array(arr) => arr,
                _ => unreachable!(),
              };

              array.items.push(Box::from(right.clone()));

              // Modify the value
              unsafe { crate::MEMORY.set_value(container.location, RuntimeValue::Array(array))? };
              Some(RuntimeValue::ArrayContainer(container.clone()))
            }
            _ => None,
          },
          _ => None,
        };

        match result {
          Some(val) => Ok(val),
          None => Err(errors::ZephyrError::runtime(
            format!(
              "Cannot handle {} {} {}",
              left.type_name(),
              expr.operator,
              right.clone().type_name()
            ),
            Location::no_location(),
          )),
        }
      }
      Expression::ComparisonOperator(expr) => {
        // Collect values
        let left = self.evaluate(*expr.left)?;
        let right = self.evaluate(*expr.right)?;

        // Check if they are the same type
        if !util::varient_eq(&left, &right) {
          return Err(ZephyrError::parser(
            format!(
              "Types are not of same type, got {} {} {}",
              left.type_name(),
              expr.operator,
              right.type_name(),
            ),
            expr.location,
          ));
        }

        let mut result = false;

        // Numbers
        if matches!(left, RuntimeValue::Number(_)) {
          let left_number = match left {
            RuntimeValue::Number(num) => num.value,
            _ => unreachable!(),
          };
          let right_number = match right {
            RuntimeValue::Number(num) => num.value,
            _ => unreachable!(),
          };

          result = match expr.operator {
            ComparisonTokenType::Equals => left_number == right_number,
            ComparisonTokenType::NotEquals => left_number == right_number,
            ComparisonTokenType::GreaterThan => left_number > right_number,
            ComparisonTokenType::GreaterThanOrEquals => left_number >= right_number,
            ComparisonTokenType::LessThan => left_number < right_number,
            ComparisonTokenType::LessThanOrEquals => left_number <= right_number,
          };
        } else if matches!(expr.operator, ComparisonTokenType::Equals)
          || matches!(expr.operator, ComparisonTokenType::NotEquals)
        {
          // Booleans
          if matches!(left, RuntimeValue::Boolean(_)) {
            let left_bool = match left {
              RuntimeValue::Boolean(bool) => bool,
              _ => unreachable!(),
            };
            let right_bool = match right {
              RuntimeValue::Boolean(bool) => bool,
              _ => unreachable!(),
            };

            if left_bool.value == right_bool.value {
              result = true;
            }
          }
          // Strings
          else if matches!(left, RuntimeValue::StringValue(_)) {
            let left_bool = match left {
              RuntimeValue::StringValue(string) => string,
              _ => unreachable!(),
            };
            let right_bool = match right {
              RuntimeValue::StringValue(string) => string,
              _ => unreachable!(),
            };

            if left_bool.value == right_bool.value {
              result = true;
            }
          }
          // Null
          else if matches!(left, RuntimeValue::Null(_)) {
            // This will always be true
            result = true;
          } else {
            return Err(ZephyrError::parser(
              format!("Cannot handle {} {} {}", left, expr.operator, right),
              expr.location,
            ));
          }
        } else {
          return Err(ZephyrError::parser(
            format!("Cannot handle {} {} {}", left, expr.operator, right),
            expr.location,
          ));
        }

        // Check if it is negate
        if matches!(expr.operator, ComparisonTokenType::NotEquals) {
          result = !result;
        }

        return Ok(RuntimeValue::Boolean(Boolean { value: result }));
      }

      Expression::LogicalExpression(expr) => {
        // Collect values
        let left = self.evaluate(*expr.left)?;
        let right = self.evaluate(*expr.right)?;

        Ok(RuntimeValue::Boolean(Boolean {
          value: match expr.operator {
            LogicalTokenType::And => left.is_truthy() && right.is_truthy(),
            LogicalTokenType::Or => left.is_truthy() || right.is_truthy(),
          },
        }))
      }
      Expression::TypeofExpression(expr) => Ok(RuntimeValue::StringValue(StringValue {
        value: match self.evaluate(*expr.value) {
          Ok(val) => val.type_name().to_string(),
          Err(err) => return Err(err),
        },
      })),
      Expression::UnaryExpression(expr) => {
        let expr_value = *expr.value;
        let value = self.evaluate(expr_value.clone())?;
        let operator = expr.operator;

        match operator {
          TokenType::UnaryOperator(UnaryOperator::Not) => Ok(RuntimeValue::Boolean(Boolean {
            value: !value.is_truthy(),
          })),
          TokenType::UnaryOperator(UnaryOperator::Dereference) => match value {
            RuntimeValue::Reference(refer) => unsafe { crate::MEMORY.get_value(refer.value) },
            RuntimeValue::Number(num) => unsafe {
              crate::MEMORY.get_value(num.value as MemoryAddress)
            },
            RuntimeValue::ArrayContainer(arr) => unsafe { crate::MEMORY.get_value(arr.location) },
            _ => Err(ZephyrError::runtime(
              format!("Cannot derference a {:?}", value.type_name()),
              Location::no_location(),
            )),
          },
          TokenType::UnaryOperator(UnaryOperator::Reference) => {
            Ok(RuntimeValue::Reference(Reference {
              value: match expr_value.clone() {
                Expression::Identifier(ident) => {
                  match self.scope.get_variable_address(&ident.symbol) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                  }
                }
                Expression::NumericLiteral(ident) => ident.value as MemoryAddress,
                _ => {
                  return Err(ZephyrError::runtime(
                    format!("Cannot reference this"),
                    Location::no_location(),
                  ))
                }
              },
            }))
          }
          _ => unimplemented!(),
        }
      }

      // ----- Statement like expressions -----
      Expression::IfExpression(expr) => {
        let test = self.evaluate(*expr.test)?;

        if test.is_truthy() {
          // Run the success block
          self.evaluate_block(*expr.success, self.scope.create_child())
        } else if let Some(alt) = expr.alternate {
          self.evaluate(*alt)
        } else {
          Ok(RuntimeValue::Null(Null {}))
        }
      }
      Expression::WhileExpression(expr) => {
        while self.evaluate(*expr.test.clone())?.is_truthy() {
          self.evaluate_block(*expr.body.clone(), self.scope.create_child())?;
        }

        Ok(RuntimeValue::Null(Null {}))
      }
      Expression::ForLoop(expr) => {
        let value = self.evaluate(*expr.value_to_iter)?.iterate()?;
        let mut values: Vec<Box<RuntimeValue>> = vec![];

        for i in value {
          let scope = self.scope.create_child();
          scope.declare_variable(&expr.identifier.symbol, *i.clone())?;
          values.push(Box::from(self.evaluate_block(expr.body.clone(), scope)?));
        }

        return Ok(to_array(values));
      }
      _ => Err(errors::ZephyrError::runtime(
        String::from("Cannot handle this AST node"),
        Location::no_location(),
      )),
    };

    // Check if no location
    match result {
      Ok(ok) => Ok(ok),
      Err(mut err) => {
        err.location = if err.location.char_start == 0 {
          node.clone().get_location()
        } else {
          err.location
        };

        Err(err)
      }
    }
  }
}
