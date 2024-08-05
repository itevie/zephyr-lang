use std::collections::HashMap;

use crate::{
  errors::{ErrorType, ZephyrError},
  lexer::{
    lexer,
    location::Location,
    token::{AdditiveTokenType, LogicalTokenType, MultiplicativeTokenType, TokenType},
  },
  parser::{
    nodes::{self, Expression},
    parser::Parser,
  },
};

use super::{
  native_functions::{self, CallOptions},
  scope::ScopeContainer,
  values::{self, RuntimeValue},
};

#[derive(Clone, Copy)]
pub struct Interpreter {
  pub scope: ScopeContainer,
  pub global_scope: ScopeContainer,
}

unsafe impl Send for Interpreter {}
unsafe impl Sync for Interpreter {}

type R = Result<RuntimeValue, ZephyrError>;

/// The first argument is the name of the function within the language,
/// the secont argument is what the rust function is called
macro_rules! add_native {
  ($name:expr, $nv_name:ident) => {
    (
      $name.to_string(),
      values::NativeFunction::make(&native_functions::$nv_name),
    )
  };
}

macro_rules! include_lib {
  ($what:expr) => {
    (include_str!($what), $what)
  };
}

impl Interpreter {
  pub fn new(directory: String) -> Self {
    // Create the scopes
    let global_scope = ScopeContainer::new(directory.clone());
    let scope = global_scope.create_child().unwrap();

    let libary_files: Vec<(&str, &str)> = vec![
      include_lib!("../lib/any.zr"),
      include_lib!("../lib/predicates.zr"),
      include_lib!("../lib/array.zr"),
      include_lib!("../lib/string.zr"),
      include_lib!("../lib/math.zr"),
      include_lib!("../lib/console.zr"),
      include_lib!("../lib/basics.zr"),
      include_lib!("../lib/time.zr"),
      include_lib!("../lib/object.zr"),
      include_lib!("../lib/network.zr"),
      include_lib!("../lib/fs.zr"),
      include_lib!("../lib/number.zr"),
      include_lib!("../lib/error.zr"),
      include_lib!("../lib/process.zr"),
      include_lib!("../lib/random.zr"),
      include_lib!("../lib/function.zr"),
      include_lib!("../lib/buffer.zr"),
      include_lib!("../lib/http.zr"),
      include_lib!("../lib/json.zr"),
      include_lib!("../lib/tests.zr"),
    ];

    let native_object = values::Object::make(HashMap::from([
      add_native!("print", print),
      add_native!("write", write),
      add_native!("read_line", read_line),
      add_native!("iter", iter),
      add_native!("reverse", reverse),
      add_native!("rg_is_match", rg_is_match),
      add_native!("rg_replace", rg_replace),
      add_native!("buff_to_utf8", buff_to_utf8),
      add_native!("utf8_to_buff", utf8_to_buff),
      add_native!("str_to_number", str_to_number),
      add_native!("error", error),
      add_native!("spawn_thread", spawn_thread),
      add_native!("clear_console", clear_console),
      add_native!("rlt", rust_lambda_test),
      add_native!("arr_ref_set", arr_ref_set),
      add_native!("unescape", unescape),
      add_native!("zephyr_to_json_n", zephyr_to_json_n),
      add_native!("get_time_nanos", get_time_nanos),
      add_native!("floor", floor),
      add_native!("slice", slice),
      add_native!("ceil", ceil),
      add_native!("parse_json", json_parse),
      add_native!("read_file", read_file),
      add_native!("push_arr", push_arr),
      add_native!("get_args", get_args),
      add_native!("random", random),
      add_native!("random_item", random_item),
      add_native!("random_range", random_range),
      add_native!("call_zephyr_function", call_zephyr_function),
    ]))
    .create_container();

    // Declare the native functions
    global_scope
      .declare_variable("__zephyr_native", native_object)
      .unwrap();

    // Load the libraries
    for lib in libary_files {
      let lib_scope = scope.create_child().unwrap();
      lib_scope.set_can_export(true).unwrap();

      // Tokenize it
      let lexed = match lexer::lex(lib.0.to_string(), format!("<lib {}>", lib.1)) {
        Ok(ok) => ok,
        Err(err) => {
          println!(
            "Error while lexing lib {}\n\n{}",
            lib.1,
            err.visualise(false)
          );
          panic!()
        }
      };

      // Parse it
      let parsed = match Parser::new(lexed)
        .produce_ast(Some(format!("<lib {}>", lib.1.replace("../lib/", ""))))
      {
        Ok(ok) => ok,
        Err(err) => {
          println!(
            "Error while parsing lib {}\n\n{}",
            lib.1,
            err.visualise(false)
          );
          panic!();
        }
      };

      let mut lib_interpreter = Interpreter {
        scope: lib_scope,
        global_scope,
      };

      // Run it
      match lib_interpreter.evaluate(Expression::Program(parsed)) {
        Ok(_) => (),
        Err(err) => {
          println!(
            "Error while executing lib {}\n\n{}",
            lib.1,
            err.visualise(false)
          );
          panic!()
        }
      };

      // Extract the exports
      for (key, value) in lib_scope.get_exports().unwrap().iter() {
        global_scope
          .declare_variable_with_address(key, *value)
          .unwrap();
      }

      lib_scope.set_can_export(false).unwrap();
    }

    Interpreter {
      scope,
      global_scope,
    }
  }

