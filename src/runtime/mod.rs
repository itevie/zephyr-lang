use std::{
    cell::RefCell,
    cmp::Reverse,
    collections::HashMap,
    rc::Rc,
    sync::{
        mpsc::{channel, Receiver, Sender, TryRecvError},
        Arc, LazyLock, Mutex,
    },
    time::Instant,
};
use uuid::Uuid;
use scope::{Scope, ScopeInnerType, Variable};
use values::{Null, RuntimeValue, RuntimeValueUtils};

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
pub mod interpreter_literals;
pub mod interpreter_loops;
pub mod interpreter_objects;
pub mod interpreter_operators;
pub mod interpreter_variables;
pub mod native;
pub mod prototype_store;
pub mod scope;
pub mod values;
pub mod zephyr_mspc;

type R = Result<RuntimeValue, ZephyrError>;

macro_rules! include_lib {
    ($what:expr) => {
        (include_str!($what), $what)
    };
}

pub struct Module {
    pub exports: HashMap<String, Option<RuntimeValue>>,
    pub scope: ScopeInnerType,
    pub wanted: Vec<(String, Location)>,
}

#[derive(Clone)]
pub struct Interpreter {
    pub scope: ScopeInnerType,
    pub global_scope: ScopeInnerType,
    pub module_cache: HashMap<String, Rc<RefCell<Module>>>,
    pub mspc: Option<zephyr_mspc::MspcChannel>,
    pub thread_count: usize,
    pub prototype_store: prototype_store::PrototypeStore,
    pub function_ids: Rc<RefCell<HashMap<Uuid, FunctionType>>>,
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
        let global_scope = Rc::from(RefCell::from(Scope::new(file_name)));
        global_scope
            .borrow_mut()
            .insert(
                "__zephyr_native".to_string(),
                Variable::from(
                    values::Object::new(native::all().iter().cloned().collect::<HashMap<_, _>>())
                        .wrap(),
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
            prototype_store: prototype_store::PrototypeStore::new(),
            function_ids: Rc::default()
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
            include_lib!("./lib/math.zr"),
            include_lib!("./lib/numbers.zr"),
            include_lib!("./lib/enums.zr"),
        ];

        for lib in library_files {
            let lib_scope = Rc::new(RefCell::new(Scope::new_from_parent(global_scope.clone())));

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

            let finished_scope = lib_scope.borrow();
            for (name, _) in &finished_scope.exported {
                let value = finished_scope.lookup(name.clone(), None).unwrap();
                global_scope
                    .borrow_mut()
                    .insert(name.clone(), Variable::from(value), None)
                    .unwrap();
            }
        }

        interpreter
    }

    pub fn base_run(&mut self, node: Node) -> R {
        let (tx, rx): (
            Sender<zephyr_mspc::MspcSendType>,
            Receiver<zephyr_mspc::MspcSendType>,
        ) = channel();
        self.mspc = Some(zephyr_mspc::MspcChannel { mspc: tx });

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
                    zephyr_mspc::MspcSendType::ThreadCreate => self.thread_count += 1,
                    zephyr_mspc::MspcSendType::ThreadDestroy => self.thread_count -= 1,
                    zephyr_mspc::MspcSendType::ThreadMessage(job) => {
                        let Some(func) = self.function_ids.borrow().get(&job.func).cloned() else {
                            panic!("Function id was not found in the hash map {}", job.func)
                        };

                        self.run_function(func.clone(), job.args.into(), NO_LOCATION.clone())?;
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

        // for (key, time) in sorted_vec {
        //     println!(
        //         "{}: {} ({})",
        //         key,
        //         format_duration(time),
        //         data.get(&key).unwrap().len()
        //     );
        // }

        // println!("{:?}", data.keys());

        result
    }

    pub fn swap_scope(&mut self, scope: ScopeInnerType) -> ScopeInnerType {
        std::mem::replace(&mut self.scope, scope)
    }

    pub fn insert_function(&self, f: FunctionType) -> Uuid {
        let uuid = Uuid::new_v4();
        self.function_ids.borrow_mut().insert(uuid, f);
        uuid
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

            // ----- operators -----
            Node::Arithmetic(expr) => self.run_arithmetic(expr),
            Node::Is(expr) => self.run_is(expr),
            Node::Comp(expr) => self.run_comp(expr),
            Node::Unary(expr) => self.run_unary(expr),
            Node::Range(expr) => self.run_range(expr),
            Node::Logical(expr) => self.run_logical(expr),

            // ----- variables -----
            Node::Declare(expr) => self.run_declare(expr),
            Node::Assign(expr) => self.run_assign(expr),
            Node::Enum(expr) => self.run_enum(expr),

            // ----- imports -----
            Node::Import(expr) => self.run_import(expr),
            Node::Export(expr) => self.run_export(expr),

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

            Node::Array(expr) => self.run_array(expr),
            Node::Object(expr) => self.run_object(expr),

            Node::Member(expr) => self.run_member(expr, None),

            Node::Number(expr) => Ok(values::Number::new(expr.value).wrap()),
            Node::ZString(expr) => Ok(values::ZString::new(expr.value).wrap()),
            Node::Symbol(expr) => {
                Ok(
                    match self
                        .scope
                        .borrow()
                        .lookup(expr.value, Some(expr.location))?
                    {
                        RuntimeValue::Export(r) => r.inner()?,
                        v => v,
                    },
                )

                /*Ok(match value {
                    RuntimeValue::Reference(r) => match r.location {
                        values::ReferenceType::Basic(_) => value,
                        values::ReferenceType::ModuleExport(_) => (*r.inner()?).clone(),
                    },
                    _ => value,
                })*/
            }

            Node::Debug(expr) => {
                let result = self.run(*expr.node)?;
                println!("{}", result.to_string(true, true, true).unwrap());
                return Ok(Null::new().wrap());
            }
        }
        .map_err(|ref x| {
            // If there is no location provided, just set it to the current node
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
use crate::runtime::values::FunctionType;
