use std::{
    cmp::Reverse,
    collections::HashMap,
    hash::Hash,
    sync::{
        mpsc::{channel, Receiver, Sender, TryRecvError},
        Arc, LazyLock, Mutex,
    },
    time::Instant,
};

use scope::{Scope, Variable};
use values::{FunctionType, Null, RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::{
        lexer::lex,
        tokens::{Location, NO_LOCATION},
    },
    parser::{
        nodes::{self, InterruptType, Node},
        Parser,
    },
};

pub mod interpreter_conditionals;
pub mod interpreter_errors;
pub mod interpreter_functions;
pub mod interpreter_helper;
pub mod interpreter_imports;
pub mod interpreter_loops;
pub mod interpreter_objects;
pub mod interpreter_operators;
pub mod interpreter_variables;
pub mod job_queue;
pub mod memory_store;
pub mod native;
pub mod scope;
pub mod values;

type R = Result<RuntimeValue, ZephyrError>;

macro_rules! include_lib {
    ($what:expr) => {
        (include_str!($what), $what)
    };
}

pub struct Module {
    pub exports: HashMap<String, Option<RuntimeValue>>,
    pub scope: Arc<Mutex<Scope>>,
    pub wanted: Vec<(String, Location)>,
}

#[derive(Debug, Clone)]
pub struct Job {
    pub func: FunctionType,
    pub args: Vec<RuntimeValue>,
}

#[derive(Debug, Clone)]
pub enum MspcSendType {
    ThreadCreate,
    ThreadDestroy,
    ThreadMessage(Job),
}

#[derive(Debug, Clone)]
pub struct MspcChannel {
    pub mspc: Sender<MspcSendType>,
}

impl MspcChannel {
    pub fn thread_start(&mut self) {
        self.mspc
            .send(MspcSendType::ThreadCreate)
            .unwrap_or_else(|e| panic!("Failed to send thread_start {:#?}", e.0))
    }

    pub fn thread_destroy(&mut self) {
        self.mspc
            .send(MspcSendType::ThreadDestroy)
            .unwrap_or_else(|e| panic!("Failed to send thread_destroy {:#?}", e.0))
    }

    pub fn thread_message(&mut self, job: Job) {
        self.mspc
            .send(MspcSendType::ThreadMessage(job))
            .unwrap_or_else(|e| panic!("Failed to send thread_message: {:?}", e))
    }
}

#[derive(Clone)]
pub struct Interpreter {
    pub scope: Arc<Mutex<Scope>>,
    pub global_scope: Arc<Mutex<Scope>>,
    pub module_cache: HashMap<String, Arc<Mutex<Module>>>,
    pub mspc: Option<MspcChannel>,
    pub thread_count: usize,
}

static NODE_TIMINGS: LazyLock<Arc<Mutex<HashMap<String, Vec<u128>>>>> =
    LazyLock::new(|| Arc::from(Mutex::from(HashMap::new())));

fn format_duration(nanos: u128) -> String {
    if nanos >= 1_000_000_000 {
        format!("{:.3} s", nanos as f64 / 1_000_000_000.0) // Convert to seconds
    } else if nanos >= 1_000_000 {
        format!("{:.3} ms", nanos as f64 / 1_000_000.0) // Convert to milliseconds
    } else if nanos >= 1_000 {
        format!("{:.3} µs", nanos as f64 / 1_000.0) // Convert to microseconds
    } else {
        format!("{} ns", nanos) // Keep as nanoseconds
    }
}