  pub fn replace_scope_with(&mut self, scope: ScopeContainer) -> ScopeContainer {
    
    std::mem::replace(&mut self.scope, scope)
  }

  pub fn evaluate(&mut self, node: Expression) -> R {
    let result = match node.clone() {
      // ----- Special Things -----
      Expression::Program(program) => {
        self.scope.set_file(program.file)?;
        self.evaluate_expr_vec(program.nodes, self.scope)
      }

      // ----- Values -----
      Expression::NumericLiteral(number) => Ok(values::Number::make(number.value)),
      Expression::StringLiteral(string) => Ok(values::StringValue::make(string.value)),
      Expression::Identifier(identifier) => self.scope.get_variable(&identifier.symbol),
      Expression::ObjectLiteral(object) => {
        let mut items: HashMap<String, RuntimeValue> = HashMap::new();

        for (key, value) in object.items {
          items.insert(key, self.evaluate(*value)?);
        }

        Ok(values::Object::make(items).create_container())
      }
      Expression::ArrayLiteral(array) => {
        let mut items: Vec<Box<RuntimeValue>> = vec![];

        for item in array.items {
          items.push(Box::from(self.evaluate(*item)?));
        }

        Ok(values::Array::make(items).create_container())
      }
      Expression::FunctionLiteral(function) => {
        let function = values::Function {
          scope: self.scope.create_child()?,
          body: function.body,
          arguments: function.arguments,
          where_clause: function.where_clauses,
          name: match function.identifier {
            Some(identifier) => Some(identifier.symbol),
            None => None,
          },
          type_call: None,
        };

        Ok(RuntimeValue::Function(function))
      }

      // ----- Expressions -----
      Expression::MemberExpression(member_expr) => self.evaluate_member_expression(member_expr),
      Expression::CallExpression(call) => {
        let to_call = self.evaluate(*call.clone().left)?;

        // Collect the arguments
        let arguments = call
          .arguments
          .iter()
          .map(|e| self.evaluate(*e.clone()))
          .collect::<Result<Vec<_>, _>>()?;

        // Get the result of the call
        let result = match to_call {
          RuntimeValue::Function(function) => {
            let mut new_arguments: Vec<Box<RuntimeValue>> =
              arguments.iter().map(|e| Box::from(e.clone())).collect();

            // Check if it is a type call
            if let Some(t) = &function.type_call {
              new_arguments.insert(0, t.clone());
            }

            // Execute it
            self.evaluate_lang_function(function, new_arguments, call.clone().location)
          }

          RuntimeValue::NativeFunction(function) => (function.func)(CallOptions {
            args: arguments,
            location: call.clone().location,
            interpreter: *self,
          }),

          RuntimeValue::NativeFunction2(function) => (function.func)(CallOptions {
            args: arguments,
            location: call.clone().location,
            interpreter: *self,
          }),

          _ => {
            return Err(ZephyrError::runtime(
              format!("Cannot call a {}", to_call.type_name()),
              call.clone().location,
            ))
          }
        };

        // Check if the err should be modified
        match result {
          Ok(ok) => Ok(ok),
          Err(mut err) => {
            err.backtrace.push(call.clone().location);
            Err(err)
          }
        }
      }
      Expression::ArithmeticOperator(expr) => {
        // Get values
        let left = self.evaluate(*expr.left.clone())?;
        let right = self.evaluate(*expr.right.clone())?;

        // Check special operators
        match expr.operator {
          TokenType::MultiplicativeOperator(MultiplicativeTokenType::Coalesce) => {
            return Ok({
              if let RuntimeValue::Null(_) = left {
                right
              } else {
                left
              }
            })
          }
          _ => (),
        };

        // Number-only operations
        if let (RuntimeValue::Number(left_number), RuntimeValue::Number(right_number)) =
          (left.clone(), right.clone())
        {
          return Ok(values::Number::make(match expr.operator {
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
          }));
        }

        // Try other operations
        let result: Option<RuntimeValue> = match expr.operator {
          TokenType::AdditiveOperator(AdditiveTokenType::Plus) => match left {
            RuntimeValue::StringValue(ref left_value) => match right {
              RuntimeValue::StringValue(ref right_value) => Some(values::StringValue::make(
                left_value.value.clone() + &right_value.value,
              )),
              RuntimeValue::Number(ref right_value) => Some(values::StringValue::make(
                left_value.value.clone() + &right_value.value.to_string(),
              )),
              RuntimeValue::Boolean(ref right_value) => Some(values::StringValue::make(
                left_value.value.clone() + &right_value.value.to_string(),
              )),
              RuntimeValue::Null(_) => {
                Some(values::StringValue::make(left_value.value.clone() + "null"))
              }
              _ => None,
            },
            _ => None,
          },
          _ => None,
        };

        match result {
          Some(val) => Ok(val),
          None => Err(ZephyrError::runtime(
            format!(
              "Cannot handle {} {} {}",
              left.type_name(),
              expr.operator,
              right.type_name()
            ),
            expr.location,
          )),
        }
      }
      Expression::ComparisonOperator(expr) => {
        let left = self.evaluate(*expr.left.clone())?;
        let right = self.evaluate(*expr.right.clone())?;

        Ok(values::Boolean::make(left.is_equal_to(right, Some(expr))?))
      }
      Expression::LogicalExpression(expr) => {
        let left = self.evaluate(*expr.left)?;
        let right = self.evaluate(*expr.right)?;

        Ok(match expr.operator {
          LogicalTokenType::And => values::Boolean::make(left.is_truthy() && right.is_truthy()),
          LogicalTokenType::Or => {
            if left.is_truthy() {
              left
            } else {
              right
            }
          }
        })
      }
      Expression::ForLoop(expr) => {
        let value = self.evaluate(*expr.value_to_iter)?.iterate()?;
        let mut values: Vec<Box<RuntimeValue>> = vec![];

        // Loop through the provided values
        for (i, v) in value.iter().enumerate() {
          // Create the loop's scope and assign the variables
          let scope = self.scope.create_child()?;
          scope.declare_variable(
            &expr.index_identifier.symbol,
            values::Number::make(i as f64),
          )?;
          if let Some(ref value_ident) = expr.value_identifier {
            scope.declare_variable(&value_ident.symbol, *v.clone())?;
          }

          // Evaluate the block
          let result = match self.evaluate_expr_vec(expr.body.nodes.clone(), scope) {
            Ok(ok) => ok,
            Err(err) => match err.error_type {
              // ----- Breaks -----
              ErrorType::Break(ref break_name) => {
                // Check if it is breaking a certain name
                if let Some(name) = break_name {
                  // Check if the current loop has a name
                  if let Some(ref current_name) = &expr.name {
                    // Check if it is the same
                    if *name == current_name.symbol {
                      break;
                    } else {
                      // Propgate
                      return Err(err);
                    }
                  } else {
                    // Propogate
                    return Err(err);
                  }
                } else {
                  break;
                }
              }
              ErrorType::Continue(ref continue_name) => {
                // Check if it is continueing a certain name
                if let Some(name) = continue_name {
                  // Check if the current loop has a name
                  if let Some(ref current_name) = &expr.name {
                    // Check if it is the same
                    if *name == current_name.symbol {
                      continue;
                    } else {
                      // Propgate
                      return Err(err);
                    }
                  } else {
                    // Propogate
                    return Err(err);
                  }
                } else {
                  continue;
                }
              }
              _ => return Err(err),
            },
          };

          // Finish up
          values.push(Box::from(result));
          scope.delete()?;
        }

        Ok(values::Array::make(values).create_container())
      }
      Expression::IfExpression(expr) => {
        let test_result = self.evaluate(*expr.test)?;

        if test_result.is_truthy() {
          self.evaluate_expr_vec(expr.success.nodes, self.scope.create_child()?)
        } else if let Some(alternate) = expr.alternate {
          self.evaluate(*alternate)
        } else {
          Ok(values::Null::make())
        }
      }
      Expression::AssignmentExpression(expr) => {
        let to_assign = self.evaluate(*expr.right)?;

        match *expr.left {
          // Assign directly to variable
          Expression::Identifier(ident) => {
            self.scope.modify_variable(&ident.symbol, to_assign)?;
          }

          // Assign via member expression
          Expression::MemberExpression(member) => {
            // Rules
            // Given array: can only assign via number & computed
            // Given object: can only assign via identifier or string & computed
            let value = self.evaluate(*member.left)?;

            match value {
              RuntimeValue::ObjectContainer(container) => {
                let key = if member.is_computed {
                  match self.evaluate(*member.key.clone())? {
                    RuntimeValue::StringValue(string) => string.value,
                    v => {
                      return Err(ZephyrError::runtime(
                        format!(
                          "Expected a string key for assigning to object, but got: {}",
                          v.type_name()
                        ),
                        member.key.get_location(),
                      ))
                    }
                  }
                } else {
                  match *member.key {
                    Expression::Identifier(ident) => ident.symbol,
                    _ => {
                      return Err(ZephyrError::runtime(
                        "Expected identifier as key to assign to object".to_string(),
                        member.key.get_location(),
                      ))
                    }
                  }
                };

                // Modify
                let mut object = container.deref();
                object.items.insert(key, to_assign);

                // Reset
                crate::MEMORY
                  .lock()
                  .unwrap()
                  .set_value(container.location, RuntimeValue::Object(object))?;
              }
              RuntimeValue::ArrayContainer(container) => {
                if !member.is_computed {
                  return Err(ZephyrError::runtime(
                    "Index should be computed".to_string(),
                    member.key.get_location(),
                  ));
                }

                let key = match self.evaluate(*member.key.clone())? {
                  RuntimeValue::Number(number) => number.value,
                  v => {
                    return Err(ZephyrError::runtime(
                      format!(
                        "Can only index arrays with numbers, but got a {}",
                        v.type_name()
                      ),
                      member.key.get_location(),
                    ))
                  }
                };

                // Expect whole number
                if key.fract() != 0.0 {
                  return Err(ZephyrError::runtime(
                    format!("Expected a whole number, but got {}", key),
                    member.key.get_location(),
                  ));
                }

                // Get array
                let mut array = container.deref();

                // Get proper index
                let index = if key < 0f64 {
                  array.items.len() - key as usize
                } else {
                  key as usize
                };

                // Check if out of bounds
                if index > array.items.len() {
                  return Err(ZephyrError::runtime(
                    format!(
                      "Index out of bounds, array's length is {} but got {}",
                      array.items.len(),
                      index
                    ),
                    member.key.get_location(),
                  ));
                }

                // Check 0
                if array.items.is_empty() {
                  array.items.insert(0, Box::new(values::Null::make()));
                }

                // Set it
                array.items[index] = Box::new(to_assign);

                // Reset
                crate::MEMORY
                  .lock()
                  .unwrap()
                  .set_value(container.location, RuntimeValue::Array(array))?;
              }
              v => {
                return Err(ZephyrError::runtime(
                  format!("Cannot assign to a {}", v.type_name()),
                  member.key.get_location(),
                ))
              }
            }
          }

          // Cannot assign
          v => {
            return Err(ZephyrError::runtime(
              format!("Cannot assign to a {:?}", v),
              v.get_location(),
            ))
          }
        }

        Ok(values::Null::make())
      }
      Expression::InExpression(expr) => {
        let test = self.evaluate(*expr.left.clone())?;
        let right = self.evaluate(*expr.right)?;

        match right {
          RuntimeValue::ArrayContainer(container) => {
            let items = container.deref().items;

            for i in items {
              if i.is_equal_to(test.clone(), None)? {
                return Ok(values::Boolean::make(true));
              }
            }
          }
          RuntimeValue::ObjectContainer(container) => {
            let string = match test {
              RuntimeValue::StringValue(string) => string.value,
              _ => {
                return Err(ZephyrError::runtime(
                  format!(
                    "Expected a string to check in object, but got {}",
                    test.type_name()
                  ),
                  expr.left.get_location(),
                ))
              }
            };

            return Ok(values::Boolean::make(
              container.deref().items.contains_key(&string),
            ));
          }
          RuntimeValue::StringValue(string) => {
            let test_string = match test {
              RuntimeValue::StringValue(string) => string.value,
              _ => {
                return Err(ZephyrError::runtime(
                  format!(
                    "Expected a string to check in string, but got {}",
                    test.type_name()
                  ),
                  expr.left.get_location(),
                ))
              }
            };

            return Ok(values::Boolean::make(string.value.contains(&test_string)));
          }
          _ => {
            return Err(ZephyrError::runtime(
              format!(
                "Cannot handle {} in {}",
                test.type_name(),
                right.type_name()
              ),
              expr.location,
            ))
          }
        };

        Ok(values::Boolean::make(false))
      }
      Expression::TypeofExpression(expr) => Ok(values::StringValue::make(
        self.evaluate(*expr.value)?.type_name().to_string(),
      )),

      // ----- Statements -----
      Expression::VariableDeclaration(declaration) => {
        let value = self.evaluate(*declaration.value)?;
        self
          .scope
          .declare_variable(&declaration.identifier.symbol, value)?;
        Ok(values::Null::make())
      }
      Expression::BreakStatement(break_stmt) => Err(ZephyrError {
        error_message: "Cannot break here".to_string(),
        error_type: ErrorType::Break(if let Some(ident) = break_stmt.name {
          Some(ident.symbol)
        } else {
          None
        }),
        backtrace: vec![],
        reference: None,
        location: break_stmt.location,
      }),
      Expression::ContinueStatement(continue_stmt) => Err(ZephyrError {
        error_message: "Cannot continue here".to_string(),
        error_type: ErrorType::Continue(if let Some(ident) = continue_stmt.name {
          Some(ident.symbol)
        } else {
          None
        }),
        backtrace: vec![],
        reference: None,
        location: continue_stmt.location,
      }),
      Expression::ReturnStatement(return_stmt) => Err(ZephyrError {
        error_message: "Cannot return here".to_string(),
        error_type: ErrorType::Return(if let Some(return_value) = return_stmt.value {
          Box::from(self.evaluate(*return_value)?)
        } else {
          Box::from(values::Null::make())
        }),
        backtrace: vec![],
        reference: None,
        location: return_stmt.location,
      }),
      Expression::ExportStatement(stmt) => {
        match *stmt.to_export {
          Expression::Identifier(ident) => {
            let value = self.scope.get_variable_address(&ident.symbol)?;
            self.scope.export(ident.symbol, value)?;
          }
          Expression::VariableDeclaration(declaration) => {
            let ident = declaration.identifier.symbol.clone();
            self.evaluate(Expression::VariableDeclaration(declaration))?;
            let address = self.scope.get_variable_address(&ident)?;
            self.scope.export(ident, address)?;
          }
          _ => {
            return Err(ZephyrError::runtime(
              format!("Cannot export a {:?}", *stmt.to_export),
              stmt.location,
            ))
          }
        };
        Ok(values::Null::make())
      } // ----- Unhandled -----
      _ => {
        return Err(ZephyrError::runtime(
          format!("The interpreter cannot handle a {:?}", node),
          node.get_location(),
        ))
      }
    };

    match result {
      Ok(ok) => Ok(ok),
      Err(mut err) => {
        // Check if it has a location
        if err.location.char_start == 0 {
          err.location = node.get_location()
        }

        Err(err)
      }
    }
  }

