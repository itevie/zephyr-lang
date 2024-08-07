use std::{
  collections::HashMap,
  path::PathBuf,
  sync::{Arc, Mutex},
  time::Instant,
};

use lazy_static::lazy_static;
use once_cell::sync::Lazy;
use uuid::Uuid;

use crate::{
  errors::{self, runtime_error, ErrorType, ZephyrError},
  lexer::{
    self,
    lexer::lex,
    location::Location,
    token::{
      AdditiveTokenType, ComparisonTokenType, LogicalTokenType, MultiplicativeTokenType, TokenType,
      UnaryOperator,
    },
  },
  parser::{
    self,
    nodes::{Block, ComparisonExpression, Expression, Identifier, MemberExpression},
    parser::Parser,
  },
  util,
};

use super::{
  memory::MemoryAddress,
  native_functions::{self, CallOptions},
  scope::ScopeContainer,
  values::{
    Array, ArrayContainer, Boolean, Function, NativeFunction, Null, Number, Object,
    ObjectContainer, RuntimeValue, StringValue,
  },
};

static IMPORT_CACHE: Lazy<Arc<Mutex<HashMap<String, ScopeContainer>>>> =
  Lazy::new(|| Arc::from(Mutex::from(HashMap::new())));
pub static NODE_EVALUATION_TIMES: Lazy<Arc<Mutex<HashMap<String, Vec<u128>>>>> =
  Lazy::new(|| Arc::from(Mutex::from(HashMap::new())));

#[derive(Clone, Copy)]
pub struct InterpreterOld {
  pub scope: ScopeContainer,
  pub global_scope: ScopeContainer,
}

unsafe impl Send for InterpreterOld {}
unsafe impl Sync for InterpreterOld {}

lazy_static! {
  pub static ref SYMBOLS: HashMap<&'static str, Uuid> = {
    let mut h = HashMap::new();
    h.insert("PrettyPrint", Uuid::new_v4());
    h
  };
}

pub fn get_symbol(name: &str) -> String {
  SYMBOLS.get(name).unwrap().to_string()
}

macro_rules! include_lib {
  ($what:expr) => {
    (include_str!($what), $what)
  };
}

