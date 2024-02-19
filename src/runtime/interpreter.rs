use std::{cell::RefCell, collections::HashMap, rc::Rc};

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
    nodes::{Block, Expression, Identifier, MemberExpression},
    parser::Parser,
  },
  util,
};

use super::{
  memory::MemoryAddress,
  native_functions,
  native_functions::CallOptions,
  scope::Scope,
  values::{
    to_array, to_object, Array, ArrayContainer, Boolean, Function, NativeFunction, Null, Number,
    Object, ObjectContainer, Reference, RuntimeValue, StringValue,
  },
};

#[path = "./handlers/mod.rs"]
pub mod handlers;

pub struct Interpreter {
  pub scope: Rc<Scope>,
  pub global_scope: Rc<Scope>,
  pub import_cache: RefCell<HashMap<String, Rc<Scope>>>,
}

macro_rules! include_lib {
  ($what:expr) => {
    (include_str!($what), $what)
  };
}

impl Interpreter {
  pub fn new(directory: String) -> Self {
    let libs: Vec<(&str, &str)> = vec![
      include_lib!("../lib/predicates.zr"),
      include_lib!("../lib/array.zr"),
      include_lib!("../lib/string.zr"),
      include_lib!("../lib/math.zr"),
      include_lib!("../lib/console.zr"),
      include_lib!("../lib/iter.zr"),
      include_lib!("../lib/time.zr"),
      include_lib!("../lib/object.zr"),
    ];
    let scope = &Rc::new(Scope::new(directory));

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
            "clear_console".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::clear_console,
            }),
          ),
          (
            "unescape".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::unescape,
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
            "ceil".to_string(),
            RuntimeValue::NativeFunction(NativeFunction {
              func: &native_functions::ceil,
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

    // Load libs
    for i in libs {
      let lib_scope = scope.clone().create_child();
      *lib_scope.can_export.borrow_mut() = true;
      let mut lib_interpreter = Interpreter {
        scope: lib_scope.clone(),
        global_scope: scope.clone(),
        import_cache: RefCell::from(HashMap::new()),
      };
      match lib_interpreter.evaluate(Expression::Program(
        match Parser::new(lex(String::from(i.0), format!("(lib){}", i.1)).unwrap()).produce_ast() {
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

      // Add defined variables to global
      for (key, value) in lib_scope.clone().exports.borrow().iter() {
        scope.variables.borrow_mut().insert((key).clone(), *value);
      }
    }

    let s = scope.create_child();
    *s.can_export.borrow_mut() = true;

    Interpreter {
      global_scope: scope.clone(),
      scope: s,
      import_cache: RefCell::from(HashMap::new()),
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

  pub fn evaluate_zephyr_function(
    &mut self,
    func: Function,
    arguments: Vec<Box<Expression>>,
    location: Location,
  ) -> Result<RuntimeValue, ZephyrError> {
    if *self.scope.pure_functions_only.borrow() {
      *self.scope.pure_functions_only.borrow_mut() = false;
      // Check if it is pure
      if !func.pure {
        return Err(ZephyrError::parser(
          "Called a non pure function, but the current scope is in pure mode".to_string(),
          location.clone(),
        ));
      }
    }
    // Get the scope
    let scope = if func.pure {
      self.global_scope.create_child()
    } else {
      func.scope.create_child()
    };
    let caller_args = arguments.clone();

    if func.pure {
      *scope.pure_functions_only.borrow_mut() = true;
    }

    let mut evalled_args: Vec<Box<RuntimeValue>> = vec![];
    for i in caller_args {
      evalled_args.push(Box::from(self.evaluate(*i)?));
    }

    // Declare the args
    for i in 0..func.arguments.len() {
      // Check if it is __args__
      if func.arguments[i].symbol == "__args__" {
        scope.declare_variable("__args__", to_array(evalled_args.clone()))?;
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
    crate::debug(
      &format!(
        "Swapping scope from {} to {}",
        self.scope.id,
        scope.clone().id
      ),
      "scope",
    );
    let prev = std::mem::replace(&mut self.scope, scope.clone());
    {
      *self.scope.pure_functions_only.borrow_mut() = true;
    }
    for clause in func.where_clause.tests {
      let res = match self.evaluate((**&clause).clone()) {
        Ok(ok) => ok,
        Err(err) => {
          // Cleanup
          {
            *self.scope.pure_functions_only.borrow_mut() = func.pure;
          }
          return Err(err);
        }
      };

      // Check if it succeeded
      if !res.is_truthy() {
        return Err(ZephyrError::runtime_with_ref(
          format!("Call failed to pass where clauses, received: {}", res),
          location,
          clause.clone().get_location().clone(),
        ));
      }
    }
    {
      *self.scope.pure_functions_only.borrow_mut() = func.pure;
    }

    let return_value = match self.evaluate_block(*func.body, scope) {
      Ok(ok) => ok,
      Err(err) => match err.error_type {
        ErrorType::Return(val) => *val,
        _ => return Err(err),
      },
    };
    crate::debug(
      &format!("Swapping scope back from {} to {}", self.scope.id, prev.id),
      "scope",
    );
    let _ = std::mem::replace(&mut self.scope, prev);

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

    Ok(variable.clone())
  }

  pub fn evaluate_member_expression(
    &mut self,
    expr: MemberExpression,
    assign: &Option<RuntimeValue>,
  ) -> Result<RuntimeValue, errors::ZephyrError> {
    let key_loc = expr.key.get_location();
    let value = match *expr.left {
      Expression::Identifier(ident) => self.evaluate_identifier(ident, true),
      _ => self.evaluate(*expr.left),
    }?;

    // Get key
    let key = if expr.is_computed {
      Some(self.evaluate((*expr.key).clone())?)
    } else {
      None
    };

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
            location: crate::MEMORY
              .lock()
              .unwrap()
              .set_value(arr_ref.location, RuntimeValue::Array(arr))?,
          }));
        }

        // Return
        return Ok(*(*&arr.items[number]).clone());
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
              key_loc.clone(),
            ))
          }
        };

        // Check if should assign
        if let Some(to_assign) = assign {
          // Check if already has it
          if object.items.contains_key(&string_key.clone()) {
            object.items.remove(&string_key.clone());
          }
          object.items.insert(string_key.clone(), to_assign.clone());

          // Update memory
          crate::MEMORY
            .lock()
            .unwrap()
            .set_value(obj_ref.location, RuntimeValue::Object(object))?;

          return Ok(RuntimeValue::ObjectContainer(obj_ref));
        }

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
      Expression::BreakStatement(stmt) => Err(ZephyrError {
        error_message: "Cannot break here".to_string(),
        error_type: ErrorType::Break,
        reference: None,
        location: stmt.location,
      }),
      Expression::ContinueStatement(stmt) => Err(ZephyrError {
        error_message: "Cannot continue here".to_string(),
        error_type: ErrorType::Continue,
        reference: None,
        location: stmt.location,
      }),
      Expression::ReturnStatement(stmt) => Err(ZephyrError {
        error_message: "Cannot return here".to_string(),
        error_type: ErrorType::Return(Box::from(if let Some(ret) = stmt.value {
          Box::from(self.evaluate(*ret)?)
        } else {
          Box::from(RuntimeValue::Null(Null {}))
        })),
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
        let mut path = std::path::PathBuf::new();
        path.push(self.scope.directory.borrow().clone());
        path.push(stmt.from.value.clone());

        crate::debug(
          &format!("Importing {}", path.display().to_string()),
          "import",
        );

        // Check if it exists
        if path.exists() == false {
          return Err(ZephyrError::runtime(
            format!(
              "Failed to import {} because the file does not exist",
              path.display().to_string()
            ),
            stmt.from.location.clone(),
          ));
        }

        // Check if it is file
        if path.is_file() == false {
          return Err(ZephyrError::runtime(
            format!(
              "Failed to import {} because the path is not a file",
              path.display().to_string()
            ),
            stmt.from.location.clone(),
          ));
        }

        let path_string = path.display().to_string();

        // Read it
        let file_contents = match std::fs::read_to_string(path_string.clone()) {
          Ok(ok) => ok,
          Err(err) => {
            return Err(ZephyrError::runtime(
              format!("Failed to read file: {}", err.to_string()),
              stmt.from.location.clone(),
            ));
          }
        };

        // Check if cache has it
        let scope = if self
          .import_cache
          .borrow()
          .contains_key(&(path_string.clone()))
        {
          crate::debug(&format!("Importing from cache {}", path_string), "import");
          self
            .import_cache
            .borrow()
            .get(&path_string)
            .unwrap()
            .clone()
        } else {
          crate::debug(&format!("Importing {}", path_string), "import");

          // Lex & Parse
          let result = lexer::lexer::lex(file_contents, path_string.clone())?;
          let mut parser = parser::parser::Parser::new(result);
          let ast = parser.produce_ast()?;

          // Create scope
          let path_pre_pop = path_string.clone();
          path.pop();
          let scope = &self.global_scope.create_child();
          *scope.directory.borrow_mut() = path.display().to_string();
          *scope.can_export.borrow_mut() = true;

          // Evaluate it
          let prev = std::mem::replace(&mut self.scope, scope.clone());
          self.evaluate(Expression::Program(ast))?;
          let _ = std::mem::replace(&mut self.scope, prev);

          // Set cache
          self
            .import_cache
            .borrow_mut()
            .insert(path_pre_pop, scope.clone());
          scope.clone()
        };

        for i in stmt.import {
          let to_import = i.0.symbol;
          let import_as = i.1.symbol;

          // Check if scope contains it
          if !scope.has_variable(&to_import) {
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
                RuntimeValue::StringValue(str) => {
                  if obj.items.contains_key(&str.value) {
                    true
                  } else {
                    false
                  }
                }
                _ => {
                  return Err(ZephyrError::runtime(
                    format!("Cannot check if object has {}", left.type_name()),
                    expr.left.get_location(),
                  ))
                }
              }
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
        let callee = self.evaluate(*expr.left)?;

        match callee {
          RuntimeValue::NativeFunction(func) => {
            let caller_args = expr2.arguments.clone();
            let args = caller_args
              .iter()
              .map(|e| self.evaluate(*e.clone()))
              .collect::<Result<Vec<_>, _>>()?;

            (func.func)(CallOptions {
              args: &args,
              location: expr2.location,
            })
          }
          RuntimeValue::Function(func) => {
            self.evaluate_zephyr_function(func, expr2.arguments, expr2.left.get_location())
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
        } else {
          if start > end {
            -1.0
          } else {
            1.0
          }
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

        Ok(to_array(array))
      }
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
              let mut array = match crate::MEMORY
                .lock()
                .unwrap()
                .get_value(container.location)?
              {
                RuntimeValue::Array(arr) => arr,
                _ => unreachable!(),
              };

              array.items.push(Box::from(right.clone()));

              // Modify the value
              crate::MEMORY
                .lock()
                .unwrap()
                .set_value(container.location, RuntimeValue::Array(array))?;
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
      Expression::UnaryExpression(expr) => {
        let expr_value = *expr.value;
        let value = self.evaluate(expr_value.clone())?;
        let operator = expr.operator;

        match operator {
          TokenType::UnaryOperator(UnaryOperator::Not) => Ok(RuntimeValue::Boolean(Boolean {
            value: !value.is_truthy(),
          })),
          TokenType::UnaryOperator(UnaryOperator::Dereference) => match value {
            RuntimeValue::Reference(refer) => crate::MEMORY.lock().unwrap().get_value(refer.value),
            RuntimeValue::Number(num) => crate::MEMORY
              .lock()
              .unwrap()
              .get_value(num.value as MemoryAddress),
            RuntimeValue::ArrayContainer(arr) => {
              crate::MEMORY.lock().unwrap().get_value(arr.location)
            }
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
          TokenType::UnaryOperator(UnaryOperator::LengthOf) => Ok(RuntimeValue::Number(Number {
            value: value.iterate()?.len() as f64,
          })),
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
        // Should while loops return everything they did like for loops?
        // give your answer!!!!!!!!!!
        let mut iters = 0;
        while self.evaluate(*expr.test.clone())?.is_truthy() {
          iters += 1;
          let res = self.evaluate_block(*expr.body.clone(), self.scope.create_child());

          // Check if continue or break
          match res {
            Err(err) => match err.error_type {
              ErrorType::Break => {
                break;
              }
              ErrorType::Continue => {
                continue;
              }
              _ => return Err(err),
            },
            Ok(_) => (),
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
            // Check if there is a catch block
            if let Some(catch) = expr.catch {
              // Check if it defines ident
              let scope = self.scope.create_child();
              if let Some(ident) = expr.catch_identifier {
                let err_obj = to_object(HashMap::from([
                  (
                    "message".to_string(),
                    RuntimeValue::StringValue(StringValue {
                      value: err.error_message,
                    }),
                  ),
                  (
                    "type".to_string(),
                    RuntimeValue::StringValue(StringValue {
                      value: format!("{:?}", err.error_type),
                    }),
                  ),
                  ("location".to_string(), err.location.to_object()),
                ]));
                scope.declare_variable(&ident.symbol, err_obj)?;
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

        for i in value {
          let scope = self.scope.create_child();
          scope.declare_variable(&expr.identifier.symbol, *i.clone())?;
          let res = match self.evaluate_block(expr.body.clone(), scope) {
            Ok(ok) => ok,
            Err(err) => match err.error_type {
              ErrorType::Break => break,
              ErrorType::Continue => continue,
              _ => return Err(err),
            },
          };
          values.push(Box::from(res));
        }

        // Check for else
        if values.len() == 0 {
          if let Some(el) = expr.none {
            return Ok(self.evaluate(Expression::Block(*el)))?;
          }
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