  pub fn evaluate_member_expression(&mut self, node: nodes::MemberExpression) -> R {
    let value = self.evaluate(*node.left.clone())?;

    // Rules:
    // object: identifier for properties
    // array: number for index
    // any: after the above, only identifier

    // Check for array
    if let RuntimeValue::ArrayContainer(ref array_container) = value {
      let key = self.evaluate(*node.key.clone())?;
      if let RuntimeValue::Number(number) = key {
        // Expect it to be a whole number
        if number.value.fract() != 0.0 {
          return Err(ZephyrError::runtime(
            format!(
              "Expected a whole number to index an array, but got {}",
              number.value
            ),
            node.key.get_location(),
          ));
        }

        // Get array and proper index (allowing negatives)
        let array = array_container.deref();
        let index = if number.value < 0f64 {
          array.items.len() - number.value as usize
        } else {
          number.value as usize
        };

        // Check out of bounds
        if index >= array.items.len() {
          return Err(ZephyrError::runtime(
            format!(
              "Index out of bounds, array's length is {} but got {}",
              array.items.len(),
              index
            ),
            node.key.get_location(),
          ));
        }

        // Return it
        return Ok(*array.items[index].clone());
      }
    }

    // Collect the key
    let collected_key = match *node.key {
      Expression::Identifier(ref identifier) => {
        if node.is_computed {
          self.evaluate(*node.key.clone())?
        } else {
          values::StringValue::make(identifier.symbol.clone())
        }
      }
      _ => self.evaluate(*node.key.clone())?,
    };

    // Check if the key was a string / identifier
    let key = if let RuntimeValue::StringValue(string) = collected_key {
      string.value
    } else {
      return Err(ZephyrError::runtime(
        format!(
          "Expected string or identifier, but got: {}",
          collected_key.type_name()
        ),
        node.key.get_location(),
      ));
    };

    // Check if it is an object
    if let RuntimeValue::ObjectContainer(ref object_container) = value.clone() {
      let object = object_container.deref();

      if object.items.contains_key(&key) {
        return Ok(object.items.get(&key).unwrap().clone());
      }
    }

    // Check type thing
    if let Some(type_value) = self.get_type_functions(value.clone(), &key)? {
      // Check if it was a function
      return Ok(match type_value {
        RuntimeValue::Function(mut function) => {
          function.type_call = Some(Box::from(value.clone()));
          RuntimeValue::Function(function)
        }
        v => v,
      });
    }

    // By now, all possibilites have failed
    Err(ZephyrError::runtime(
      format!("Value does not contain property {}", key),
      node.left.get_location(),
    ))
  }