impl InterpreterOld {
  pub fn new(directory: String) -> Self {
    let start = Instant::now();
    let libs: Vec<(&str, &str)> = vec![
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
    let scope = ScopeContainer::new(directory);

    let native_address = crate::MEMORY
      .lock()
      .unwrap()
      .add_value(RuntimeValue::Object(Object {
        items: HashMap::from([
          (
            "print".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::print,
            }),
          ),
          (
            "write".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::write,
            }),
          ),
          (
            "read_line".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::read_line,
            }),
          ),
          (
            "iter".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::iter,
            }),
          ),
          (
            "reverse".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::reverse,
            }),
          ),
          (
            "rg_is_match".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::rg_is_match,
            }),
          ),
          (
            "rg_replace".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::rg_replace,
            }),
          ),
          (
            "buff_to_utf8".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::buff_to_utf8,
            }),
          ),
          (
            "utf8_to_buff".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::utf8_to_buff,
            }),
          ),
          (
            "str_to_number".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::str_to_number,
            }),
          ),
          (
            "error".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::error,
            }),
          ),
          (
            "spawn_thread".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::spawn_thread,
            }),
          ),
          (
            "clear_console".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::clear_console,
            }),
          ),
          (
            "rlt".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::rust_lambda_test,
            }),
          ),
          (
            "arr_ref_set".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::arr_ref_set,
            }),
          ),
          (
            "unescape".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::unescape,
            }),
          ),
          (
            "zephyr_to_json_n".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::zephyr_to_json_n,
            }),
          ),
          (
            "get_time_nanos".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::get_time_nanos,
            }),
          ),
          (
            "floor".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::floor,
            }),
          ),
          (
            "slice".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::slice,
            }),
          ),
          (
            "ceil".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::ceil,
            }),
          ),
          (
            "parse_json".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::json_parse,
            }),
          ),
          (
            "read_file".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::read_file,
            }),
          ),
          (
            "push_arr".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::push_arr,
            }),
          ),
          (
            "get_args".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::get_args,
            }),
          ),
          (
            "random".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::random,
            }),
          ),
          (
            "call_zephyr_function".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::call_zephyr_function,
            }),
          ),
          (
            "random_item".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::random_item,
            }),
          ),
          (
            "random_range".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::random_range,
            }),
          ),
        ]),
      }));

    scope
      .declare_variable(
        "__zephyr_native",
        RuntimeValue::ObjectContainer(ObjectContainer {
          location: native_address,
        }),
      )
      .unwrap();

    // Create symbol object
    let mut items: HashMap<String, RuntimeValue> = HashMap::new();
    for (key, val) in SYMBOLS.clone() {
      items.insert(key.to_string(), StringValue::make(val.to_string()));
    }
    match scope.declare_variable("Symbols", Object::make(items).create_container()) {
      Err(_) => panic!("FAILED TO LOAD SYMBOLS!"),
      Ok(_) => (),
    };

    // Load libs
    for i in libs {
      let lib_scope = scope.create_child().unwrap();
      lib_scope.set_can_export(true).unwrap();
      let mut lib_InterpreterOld = InterpreterOld {
        scope: lib_scope,
        global_scope: scope,
      };
      match lib_interpreter.evaluate(Expression::Program(
        match Parser::new(
          lex(
            String::from(i.0),
            format!("(lib){}", i.1.replace("../lib/", "")),
          )
          .unwrap(),
        )
        .produce_ast(Some(format!("<lib {}>", i.1.replace("../lib/", ""))))
        {
          Ok(ok) => ok,
          Err(err) => {
            println!(
              "ERROR WHILE PARSEING LIB (PARSER): {}\n\n{}",
              i.1,
              err.visualise(false)
            );
            panic!()
          }
        },
      )) {
        Ok(_) => (),
        Err(err) => println!(
          "ERROR WHILE PARSEING LIB (RUNTIME): {}\n\n{}",
          i.1,
          err.visualise(false)
        ),
      };

      crate::verbose(&format!("Finished loading lib: {}", i.1), "lib-loader");

      // Add defined variables to global
      for (key, value) in lib_scope.get_exports().unwrap().iter() {
        scope.declare_variable_with_address(key, *value).unwrap();
      }
    }

    let s = scope.create_child().unwrap();
    s.set_can_export(true).unwrap();

    crate::debug(
      &format!("Took {}ms to load interpreter", start.elapsed().as_millis()),
      "interpreter",
    );

    InterpreterOld {
      global_scope: scope,
      scope: s,
    }
  }

  pub fn get_proto(
    &mut self,
    value: RuntimeValue,
    key: String,
  ) -> Result<Option<Function>, ZephyrError> {
    let value = match match value {
      RuntimeValue::StringValue(_) => Some(self.global_scope.get_variable("String")?),
      RuntimeValue::Number(_) => Some(self.global_scope.get_variable("Number")?),
      RuntimeValue::ArrayContainer(_) => Some(self.global_scope.get_variable("Array")?),
      RuntimeValue::ObjectContainer(_) => Some(self.global_scope.get_variable("Object")?),
      RuntimeValue::Function(_) => Some(self.global_scope.get_variable("Function")?),
      _ => None,
    } {
      Some(s) => match s {
        RuntimeValue::ObjectContainer(obj) => {
          match crate::MEMORY.lock().unwrap().get_value(obj.location)? {
            RuntimeValue::Object(obj) => obj,
            _ => unreachable!(),
          }
        }
        _ => unreachable!(),
      },
      None => return Ok(None),
    };

    let any = match self.global_scope.get_variable("Any")? {
      s => match s {
        RuntimeValue::ObjectContainer(obj) => {
          match crate::MEMORY.lock().unwrap().get_value(obj.location)? {
            RuntimeValue::Object(obj) => obj,
            _ => unreachable!(),
          }
        }
        _ => unreachable!(),
      },
    };

    if !value.items.contains_key(&key) {
      // Check if Any contains it
      if any.items.contains_key(&key) {
        Ok(Some(match any.items.get(&key).unwrap() {
          RuntimeValue::Function(func) => func.clone(),
          _ => unreachable!(),
        }))
      } else {
        Ok(None)
      }
    } else {
      Ok(Some(match value.items.get(&key).unwrap() {
        RuntimeValue::Function(func) => func.clone(),
        _ => unreachable!(),
      }))
    }
  }

  pub fn evaluate_block(
    &mut self,
    block: Block,
    scope: ScopeContainer,
  ) -> Result<RuntimeValue, ZephyrError> {
    let prev_scope = std::mem::replace(&mut self.scope, scope);
    let mut last_value: Option<RuntimeValue> = None;

    for expr in block.nodes {
      last_value = Some(match self.evaluate(*expr) {
        Ok(val) => val,
        Err(err) => {
          let _ = std::mem::replace(&mut self.scope, prev_scope);
          return Err(err);
        }
      });
    }

    let _ = std::mem::replace(&mut self.scope, prev_scope);

    match last_value {
      None => Ok(RuntimeValue::Null(Null {})),
      Some(val) => Ok(val),
    }
  }

  pub fn expr_to_runtime(
    &mut self,
    args: Vec<Box<Expression>>,
  ) -> Result<Vec<Box<RuntimeValue>>, ZephyrError> {
    let mut evalled_args: Vec<Box<RuntimeValue>> = vec![];
    for i in args {
      evalled_args.push(Box::from(self.evaluate(*i)?))
    }

    Ok(evalled_args)
  }

  pub fn is_equal(
    left: &RuntimeValue,
    right: &RuntimeValue,
    expr: Option<ComparisonExpression>,
  ) -> Result<bool, ZephyrError> {
    let operator = if let Some(_expr) = expr.clone() {
      _expr.operator
    } else {
      ComparisonTokenType::Equals
    };
    let location = if let Some(_expr) = expr.clone() {
      _expr.location
    } else {
      Location::no_location()
    };

    // Check if they are the same type
    if !util::varient_eq(&left.clone(), &right.clone()) {
      return Ok(false);
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

      if let Some(_expr) = expr {
        result = match _expr.operator {
          ComparisonTokenType::Equals => left_number == right_number,
          ComparisonTokenType::NotEquals => left_number == right_number,
          ComparisonTokenType::GreaterThan => left_number > right_number,
          ComparisonTokenType::GreaterThanOrEquals => left_number >= right_number,
          ComparisonTokenType::LessThan => left_number < right_number,
          ComparisonTokenType::LessThanOrEquals => left_number <= right_number,
        };
      } else {
        result = left_number == right_number;
      }
    } else if matches!(expr, None)
      || matches!(expr.clone().unwrap().operator, ComparisonTokenType::Equals)
      || matches!(
        expr.clone().unwrap().operator,
        ComparisonTokenType::NotEquals
      )
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
      // Arrays
      else if matches!(left, RuntimeValue::ArrayContainer(_)) {
        let left_bool = match left {
          RuntimeValue::ArrayContainer(arr) => arr,
          _ => unreachable!(),
        };
        let right_bool = match right {
          RuntimeValue::ArrayContainer(arr) => arr,
          _ => unreachable!(),
        };

        if left_bool.location == right_bool.location {
          result = true;
        }
      }
      // Object
      else if matches!(left, RuntimeValue::ObjectContainer(_)) {
        println!("{:?} {:?}", left, right);
        let left_bool = match left {
          RuntimeValue::ObjectContainer(obj) => obj,
          _ => unreachable!(),
        };
        let right_bool = match right {
          RuntimeValue::ObjectContainer(obj) => obj,
          _ => unreachable!(),
        };

        if left_bool.location == right_bool.location {
          result = true;
        }
      }
      // Null
      else if matches!(left, RuntimeValue::Null(_)) {
        // This will always be true
        result = true;
      } else {
        return Err(ZephyrError::parser(
          format!("Cannot handle {} {} {}", left, operator, right),
          location,
        ));
      }
    } else {
      return Err(ZephyrError::parser(
        format!("Cannot handle {} {} {}", left, operator, right),
        location,
      ));
    }

    Ok(result)
  }

  pub fn evaluate_zephyr_function(
    &mut self,
    func: Function,
    arguments: Vec<Box<RuntimeValue>>,
    location: Location,
  ) -> Result<RuntimeValue, ZephyrError> {
    let start_time = std::time::Instant::now();
    /*if self.scope.get_pure_functions_only()? {
      self.scope.set_pure_functions_only(false)?;
      // Check if it is pure
      if !func.pure {
        return Err(ZephyrError::parser(
          "Called a non pure function, but the current scope is in pure mode".to_string(),
          location.clone(),
        ));
      }
    }*/
    // Get the scope
    let scope = if func.pure {
      self.global_scope.create_child()?
    } else {
      func.scope.create_child()?
    };
    let caller_args = arguments.clone();

    /*if func.pure {
      scope.set_pure_functions_only(true)?;
    }*/

    let evalled_args: Vec<Box<RuntimeValue>> = caller_args;

    // Declare the args
    for i in 0..func.arguments.len() {
      // Check if it is __args__
      if func.arguments[i].symbol == "__args__" {
        scope.declare_variable(
          "__args__",
          Array::make(evalled_args.clone()).create_container(),
        )?;
        continue;
      }
      let assigned = if i < arguments.len() {
        *evalled_args.get(i).unwrap().clone()
      } else {
        RuntimeValue::Null(Null {})
      };

      scope.declare_variable(&func.arguments.get(i).unwrap().clone().symbol, assigned)?;
    }

    // Check where clauses
    crate::verbose(
      &format!("Swapping scope from {} to {}", self.scope.id, scope.id),
      "scope",
    );
    let prev = std::mem::replace(&mut self.scope, scope);
    /*{
      self.scope.set_pure_functions_only(true)?;
    }*/
    for clause in func.where_clause.tests {
      let res = match self.evaluate((*clause).clone()) {
        Ok(ok) => ok,
        Err(err) => {
          // Cleanup
          {
            self.scope.set_pure_functions_only(func.pure)?;
            crate::verbose(
              &format!(
                "Swapping scope back from {} to {} due to error",
                self.scope.id, prev.id
              ),
              "scope",
            );
            let _ = std::mem::replace(&mut self.scope, prev);
          }
          return Err(err);
        }
      };

      // Check if it succeeded
      if !res.is_truthy() {
        return Err(ZephyrError::runtime_with_ref(
          format!("Call failed to pass where clauses, received: {}", res),
          location,
          clause.get_location(),
        ));
      }
    }
    /*{
      self.scope.set_pure_functions_only(func.pure)?;
    }*/

    let return_value = match self.evaluate_block(*func.body, scope) {
      Ok(ok) => ok,
      Err(err) => match err.error_type {
        ErrorType::Return(val) => *val,
        _ => return Err(err),
      },
    };
    crate::verbose(
      &format!("Swapping scope back from {} to {}", self.scope.id, prev.id),
      "scope",
    );
    let _ = std::mem::replace(&mut self.scope, prev);

    // Check if it is predicate
    if let Some(ref f) = func.name {
      // Check endswith ?
      if f.ends_with('?') {
        // Check for boolean
        if !matches!(return_value, RuntimeValue::Boolean(_)) {
          return Err(ZephyrError::runtime(
            "Predicate function can only return booleans".to_string(),
            Location::no_location(),
          ));
        }
      }
    }

    // Check for debug
    let end_time = start_time.elapsed();
    if crate::ARGS.function_evaluation_times {
      // Check if it has it
      let name = &format!(
        "#f#{}##{}",
        func.name.clone().unwrap_or("?anonymous".to_string()),
        func
          .scope
          .get_file()?
          .unwrap_or("<unknown origin>".to_string())
      );

      // Check if thing contains if
      if !NODE_EVALUATION_TIMES.lock().unwrap().contains_key(name) {
        NODE_EVALUATION_TIMES
          .lock()
          .unwrap()
          .insert(name.to_string(), vec![]);
      }

      // Add the time to it
      NODE_EVALUATION_TIMES
        .lock()
        .unwrap()
        .get_mut(name)
        .unwrap()
        .push(end_time.as_millis());
    }

    Ok(return_value)
  }

  pub fn evaluate_identifier(
    &mut self,
    ident: Identifier,
    skip_getter: bool,
  ) -> Result<RuntimeValue, ZephyrError> {
    let variable = self.scope.get_variable(&ident.symbol)?;

    // Check if object & has __get
    if !skip_getter {
      if let RuntimeValue::ObjectContainer(obj) = variable.clone() {
        let obj = match crate::MEMORY.lock().unwrap().get_value(obj.location)? {
          RuntimeValue::Object(obj) => obj,
          _ => unreachable!(),
        };

        // Check if it has __get
        if obj.items.contains_key("__get") {
          let func = match obj.items.get_key_value("__get").unwrap().1 {
            RuntimeValue::Function(func) => func,
            _ => {
              return Err(ZephyrError::runtime(
                "Expected function for __get".to_string(),
                ident.location,
              ))
            }
          };
          return self.evaluate_zephyr_function(func.clone(), vec![], ident.location);
        }
      }
    }

    Ok(variable)
  }

  pub fn evaluate_member_expression(
    &mut self,
    expr: MemberExpression,
    assign: &Option<RuntimeValue>,
  ) -> Result<RuntimeValue, errors::ZephyrError> {
    let key_loc = expr.key.get_location();
    let value = match *expr.left.clone() {
      Expression::Identifier(ident) => self.evaluate_identifier(ident, true),
      _ => self.evaluate(*expr.left.clone()),
    }?;

    // Get key
    let mut key = if expr.is_computed {
      Some(self.evaluate((*expr.key).clone())?)
    } else {
      None
    };

    // Check if has proto thing
    let thing = self.get_proto(
      value.clone(),
      match key {
        Some(ref val) => match val {
          RuntimeValue::StringValue(s) => s.value.clone(),
          _ => "".to_string(),
        },
        None => match (*expr.key).clone() {
          Expression::Identifier(ident) => ident.symbol,
          _ => "".to_string(),
        },
      },
    )?;

    // Check if it actually had the type function
    if let Some(func) = thing {
      let mut mutfunc = func;
      mutfunc.type_call = Some(Box::from(value.clone()));
      return Ok(RuntimeValue::Function(mutfunc));
    }

    if matches!(key, None) {
      key = match (*expr.key).clone() {
        Expression::Identifier(ident) => Some(StringValue::make(ident.symbol)),
        _ => None,
      };
    }

    match value {
      RuntimeValue::ArrayContainer(arr_ref) => {
        // Make sure it is is_computed
        if !expr.is_computed {
          return Err(ZephyrError::runtime(
            "Expected computed expression here".to_string(),
            key_loc,
          ));
        }

        // Get the referenced array
        let mut arr = match crate::MEMORY.lock().unwrap().get_value(arr_ref.location)? {
          RuntimeValue::Array(arr) => arr,
          _ => unreachable!(),
        };

        // Can only index via numbers
        let number = match key {
          Some(RuntimeValue::Number(num)) => num.value as usize,
          Some(RuntimeValue::ArrayContainer(array)) => {
            let array2 = match crate::MEMORY.lock().unwrap().get_value(array.location)? {
              RuntimeValue::Array(arr) => arr,
              _ => unreachable!(),
            };

            let mut result: Vec<Box<RuntimeValue>> = vec![];

            // Check if all items are number
            for i in array2.items {
              match *i {
                RuntimeValue::Number(num) => {
                  // Check if original array has this value
                  if arr.items.len() <= num.value as usize {
                    return Err(ZephyrError::runtime(
                      format!("Index out of bounds {}", num.value),
                      Location::no_location(),
                    ));
                  }

                  result.push(arr.items.get(num.value as usize).unwrap().clone());
                }
                _ => {
                  return Err(ZephyrError::runtime_with_ref(
                    "To index an array with an array, all items must be of type number".to_string(),
                    expr.key.get_location(),
                    expr.left.get_location(),
                  ))
                }
              }
            }

            // Add to memory
            let address = crate::MEMORY
              .lock()
              .unwrap()
              .add_value(RuntimeValue::Array(Array { items: result }));

            // Finish
            return Ok(RuntimeValue::ArrayContainer(ArrayContainer {
              location: address,
            }));
          }
          _ => {
            return Err(errors::ZephyrError::runtime(
              format!(
                "Can only index array with numbers, but got {}",
                key.unwrap().type_name()
              ),
              Location::no_location(),
            ))
          }
        };

        // Check if out of bounds
        if arr.items.len() <= number {
          // Check if it is assignment
          if assign.is_some() {
            // Only allow +1
            if arr.items.len() + 1 < number {
              return Err(ZephyrError::runtime(
                "Index out of bounds".to_string(),
                key_loc,
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
            location: crate::MEMORY
              .lock()
              .unwrap()
              .set_value(arr_ref.location, RuntimeValue::Array(arr))?,
          }));
        }

        // Return
        Ok(*(arr.items[number]).clone())
      }
      RuntimeValue::ObjectContainer(obj_ref) => {
        let mut object = match crate::MEMORY.lock().unwrap().get_value(obj_ref.location)? {
          RuntimeValue::Object(obj) => obj,
          _ => unreachable!(),
        };

        let string_key = match key {
          Some(RuntimeValue::StringValue(num)) => num.value,
          None => match (*expr.key).clone() {
            Expression::Identifier(ident) => ident.symbol,
            _ => unreachable!(),
          },
          _ => {
            return Err(errors::ZephyrError::runtime(
              format!(
                "Can only index object with strings, but got {}",
                key.unwrap().type_name()
              ),
              key_loc,
            ))
          }
        };

        // Check if should assign
        if let Some(to_assign) = assign {
          // Check if already has it
          if object.items.contains_key(&string_key) {
            object.items.remove(&string_key);
          }
          object.items.insert(string_key, to_assign.clone());

          // Update memory
          crate::MEMORY
            .lock()
            .unwrap()
            .set_value(obj_ref.location, RuntimeValue::Object(object))?;

          return Ok(RuntimeValue::ObjectContainer(obj_ref));
        }

        // Check if object has the item defined
        if object.items.contains_key(&string_key) {
          Ok((*object.items.get(&string_key).unwrap()).clone())
        } else {
          Err(errors::ZephyrError::runtime(
            format!("Object does not contain definition for key {}", string_key),
            key_loc,
          ))
        }
      }
      RuntimeValue::StringValue(str) => {
        // Can only index via numbers
        let number = match key {
          Some(RuntimeValue::Number(num)) => num.value as usize,
          _ => {
            return Err(errors::ZephyrError::runtime(
              format!("Can only index array with numbers, but got {:?}", key),
              expr.location,
            ));
          }
        };

        if str.value.len() <= number {
          return Err(errors::ZephyrError::runtime(
            "Index out of bounds".to_string(),
            Location::no_location(),
          ));
        }

        Ok(RuntimeValue::StringValue(StringValue {
          value: str.value.chars().nth(number).unwrap().to_string(),
        }))
      }
      _ => Err(errors::ZephyrError::runtime(
        format!("Cannot index a {}", value.type_name()),
        Location::no_location(),
      )),
    }
  }

  pub fn evaluate(&mut self, _node: Expression) -> Result<RuntimeValue, errors::ZephyrError> {
    let start_time = std::time::Instant::now();
    let node = &_node;
    let result = match node.clone() {
      /////////////////////////////////
      // ----- Special Things ----- //
      ///////////////////////////////
      Expression::Program(program) => {
        let mut last_value: Option<RuntimeValue> = None;
        self.scope.set_file(program.file)?;

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

      Expression::Block(block) => self.evaluate_block(block, self.scope.create_child()?),

      /////////////////////////////
      // ----- Statements ----- //
      ///////////////////////////
      Expression::ThrowStatement(stmt) => {
        let value = self.evaluate(*stmt.what.clone())?;

        Err(match value {
          RuntimeValue::ErrorValue(old_err) => {
            let mut err = old_err.error.clone();
            err.backtrace.push(stmt.location);
            err
          }
          val => ZephyrError {
            error_message: "An exception was thrown".to_string(),
            location: stmt.what.get_location(),
            error_type: ErrorType::UserDefined(Box::from(val)),
            reference: None,
            backtrace: vec![],
          },
        })
        // Get the error_struct? func
        /*let error_struct = match self.global_scope.get_variable("error_struct?")? {
          RuntimeValue::Function(func) => func,
          _ => unreachable!(),
        };

        // Evaluate it
        match self.evaluate_zephyr_function(
          error_struct,
          vec![Box::new(value.clone())],
          stmt.location,
        )? {
          RuntimeValue::Boolean(bool) => {
            if !bool.value {
              return Err(ZephyrError::runtime(
                "Threw invalid error obj".to_string(),
                stmt.location,
              ));
            } else {
            }
          }
          _ => unreachable!(),
        };

        let obj_value = match &value {
          RuntimeValue::ObjectContainer(cont) => {
            match crate::MEMORY.lock().unwrap().get_value(cont.location)? {
              RuntimeValue::Object(obj) => obj,
              _ => unreachable!(),
            }
          }
          _ => unreachable!(),
        };

        let message = match obj_value.items.get("message") {
          Some(RuntimeValue::StringValue(str)) => str.value.clone(),
          _ => unreachable!(),
        };

        let _typ = match obj_value.items.get("type") {
          Some(RuntimeValue::StringValue(str)) => str.value.clone(),
          _ => unreachable!(),
        };

        let data = match obj_value.items.get("data") {
          Some(val) => val.clone(),
          _ => RuntimeValue::Null(Null {}),
        };

        return Err(ZephyrError {
          location: stmt.location,
          error_message: message,
          error_type: ErrorType::UserDefined(Box::from(data)),
          backtrace: vec![],
          reference: None,
        });*/
      }
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
      Expression::BreakStatement(stmt) => Err(ZephyrError {
        error_message: "Cannot break here".to_string(),
        error_type: ErrorType::Break(if let Some(ident) = stmt.name {
          Some(ident.symbol)
        } else {
          None
        }),
        backtrace: vec![],
        reference: None,
        location: stmt.location,
      }),
      Expression::ContinueStatement(stmt) => Err(ZephyrError {
        error_message: "Cannot continue here".to_string(),
        error_type: ErrorType::Continue(if let Some(ident) = stmt.name {
          Some(ident.symbol)
        } else {
          None
        }),
        backtrace: vec![],
        reference: None,
        location: stmt.location,
      }),
      Expression::ReturnStatement(stmt) => Err(ZephyrError {
        error_message: "Cannot return here".to_string(),
        error_type: ErrorType::Return(if let Some(ret) = stmt.value {
          Box::from(self.evaluate(*ret)?)
        } else {
          Box::from(RuntimeValue::Null(Null {}))
        }),
        backtrace: vec![],
        reference: None,
        location: stmt.location,
      }),
      Expression::AssertStatement(stmt) => {
        let value = self.evaluate(*stmt.value)?;
        if !value.is_truthy() {
          return Err(ZephyrError::runtime(
            "Expression did not pass assertion, expected truthy value".to_string(),
            stmt.location,
          ));
        }

        Ok(RuntimeValue::Null(Null {}))
      }
      Expression::ImportStatement(stmt) => {
        // Create path
        let mut path = util::path_resolver::resolve(
          PathBuf::from(self.scope.clone().get_directory().unwrap()),
          &stmt.from.value.clone(),
        )?;

        crate::debug(&format!("Importing {}", path.display()), "import");

        // Check if it exists
        if !path.exists() {
          return Err(ZephyrError::runtime(
            format!(
              "Failed to import {} because the file does not exist",
              path.display()
            ),
            stmt.from.location,
          ));
        }

        // Check if it is file
        if !path.is_file() {
          return Err(ZephyrError::runtime(
            format!(
              "Failed to import {} because the path is not a file",
              path.display()
            ),
            stmt.from.location,
          ));
        }

        let path_string = path.display().to_string();

        // Read it
        let file_contents = match std::fs::read_to_string(path_string.clone()) {
          Ok(ok) => ok,
          Err(err) => {
            return Err(ZephyrError::runtime(
              format!("Failed to read file: {}", err),
              stmt.from.location,
            ));
          }
        };

        // Check if cache has it
        let scope = if IMPORT_CACHE.lock().unwrap().contains_key(&(path_string)) {
          crate::verbose(&format!("Importing from cache {}", path_string), "import");
          *IMPORT_CACHE.lock().unwrap().get(&path_string).unwrap()
        } else {
          crate::verbose(&format!("Importing {}", path_string), "import");

          // Lex & Parse
          let result = lexer::lexer::lex(file_contents, path_string.clone())?;
          let mut parser = parser::parser::Parser::new(result);
          let ast = parser.produce_ast(Some(path_string.clone()))?;

          // Create scope
          let path_pre_pop = path_string.clone();
          path.pop();
          let scope = self.global_scope.create_child()?;
          scope.set_directory(path.display().to_string())?;
          scope.set_can_export(true)?;

          // Evaluate it
          let prev = std::mem::replace(&mut self.scope, scope);
          self.evaluate(Expression::Program(ast))?;
          let _ = std::mem::replace(&mut self.scope, prev);

          // Set cache
          IMPORT_CACHE.lock().unwrap().insert(path_pre_pop, scope);
          scope
        };

        for i in stmt.import {
          let to_import = i.0.symbol;
          let import_as = i.1.symbol;

          // Check if it is *
          if to_import == "*" {
            let exports = scope.get_exports()?;
            let mut object = Object {
              items: HashMap::new(),
            };

            for i in exports {
              object
                .items
                .insert(i.0, crate::MEMORY.lock().unwrap().get_value(i.1)?);
            }

            // Add to memory
            let obj = RuntimeValue::ObjectContainer(ObjectContainer {
              location: crate::MEMORY
                .lock()
                .unwrap()
                .add_value(RuntimeValue::Object(object)),
            });

            self.scope.declare_variable(&import_as, obj)?;
            continue;
          }

          // Check if scope contains it
          if !scope.has_variable(&to_import)? {
            return Err(ZephyrError::runtime(
              format!("The import does not contain a definition for {}", to_import),
              i.0.location,
            ));
          }

          // Define it
          self
            .scope
            .declare_variable(&import_as, scope.get_variable(&to_import)?)?;
        }

        Ok(RuntimeValue::Null(Null {}))
      }
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
          _ => unimplemented!(),
        }
        Ok(RuntimeValue::Null(Null {}))
      }

      ///////////////////////////
      // ----- Literals ----- //
      /////////////////////////
      Expression::NumericLiteral(literal) => Ok(RuntimeValue::Number(Number {
        value: literal.value,
      })),
      Expression::Identifier(ident) => self.evaluate_identifier(ident, false),
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
        let address = crate::MEMORY
          .lock()
          .unwrap()
          .add_value(RuntimeValue::Object(object));

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
        let address = crate::MEMORY
          .lock()
          .unwrap()
          .add_value(RuntimeValue::Array(array));

        // Finish
        Ok(RuntimeValue::ArrayContainer(ArrayContainer {
          location: address,
        }))
      }
      Expression::FunctionLiteral(stmt) => {
        //let _name = stmt.identifier.symbol;
        let child_scope = self.scope.create_child()?;

        if stmt.is_pure {
          child_scope.set_pure_functions_only(true)?;
        }

        let function = Function {
          scope: child_scope,
          body: stmt.body,
          arguments: stmt.arguments,
          where_clause: stmt.where_clauses,
          name: match stmt.identifier {
            Some(ident) => Some(ident.symbol),
            None => None,
          },
          pure: stmt.is_pure,
          type_call: None,
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
      Expression::InExpression(expr) => {
        let left = self.evaluate(*expr.left.clone())?;
        let right = self.evaluate(*expr.right.clone())?;

        Ok(RuntimeValue::Boolean(Boolean {
          value: match right {
            RuntimeValue::ObjectContainer(obj) => {
              let obj = match crate::MEMORY.lock().unwrap().get_value(obj.location)? {
                RuntimeValue::Object(obj) => obj,
                _ => unreachable!(),
              };
              match left {
                RuntimeValue::StringValue(str) => obj.items.contains_key(&str.value),
                _ => {
                  return Err(ZephyrError::runtime(
                    format!("Cannot check if object has {}", left.type_name()),
                    expr.left.get_location(),
                  ))
                }
              }
            }
            RuntimeValue::ArrayContainer(arr) => {
              let arr: Array = match crate::MEMORY.lock().unwrap().get_value(arr.location)? {
                RuntimeValue::Array(arr) => arr,
                _ => unreachable!(),
              };

              let mut contains = false;

              for i in arr.items {
                if Interpreter::is_equal(&*i, &left, None)? {
                  contains = true;
                  break;
                }
              }

              contains
            }
            _ => {
              return Err(ZephyrError::runtime(
                format!("Cannot use in with {}", right.type_name()),
                expr.right.get_location(),
              ))
            }
          },
        }))
      }
      Expression::CallExpression(expr) => {
        let expr2 = expr.clone();

        // Get the function to call
        let callee = self.evaluate(*expr.left)?;

        // Check which type of function it is
        let result = match callee {
          // Normal native function
          RuntimeValue::NativeFunction(func) => {
            // Collect arguments
            let caller_args = expr2.arguments.clone();
            let args = caller_args
              .iter()
              .map(|e| self.evaluate(*e.clone()))
              .collect::<Result<Vec<_>, _>>()?;

            // Call it
            (func.func)(CallOptions {
              args,
              location: expr2.location,
              interpreter: *self,
            })
          }

          // Arc'd function
          RuntimeValue::NativeFunction2(func) => {
            // Collect arguments
            let caller_args = expr2.arguments.clone();
            let args = caller_args
              .iter()
              .map(|e| self.evaluate(*e.clone()))
              .collect::<Result<Vec<_>, _>>()?;

            // Call it
            (func.func)(CallOptions {
              args,
              location: expr2.location,
              interpreter: *self,
            })
          }
          RuntimeValue::Function(func) => {
            // Collect arguments
            let mut args = self.expr_to_runtime(expr2.arguments)?;
            if let Some(t) = func.clone().type_call {
              args.insert(0, t);
            }

            // Collect it
            self.evaluate_zephyr_function(func, args, expr2.left.get_location())
          }
          _ => Err(runtime_error!("Expected a function to call".to_string())),
        };

        match result {
          Ok(ok) => Ok(ok),
          Err(mut err) => {
            err.backtrace.push(_node.get_location());
            Err(err)
          }
        }
      }
      Expression::AssignmentExpression(expr) => {
        let right = self.evaluate(*expr.right)?;

        match *expr.left {
          Expression::MemberExpression(mem) => self.evaluate_member_expression(mem, &Some(right)),
          Expression::Identifier(ident) => self.scope.modify_variable(&ident.symbol, right),
          _ => unimplemented!(),
        }
      }
      Expression::MemberExpression(expr) => self.evaluate_member_expression(expr, &None),
      Expression::RangeExpression(expr) => {
        let mut array: Vec<Box<RuntimeValue>> = vec![];

        let start = match self.evaluate(*expr.clone().from)? {
          RuntimeValue::Number(num) => num.value,
          _ => {
            return Err(ZephyrError::runtime(
              "Expected number for from in range".to_string(),
              expr.from.get_location(),
            ))
          }
        };

        let end = match self.evaluate(*expr.clone().to)? {
          RuntimeValue::Number(num) => num.value,
          _ => {
            return Err(ZephyrError::runtime(
              "Expected number for to in range".to_string(),
              expr.to.get_location(),
            ))
          }
        };

        let step_expr = if let Some(step) = expr.clone().step {
          Some(match self.evaluate(*step.clone())? {
            RuntimeValue::Number(num) => num.value,
            _ => {
              return Err(ZephyrError::runtime(
                "Expected number for step in range".to_string(),
                step.get_location(),
              ))
            }
          })
        } else {
          None
        };

        let step = if let Some(s) = step_expr {
          s
        } else if start > end {
          -1.0
        } else {
          1.0
        };

        let one_step = start + step;

        // Check if inf loop
        if (start > one_step) && (start < end) {
          return Err(ZephyrError::parser(
            "This range will result in an infinite loop".to_string(),
            expr.location,
          ));
        }

        let mut st = start;

        if st < end {
          while st < end + ((if expr.uninclusive { 0 } else { 1 }) as f64) {
            array.push(Box::from(RuntimeValue::Number(Number { value: st })));
            st += step;
          }
        } else {
          while st > end - ((if expr.uninclusive { 0 } else { 1 }) as f64) {
            array.push(Box::from(RuntimeValue::Number(Number { value: st })));
            st += step;
          }
        }

        Ok(Array::make(array).create_container())
      }
      Expression::ArithmeticOperator(expr) => {
        // Collect values
        let left = self.evaluate(*expr.left.clone())?;
        let right = self.evaluate(*expr.right.clone())?;

        // Check if the operator takes in stuff other than numbers
        match expr.operator {
          TokenType::MultiplicativeOperator(MultiplicativeTokenType::Coalesce) => {
            return Ok({
              if left.type_name() == "null" {
                right
              } else {
                left
              }
            })
          }
          _ => (),
        };

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
          TokenType::MultiplicativeOperator(MultiplicativeTokenType::Multiply) => match left {
            RuntimeValue::StringValue(ref left_value) => match right.clone() {
              RuntimeValue::Number(num) => Some(RuntimeValue::StringValue(StringValue {
                value: left_value.value.clone().repeat(num.value as usize),
              })),
              _ => None,
            },
            _ => None,
          },
          TokenType::AdditiveOperator(AdditiveTokenType::Plus) => match left {
            RuntimeValue::StringValue(ref left_value) => {
              let right_value: Option<String> = match right.clone() {
                RuntimeValue::StringValue(ref string_value) => {
                  Some(String::from(&*string_value.value))
                }
                RuntimeValue::Number(ref number_value) => Some(number_value.value.to_string()),
                RuntimeValue::Boolean(ref bool_value) => Some(bool_value.value.to_string()),
                RuntimeValue::Null(_) => Some("null".to_string()),
                _ => {
                  return Err(ZephyrError::runtime_with_ref(
                    format!("Cannot coerce a {} to a string", right.type_name()),
                    expr.location,
                    expr.right.get_location(),
                  ))
                }
              };

              right_value.map(|val| {
                RuntimeValue::StringValue(StringValue {
                  value: String::from(&*left_value.value) + &*val,
                })
              })
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
              right.type_name()
            ),
            expr.location,
          )),
        }
      }
      Expression::ComparisonOperator(expr) => {
        // Collect values
        let left = self.evaluate(*expr.clone().left)?;
        let right = self.evaluate(*expr.clone().right)?;

        let mut result = Interpreter::is_equal(&left, &right, Some(expr.clone()))?;

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

        Ok(match expr.operator {
          LogicalTokenType::And => RuntimeValue::Boolean(Boolean {
            value: left.is_truthy() && right.is_truthy(),
          }),
          LogicalTokenType::Or => {
            if left.is_truthy() {
              left
            } else if right.is_truthy() {
              right
            } else {
              left
            }
          }
        })
      }
      Expression::TypeofExpression(expr) => Ok(RuntimeValue::StringValue(StringValue {
        value: match self.evaluate(*expr.value) {
          Ok(val) => val.type_name().to_string(),
          Err(err) => return Err(err),
        },
      })),
      Expression::UnaryRightExpression(expr) => {
        let expr_value = *expr.value.clone();
        let operator = expr.operator;

        match operator {
          TokenType::UnaryOperator(UnaryOperator::Increment) => {
            // Expect an identifier (++a)
            match expr_value.clone() {
              Expression::Identifier(ident) => {
                // Get the variable and expect it to be a number
                let value = match self.scope.get_variable(&ident.symbol)? {
                  RuntimeValue::Number(num) => num.value,
                  val => {
                    return Err(ZephyrError::runtime(
                      format!(
                        "Expected a number to increment, but got {}",
                        val.type_name()
                      ),
                      expr_value.get_location(),
                    ))
                  }
                };

                self.scope.modify_variable(
                  &ident.symbol,
                  RuntimeValue::Number(Number { value: value + 1.0 }),
                )?;

                Ok(RuntimeValue::Number(Number { value: value }))
              }
              _ => {
                return Err(ZephyrError::runtime(
                  format!("Cannot increment a {:?}", expr_value),
                  expr_value.get_location(),
                ))
              }
            }
          }
          TokenType::UnaryOperator(UnaryOperator::Decrement) => {
            // Expect an identifier (--a)
            match expr_value.clone() {
              Expression::Identifier(ident) => {
                // Get the variable and expect it to be a number
                let value = match self.scope.get_variable(&ident.symbol)? {
                  RuntimeValue::Number(num) => num.value,
                  val => {
                    return Err(ZephyrError::runtime(
                      format!(
                        "Expected a number to decrement, but got {}",
                        val.type_name()
                      ),
                      expr_value.get_location(),
                    ))
                  }
                };

                self.scope.modify_variable(
                  &ident.symbol,
                  RuntimeValue::Number(Number { value: value - 1.0 }),
                )?;

                Ok(RuntimeValue::Number(Number { value: value }))
              }
              _ => {
                return Err(ZephyrError::runtime(
                  format!("Cannot decrement a {:?}", expr_value),
                  expr_value.get_location(),
                ))
              }
            }
          }
          _ => unimplemented!(),
        }
      }
      Expression::UnaryExpression(expr) => {
        let expr_value = *expr.value.clone();
        let value = self.evaluate(expr_value.clone())?;
        let operator = expr.operator;

        match operator {
          TokenType::AdditiveOperator(AdditiveTokenType::Minus) => match value {
            RuntimeValue::Number(number) => Ok(RuntimeValue::Number(Number {
              value: -number.value,
            })),
            _ => {
              return Err(ZephyrError::runtime(
                format!("Cannot negate a {}", value.type_name()),
                expr.location,
              ))
            }
          },
          TokenType::AdditiveOperator(AdditiveTokenType::Plus) => match value {
            RuntimeValue::Number(number) => Ok(RuntimeValue::Number(Number {
              value: number.value.abs(),
            })),
            _ => {
              return Err(ZephyrError::runtime(
                format!("Cannot negate a {}", value.type_name()),
                expr.location,
              ))
            }
          },
          TokenType::UnaryOperator(UnaryOperator::Not) | TokenType::Not => {
            Ok(RuntimeValue::Boolean(Boolean {
              value: !value.is_truthy(),
            }))
          }
          TokenType::UnaryOperator(UnaryOperator::Dereference) => match value {
            RuntimeValue::Number(num) => crate::MEMORY
              .lock()
              .unwrap()
              .get_value(num.value as MemoryAddress),
            RuntimeValue::ArrayContainer(arr) => {
              crate::MEMORY.lock().unwrap().get_value(arr.location)
            }
            _ => Err(ZephyrError::runtime(
              format!("Cannot derference a {:?}", value.type_name()),
              expr.value.get_location(),
            )),
          },
          TokenType::UnaryOperator(UnaryOperator::LengthOf) => Ok(RuntimeValue::Number(Number {
            value: value.iterate()?.len() as f64,
          })),
          TokenType::UnaryOperator(UnaryOperator::Increment) => {
            // Expect an identifier (++a)
            match expr_value.clone() {
              Expression::Identifier(ident) => {
                // Get the variable and expect it to be a number
                let value = match self.scope.get_variable(&ident.symbol)? {
                  RuntimeValue::Number(num) => num.value,
                  val => {
                    return Err(ZephyrError::runtime(
                      format!(
                        "Expected a number to increment, but got {}",
                        val.type_name()
                      ),
                      expr_value.get_location(),
                    ))
                  }
                };

                self.scope.modify_variable(
                  &ident.symbol,
                  RuntimeValue::Number(Number { value: value + 1.0 }),
                )?;

                Ok(RuntimeValue::Number(Number { value: value + 1.0 }))
              }
              _ => {
                return Err(ZephyrError::runtime(
                  format!("Cannot increment a {}", value.type_name()),
                  expr_value.get_location(),
                ))
              }
            }
          }
          TokenType::UnaryOperator(UnaryOperator::Decrement) => {
            // Expect an identifier (--a)
            match expr_value.clone() {
              Expression::Identifier(ident) => {
                // Get the variable and expect it to be a number
                let value = match self.scope.get_variable(&ident.symbol)? {
                  RuntimeValue::Number(num) => num.value,
                  val => {
                    return Err(ZephyrError::runtime(
                      format!(
                        "Expected a number to decrement, but got {}",
                        val.type_name()
                      ),
                      expr_value.get_location(),
                    ))
                  }
                };

                self.scope.modify_variable(
                  &ident.symbol,
                  RuntimeValue::Number(Number { value: value - 1.0 }),
                )?;

                Ok(RuntimeValue::Number(Number { value: value - 1.0 }))
              }
              _ => {
                return Err(ZephyrError::runtime(
                  format!("Cannot decrement a {}", value.type_name()),
                  expr_value.get_location(),
                ))
              }
            }
          }
          _ => unimplemented!(),
        }
      }

      // ----- Statement like expressions -----
      Expression::IfExpression(expr) => {
        let test = self.evaluate(*expr.test)?;

        if test.is_truthy() {
          // Run the success block
          self.evaluate_block(*expr.success, self.scope.create_child()?)
        } else if let Some(alt) = expr.alternate {
          self.evaluate(*alt)
        } else {
          Ok(RuntimeValue::Null(Null {}))
        }
      }
      Expression::WhileExpression(expr) => {
        // Should while loops return everything they did like for loops?
        // give your answer!!!!!!!!!!
        let mut iters = 0;
        while self.evaluate(*expr.test.clone())?.is_truthy() {
          iters += 1;
          let scope = self.scope.create_child()?;
          let res = self.evaluate_block(*expr.body.clone(), scope);
          scope.delete()?;

          // Check if continue or break
          if let Err(err) = res {
            match err.error_type {
              ErrorType::Break(ref _name) => {
                // Check if returned a name
                if let Some(name) = _name {
                  // Check if self has a name
                  if let Some(while_name) = expr.name.clone() {
                    // Check if same
                    if name.clone() == while_name.symbol {
                      break;
                    } else {
                      // Propogate
                      return Err(err.clone());
                    }
                  } else {
                    // Propogate
                    return Err(err.clone());
                  }
                } else {
                  break;
                }
              }
              ErrorType::Continue(ref _name) => {
                // Check if returned a name
                if let Some(name) = _name {
                  // Check if self has a name
                  if let Some(while_name) = expr.name.clone() {
                    // Check if same
                    if name.clone() == while_name.symbol {
                      continue;
                    } else {
                      // Propogate
                      return Err(err.clone());
                    }
                  } else {
                    // Propogate
                    return Err(err.clone());
                  }
                } else {
                  continue;
                }
              }
              _ => return Err(err),
            }
          }
        }

        // Check if should run else
        if let Some(el) = expr.none {
          if iters == 0 {
            self.evaluate(Expression::Block(*el))?;
          }
        }

        Ok(RuntimeValue::Null(Null {}))
      }
      Expression::TryExpression(expr) => {
        // Run the main block
        let result = match self.evaluate(Expression::Block(*expr.main)) {
          Ok(ok) => ok,
          Err(err) => {
            // Check if it is a return one
            match err.error_type {
              ErrorType::Continue(_) => return Err(err),
              ErrorType::Break(_) => return Err(err),
              ErrorType::Return(_) => return Err(err),
              _ => (),
            };
            // Check if there is a catch block
            if let Some(catch) = expr.catch {
              // Check if it defines ident
              let scope = self.scope.create_child()?;
              if let Some(ident) = expr.catch_identifier {
                scope.declare_variable(&ident.symbol, err.to_runtime_error())?;
              }
              self.evaluate_block(*catch, scope)?
            } else {
              RuntimeValue::Null(Null {})
            }
          }
        };

        // Check for finally
        if let Some(finally) = expr.finally {
          self.evaluate(Expression::Block(*finally))?;
        }

        Ok(result)
      }
      Expression::ForLoop(expr) => {
        let value = self.evaluate(*expr.value_to_iter)?.iterate()?;
        let mut values: Vec<Box<RuntimeValue>> = vec![];

        // Loop through the provided values
        for (i, v) in value.iter().enumerate() {
          // Create the scope and assign the loop variables
          let scope = self.scope.create_child()?;
          scope.declare_variable(&expr.index_identifier.symbol, Number::make(i as f64))?;
          if let Some(ref value_ident) = expr.value_identifier {
            scope.declare_variable(&value_ident.symbol, *v.clone())?;
          }

          let res = match self.evaluate_block(expr.body.clone(), scope) {
            Ok(ok) => ok,
            Err(err) => match err.error_type {
              ErrorType::Break(ref _name) => {
                // Check if returned a name
                if let Some(name) = _name {
                  // Check if self has a name
                  if let Some(while_name) = expr.name.clone() {
                    // Check if same
                    if name.clone() == while_name.symbol {
                      break;
                    } else {
                      // Propogate
                      return Err(err.clone());
                    }
                  } else {
                    // Propogate
                    return Err(err.clone());
                  }
                } else {
                  break;
                }
              }
              ErrorType::Continue(ref _name) => {
                // Check if returned a name
                if let Some(name) = _name {
                  // Check if self has a name
                  if let Some(while_name) = expr.name.clone() {
                    // Check if same
                    if name.clone() == while_name.symbol {
                      continue;
                    } else {
                      // Propogate
                      return Err(err.clone());
                    }
                  } else {
                    // Propogate
                    return Err(err.clone());
                  }
                } else {
                  continue;
                }
              }
              _ => return Err(err),
            },
          };
          values.push(Box::from(res));
          scope.delete()?;
        }

        // Check for else
        if values.is_empty() {
          if let Some(el) = expr.none {
            return Ok(self.evaluate(Expression::Block(*el)))?;
          }
        }

        return Ok(Array::make(values).create_container());
      }
      _ => Err(errors::ZephyrError::runtime(
        String::from("Cannot handle this AST node"),
        Location::no_location(),
      )),
    };

    // Check for debug
    let end_time = start_time.elapsed();
    if crate::ARGS.node_evaluation_times {
      // Check if it has it
      let val = format!("{:?}", node);
      let name = val.split("(").collect::<Vec<&str>>()[0];

      // Check if thing contains if
      if !NODE_EVALUATION_TIMES.lock().unwrap().contains_key(name) {
        NODE_EVALUATION_TIMES
          .lock()
          .unwrap()
          .insert(name.to_string(), vec![]);
      }

      // Add the time to it
      NODE_EVALUATION_TIMES
        .lock()
        .unwrap()
        .get_mut(name)
        .unwrap()
        .push(end_time.as_millis());
    }

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