impl Interpreter {
    pub fn new(file_name: String) -> Self {
        let global_scope = Arc::from(Mutex::from(Scope::new(file_name)));
        global_scope
            .lock()
            .unwrap()
            .insert(
                "__zephyr_native".to_string(),
                Variable::from(
                    values::Object::new(native::all().iter().cloned().collect::<HashMap<_, _>>())
                        .as_ref_val(),
                ),
                None,
            )
            .unwrap();

        let mut interpreter = Interpreter {
            global_scope: global_scope.clone(),
            scope: global_scope.clone(),
            module_cache: HashMap::new(),
            thread_count: 0,
            mspc: None,
        };

        let library_files: Vec<(&str, &str)> = vec![
            include_lib!("./lib/any.zr"),
            include_lib!("./lib/events.zr"),
            include_lib!("./lib/basic.zr"),
            include_lib!("./lib/strings.zr"),
            include_lib!("./lib/arrays.zr"),
            include_lib!("./lib/fs.zr"),
            include_lib!("./lib/module.zr"),
            include_lib!("./lib/result.zr"),
        ];

        for lib in library_files {
            let lib_scope = Arc::new(Mutex::new(Scope::new_from_parent(global_scope.clone())));

            let parsed = Parser::new(
                lex(lib.0, lib.1.to_string())
                    .unwrap_or_else(|e| panic!("{}", e._visualise(lib.0.to_string()))),
                lib.1.to_string(),
            )
            .produce_ast()
            .unwrap_or_else(|e| panic!("{}", e._visualise(lib.0.to_string())));

            std::mem::swap(&mut interpreter.scope, &mut lib_scope.clone());
            interpreter
                .run_exported(match parsed {
                    Node::Block(b) => nodes::ExportedBlock {
                        nodes: b.nodes,
                        location: b.location,
                    },
                    _ => panic!(),
                })
                .unwrap_or_else(|e| panic!("{}", e._visualise(lib.0.to_string())));
            std::mem::swap(&mut interpreter.scope, &mut lib_scope.clone());

            let finished_scope = lib_scope.lock().unwrap();
            for (name, _) in &finished_scope.exported {
                let value = finished_scope.lookup(name.clone(), None).unwrap();
                global_scope
                    .lock()
                    .unwrap()
                    .insert(name.clone(), Variable::from(value), None)
                    .unwrap();
            }
        }

        interpreter
    }

    pub fn base_run(&mut self, node: Node) -> R {
        let (tx, rx): (Sender<MspcSendType>, Receiver<MspcSendType>) = channel();
        self.mspc = Some(MspcChannel { mspc: tx });

        let result = self.run(node);

        /*if self.thread_count == 0 {
            loop {
                match rx.try_recv() {
                    Ok(value) => match value {
                        MspcSendType::ThreadMessage(job) => {
                            self.run_function(job.func, job.args, NO_LOCATION.clone())?;
                        }
                        _ => (),
                    },
                    Err(_) => break,
                }
            }
            return result;
        }*/

        loop {
            match rx.try_recv() {
                Ok(value) => match value {
                    MspcSendType::ThreadCreate => self.thread_count += 1,
                    MspcSendType::ThreadDestroy => self.thread_count -= 1,
                    MspcSendType::ThreadMessage(job) => {
                        self.run_function(job.func, job.args, NO_LOCATION.clone())?;
                    }
                },
                Err(TryRecvError::Empty) => {
                    if self.thread_count == 0 {
                        break;
                    }
                    std::thread::yield_now();
                }
                Err(TryRecvError::Disconnected) => {
                    break;
                }
            }
        }

        let data = NODE_TIMINGS.lock().unwrap();
        let mut sorted_vec: Vec<(String, u128)> = data
            .clone()
            .into_iter()
            .map(|(name, values)| {
                let avg = values.iter().sum::<u128>() / values.len() as u128;
                (name, avg)
            })
            .collect();

        sorted_vec.sort_by_key(|&(_, avg)| Reverse(avg));

        for (key, time) in sorted_vec {
            println!(
                "{}: {} ({})",
                key,
                format_duration(time),
                data.get(&key).unwrap().len()
            );
        }

        println!("{:?}", data.keys());

        return result;
    }

    pub fn swap_scope(&mut self, scope: Arc<Mutex<Scope>>) -> Arc<Mutex<Scope>> {
        std::mem::replace(&mut self.scope, scope)
    }