  pub fn evaluate_lang_function(
    &mut self,
    function: values::Function,
    arguments: Vec<Box<RuntimeValue>>,
    location: Location,
  ) -> R {
    let scope = function.scope.create_child()?;

    for (index, value) in function.arguments.iter().enumerate() {
      // Check if it is the __args__
      if value.symbol == "__args__" {
        // Check if it is at the last position
        if index != function.arguments.len() - 1 {
          return Err(ZephyrError::lexer(
            "__args__ must be the last item in the parameters".to_string(),
            value.location,
          ));
        }

        scope.declare_variable(
          "__args__",
          values::Array::make(arguments.clone()).create_container(),
        )?;
      }

      // Get what it is to assign
      let to_assign = if index < arguments.len() {
        *arguments.get(index).unwrap().clone()
      } else {
        values::Null::make()
      };

      // Assign it in the scope
      scope.declare_variable(&value.symbol, to_assign)?;
    }

    // Swap scopes
    let prev_scope = self.replace_scope_with(scope);

    // Check where clauses
    for clause in function.where_clause.tests {
      let result = match self.evaluate(*clause.clone()) {
        Ok(ok) => ok,
        Err(err) => {
          self.replace_scope_with(prev_scope);
          return Err(err);
        }
      };

      // Check if it succeeded
      if !result.is_truthy() {
        self.replace_scope_with(prev_scope);
        return Err(ZephyrError::runtime_with_ref(
          format!("Call failed to pass where clauses, recieved {}", result),
          location,
          clause.get_location(),
        ));
      }
    }

    // Evaluate the body
    let return_value = match self.evaluate_expr_vec(function.body.nodes, self.scope) {
      Ok(ok) => ok,
      Err(err) => match err.error_type {
        ErrorType::Return(val) => *val,
        _ => return Err(err),
      },
    };

    let _ = self.replace_scope_with(prev_scope);

    // Check if the function was a predicate and didn't return boolean
    if let Some(ref function_name) = function.name {
      if function_name.ends_with('?') && !matches!(return_value, RuntimeValue::Boolean(_)) {
        return Err(ZephyrError::runtime(
          format!(
            "Predicate functions can only return booleans, but it returned: {}",
            return_value
          ),
          location,
        ));
      }
    }

    Ok(return_value)
  }

  pub fn evaluate_expr_vec(&mut self, nodes: Vec<Box<Expression>>, scope: ScopeContainer) -> R {
    // Swap the scope with the wanted one
    let prev_scope = self.replace_scope_with(scope);
    let mut last_evaluated: Option<RuntimeValue> = None;

    // Evaluate each element
    for expr in nodes {
      last_evaluated = match self.evaluate(*expr) {
        Ok(ok) => Some(ok),
        Err(err) => {
          // Swap the scope back
          let _ = self.replace_scope_with(prev_scope);
          return Err(err);
        }
      }
    }

    // Swap the scope back
    let _ = self.replace_scope_with(prev_scope);

    match last_evaluated {
      None => Ok(values::Null::make()),
      Some(val) => Ok(val),
    }
  }

  pub fn get_type_functions(
    &mut self,
    value: RuntimeValue,
    key: &str,
  ) -> Result<Option<RuntimeValue>, ZephyrError> {
    let variable_to_get = match value {
      RuntimeValue::StringValue(_) => "String",
      RuntimeValue::Number(_) => "Number",
      RuntimeValue::ArrayContainer(_) => "Array",
      RuntimeValue::ObjectContainer(_) => "Object",
      RuntimeValue::Function(_) => "Function",
      _ => "Any",
    };

    // Collect object
    let variable = self.global_scope.get_variable(variable_to_get)?;
    let object = match variable {
      RuntimeValue::ObjectContainer(container) => container.deref(),
      _ => {
        return Err(ZephyrError::runtime(
          format!(
            "Global type function should be of type object, but got: {}",
            variable.type_name()
          ),
          Location::no_location(),
        ))
      }
    };

    // Check if it has the wanted function
    if object.items.contains_key(key) {
      return Ok(Some(object.items.get(key).unwrap().clone()));
    }

    // Check if Any has it
    if variable_to_get != "Any" {
      let any_variable = self.global_scope.get_variable("Any")?;
      let any_object = match any_variable {
        RuntimeValue::ObjectContainer(container) => container.deref(),
        _ => {
          return Err(ZephyrError::runtime(
            format!(
              "Global type function should be of type object, but got: {}",
              any_variable.type_name()
            ),
            Location::no_location(),
          ))
        }
      };

      // Check if it has the wanted function
      if any_object.items.contains_key(key) {
        return Ok(Some(any_object.items.get(key).unwrap().clone()));
      }
    }

    Ok(None)
  }
}