    pub fn run(&mut self, node: Node) -> R {
        let start = Instant::now();
        let result = match node.clone() {
            // ----- conditionals -----
            Node::If(expr) => self.run_if(expr),
            Node::Match(expr) => self.run_match(expr),

            // ----- functions -----
            Node::Function(expr) => self.run_make_function(expr),
            Node::Call(expr) => self.run_call(expr),

            // ----- helpers -----
            Node::Block(expr) => self.run_block(expr),
            Node::ExportedBlock(expr) => self.run_exported(expr),

            // ----- loops -----
            Node::WhileLoop(expr) => self.run_while(expr),
            Node::For(expr) => self.run_for(expr),

            // ----- errors -----
            Node::PropogateError(expr) => self.run_propogate_error(expr),
            Node::EncapsulateError(expr) => self.run_encapsulate_error(expr),

            // ----- operators -----
            Node::Arithmetic(expr) => self.run_arithmetic(expr),
            Node::Is(expr) => self.run_is(expr),
            Node::Comp(expr) => self.run_comp(expr),
            Node::Unary(expr) => self.run_unary(expr),
            Node::Range(expr) => {
                let start = match self.run(*expr.start.clone())? {
                    RuntimeValue::Number(n) => n.value,
                    _ => {
                        return Err(ZephyrError {
                            message: "Expected number for start of range".to_string(),
                            code: ErrorCode::TypeError,
                            location: Some(expr.start.location().clone()),
                        })
                    }
                };
                let end = match self.run(*expr.end.clone())? {
                    RuntimeValue::Number(n) => n.value,
                    _ => {
                        return Err(ZephyrError {
                            message: "Expected number for end of range".to_string(),
                            code: ErrorCode::TypeError,
                            location: Some(expr.end.location().clone()),
                        })
                    }
                };
                let step = match expr.step {
                    Some(v) => match self.run(*v.clone())? {
                        RuntimeValue::Number(n) => Some(n.value),
                        _ => {
                            return Err(ZephyrError {
                                message: "Expected number for step of range".to_string(),
                                code: ErrorCode::TypeError,
                                location: Some(v.location().clone()),
                            })
                        }
                    },
                    None => None,
                };

                Ok(RuntimeValue::RangeValue(values::RangeValue {
                    options: RuntimeValueDetails::default(),
                    start,
                    end,
                    step,
                    inclusive_end: expr.inclusive_end,
                }))
            }

            // ----- variables -----
            Node::Declare(expr) => self.run_declare(expr),
            Node::Assign(expr) => self.run_assign(expr),
            Node::Enum(expr) => self.run_enum(expr),

            // ----- imports -----
            Node::Import(expr) => self.run_import(expr),
            Node::Export(expr) => self.run_export(expr),

            // ----- others -----
            /*Node::WhenClause(expr) => {
                let emitter = match self.run(*expr.emitter.clone())? {
                    RuntimeValue::EventEmitter(e) => e,
                    e => {
                        return Err(ZephyrError {
                            message: format!("Cannot listen to a {} for events", e.type_name()),
                            code: ErrorCode::TypeError,
                            location: Some(expr.emitter.location().clone()),
                        })
                    }
                };

                let message = match self.run(*expr.message.clone())? {
                    RuntimeValue::ZString(s) => s,
                    e => {
                        return Err(ZephyrError {
                            message: format!(
                                "Expected string for message, but got {}",
                                e.type_name()
                            ),
                            code: ErrorCode::TypeError,
                            location: Some(expr.emitter.location().clone()),
                        })
                    }
                };

                let func = match self.run(*expr.func.clone())? {
                    RuntimeValue::Function(f) => FunctionType::Function(f),
                    RuntimeValue::NativeFunction(f) => FunctionType::NativeFunction(f),
                    e => {
                        return Err(ZephyrError {
                            message: format!(
                                "Expected function for listener, but got {}",
                                e.type_name()
                            ),
                            code: ErrorCode::TypeError,
                            location: Some(expr.emitter.location().clone()),
                        })
                    }
                };

                emitter.add_listener(message.value, func, expr.location)?;

                Ok(values::Null::new())
            }*/
            Node::Interrupt(expr) => match expr.t {
                InterruptType::Continue => Err(ZephyrError {
                    message: "Cannot continue here".to_string(),
                    code: ErrorCode::Continue,
                    location: Some(expr.location.clone()),
                }),
                InterruptType::Break => Err(ZephyrError {
                    message: "Cannot break here".to_string(),
                    code: ErrorCode::Break,
                    location: Some(expr.location.clone()),
                }),
                InterruptType::Return(val) => {
                    let value = if let Some(v) = val {
                        Some(self.run(*v)?)
                    } else {
                        None
                    };

                    Err(ZephyrError {
                        message: "Cannot return here".to_string(),
                        code: ErrorCode::Return(value),
                        location: Some(expr.location.clone()),
                    })
                }
            },

            Node::Array(expr) => {
                let mut items: Vec<RuntimeValue> = vec![];
                for i in expr.items {
                    items.push(self.run(*i)?);
                }
                Ok(values::Array::new(items).as_ref_val())
            }
            Node::Object(expr) => {
                let mut items: HashMap<String, RuntimeValue> = HashMap::new();

                for (k, v) in expr.items {
                    items.insert(k, self.run(*v.value)?);
                }

                Ok(values::Object::new(items).as_ref_val())
            }

            Node::Member(expr) => self.run_member(expr, None),

            Node::Number(expr) => Ok(values::Number::new(expr.value).wrap()),
            Node::ZString(expr) => Ok(values::ZString::new(expr.value).wrap()),
            Node::Symbol(expr) => Ok(
                match self
                    .scope
                    .lock()
                    .unwrap()
                    .lookup(expr.value, Some(expr.location))?
                    .clone()
                {
                    RuntimeValue::Reference(r) => match r.location {
                        values::ReferenceType::Basic(_) => RuntimeValue::Reference(r.clone()),
                        values::ReferenceType::ModuleExport(_) => (*r.inner()?).clone(),
                    },
                    x => x,
                },
            ),

            Node::DebugNode(expr) => {
                let result = self.run(*expr.node)?;
                println!("{}", result.to_string(true, true, true).unwrap());
                return Ok(Null::new().wrap());
            }
        }
        .map_err(|ref x| {
            let mut err = x.clone();
            if let None = x.location {
                err.location = Some(node.location().clone())
            }
            err
        });

        let done = start.elapsed().as_nanos();
        let key = format!("Node:{:?}", node).split("(").collect::<Vec<&str>>()[0].to_string();
        insert_node_timing(key, done);

        result
    }