/*
⠀⢸⠂⠀⠀⠀⠘⣧⠀⠀⣟⠛⠲⢤⡀⠀⠀⣰⠏⠀⠀⠀⠀⠀⢹⡀
⠀⡿⠀⠀⠀⠀⠀⠈⢷⡀⢻⡀⠀⠀⠙⢦⣰⠏⠀⠀⠀⠀⠀⠀⢸⠀
⠀⡇⠀⠀⠀⠀⠀⠀⢀⣻⠞⠛⠀⠀⠀⠀⠻⠀⠀⠀⠀⠀⠀⠀⢸⠀
⠀⡇⠀⠀⠀⠀⠀⠀⠛⠓⠒⠓⠓⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⠀
⠀⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣸⠀
⠀⢿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⣀⣀⣀⠀⠀⢀⡟⠀
⠀⠘⣇⠀⠘⣿⠋⢹⠛⣿⡇⠀⠀⠀⠀⣿⣿⡇⠀⢳⠉⠀⣠⡾⠁⠀
⣦⣤⣽⣆⢀⡇⠀⢸⡇⣾⡇⠀⠀⠀⠀⣿⣿⡷⠀⢸⡇⠐⠛⠛⣿⠀
⠹⣦⠀⠀⠸⡇⠀⠸⣿⡿⠁⢀⡀⠀⠀⠿⠿⠃⠀⢸⠇⠀⢀⡾⠁⠀
⠀⠈⡿⢠⢶⣡⡄⠀⠀⠀⠀⠉⠁⠀⠀⠀⠀⠀⣴⣧⠆⠀⢻⡄⠀⠀
⠀⢸⠃⠀⠘⠉⠀⠀⠀⠠⣄⡴⠲⠶⠴⠃⠀⠀⠀⠉⡀⠀⠀⢻⡄⠀
⠀⠘⠒⠒⠻⢦⣄⡀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⣤⠞⠛⠒⠛⠋⠁⠀
⠀⠀⠀⠀⠀⠀⠸⣟⠓⠒⠂⠀⠀⠀⠀⠀⠈⢷⡀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠙⣦⠀⠀⠀⠀⠀⠀⠀⠀⠈⢷⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⣼⣃⡀⠀⠀⠀⠀⠀⠀⠀⠀⠘⣆⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠉⣹⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⢻⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⡿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⡆⠀⠀⠀⠀⠀
*/