    pub fn member(&mut self, expr: nodes::Member) -> R {
        let left = self.run(*expr.left.clone())?.as_ref_tuple()?;

        if !expr.computed {
            let key = match *expr.right {
                Node::Symbol(sym) => sym.value,
                _ => unreachable!(),
            };

            todo!();
        } else {
            let right = self.run(*expr.right.clone())?.as_ref_tuple()?;

            match left {
                // object[_]
                (RuntimeValue::Object(obj), Some(_)) => match right {
                    // object[string]
                    (RuntimeValue::ZString(string), None) => {
                        if !obj.items.contains_key(&string.value) {
                            return Err(ZephyrError {
                                code: ErrorCode::InvalidKey,
                                message: format!("Object does not contain key {}", string.value),
                                location: Some(expr.right.location().clone()),
                            });
                        }

                        Ok(obj.items.get(&string.value).unwrap().clone())
                    }
                    _ => {
                        return Err(ZephyrError {
                            code: ErrorCode::InvalidOperation,
                            message: format!(
                                "Cannot access an object with a {}",
                                right.0.type_name()
                            ),
                            location: Some(expr.right.location().clone()),
                        })
                    }
                },
                // array[_]
                (RuntimeValue::Array(arr), Some(_)) => match right {
                    // array[number]
                    (RuntimeValue::Number(num), None) => {
                        // Check out of bounds
                        if num.value as usize >= arr.items.len() {
                            return Err(ZephyrError {
                                code: ErrorCode::OutOfBounds,
                                message: format!(
                                    "Array length is {}, but index wanted was {}",
                                    arr.items.len(),
                                    num.value
                                ),
                                location: Some(expr.location),
                            });
                        }

                        Ok(arr.items[num.value as usize].clone())
                    }
                    // array[array]
                    (RuntimeValue::Array(key_arr), Some(_)) => {
                        let mut items: Vec<RuntimeValue> = vec![];

                        for (index, i) in key_arr.items.iter().enumerate() {
                            match i {
                                RuntimeValue::Number(num) => items.push({
                                    // Check out of bounds
                                    if num.value as usize >= arr.items.len() {
                                        return Err(ZephyrError {
                                            code: ErrorCode::OutOfBounds,
                                            message: format!(
                                                "Array length is {}, but index wanted was {} at index {}",
                                                arr.items.len(),
                                                num.value,
                                                index
                                            ),
                                            location: Some(expr.location),
                                        });
                                    }

                                    arr.items[num.value as usize].clone()
                                }),
                                _ => return Err(ZephyrError {
                                    code: ErrorCode::InvalidOperation,
                                    message: format!(
                                        "All elements in array key must be a number, but got {} at index {}", 
                                        i.type_name(),
                                        index
                                    ),
                                    location: None,
                                })
                            }
                        }

                        Ok(values::Array::new(items).as_ref_val())
                    }
                    _ => {
                        return Err(ZephyrError {
                            code: ErrorCode::InvalidOperation,
                            message: format!(
                                "Cannot access an array with a {}",
                                right.0.type_name()
                            ),
                            location: Some(expr.right.location().clone()),
                        })
                    }
                },
                _ => {
                    return Err(ZephyrError {
                        code: ErrorCode::InvalidOperation,
                        message: format!("Cannot access a {}", left.0.type_name()),
                        location: Some(expr.left.location().clone()),
                    })
                }
            }
        }
    }
}

pub fn insert_node_timing(key: String, time: u128) {
    let mut lock = NODE_TIMINGS.lock().unwrap();
    if let Some(val) = lock.get_mut(&key) {
        val.push(time);
    } else {
        lock.insert(key, vec![time]);
    }
}

macro_rules! time_this {
    ($name:expr, $what:expr) => {{
        let time = Instant::now();

        let result = $what;

        insert_node_timing($name, time.elapsed().as_nanos());

        result
    }};
}

pub(crate) use time_this;
