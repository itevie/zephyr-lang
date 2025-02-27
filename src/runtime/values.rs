use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::{Comparison, Location},
    parser::nodes,
};

use super::{
    memory_store,
    native::NativeExecutionContext,
    scope::{PrototypeStore, Scope},
    Job, MspcChannel, R,
};

#[derive(Debug, Clone)]
pub struct RuntimeValueDetails<'a> {
    pub tags: HashMap<String, String>,
    pub proto: Option<usize>,
    pub proto_value: Option<&'a RuntimeValue<'a>>,
}

impl RuntimeValueDetails<'_> {
    pub fn with_proto(id: usize) -> Self {
        Self {
            proto: Some(id),
            ..Default::default()
        }
    }
}

impl Default for RuntimeValueDetails<'_> {
    fn default() -> Self {
        Self {
            tags: Arc::from(Mutex::from(HashMap::default())),
            proto: None,
            proto_value: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum RuntimeValue<'a> {
    Number(Number<'a>),
    Null(Null<'a>),
    ZString(ZString<'a>),
    Boolean(Boolean<'a>),
    Reference(Reference<'a>),
    Function(Function<'a>),
    NativeFunction(NativeFunction<'a>),
    Array(Array<'a>),
    Object(Object<'a>),
    EventEmitter(EventEmitter<'a>),
}

impl RuntimeValue<'_> {
    /// Returns the predefined type of the value
    pub fn type_name(&self) -> &str {
        match self {
            RuntimeValue::Boolean(_) => "boolean",
            RuntimeValue::Null(_) => "null",
            RuntimeValue::Number(_) => "number",
            RuntimeValue::ZString(_) => "string",
            RuntimeValue::Reference(_) => "reference",
            RuntimeValue::Function(_) => "function",
            RuntimeValue::NativeFunction(_) => "native_function",
            RuntimeValue::Array(_) => "array",
            RuntimeValue::Object(_) => "object",
            RuntimeValue::EventEmitter(_) => "event_emitter",
        }
    }

    pub fn iter(&self) -> Result<Vec<RuntimeValue>, ZephyrError> {
        match self {
            RuntimeValue::ZString(str) => Ok(str
                .value
                .chars()
                .map(|v| ZString::new(v.to_string()))
                .collect::<Vec<RuntimeValue>>()),
            v => Err(ZephyrError {
                message: format!("Cannot iter a {}", v.type_name()),
                code: ErrorCode::CannotIterate,
                location: None,
            }),
        }
    }

    /// Gets the options struct no matter what the underlying type is
    pub fn options(&self) -> &RuntimeValueDetails {
        match self {
            RuntimeValue::Array(v) => &v.options,
            RuntimeValue::Boolean(v) => &v.options,
            RuntimeValue::Function(v) => &v.options,
            RuntimeValue::NativeFunction(v) => &v.options,
            RuntimeValue::Null(v) => &v.options,
            RuntimeValue::Number(v) => &v.options,
            RuntimeValue::Object(v) => &v.options,
            RuntimeValue::Reference(v) => &v.options,
            RuntimeValue::ZString(v) => &v.options,
            RuntimeValue::EventEmitter(v) => &v.options,
        }
    }

    pub fn set_options(&mut self, new: RuntimeValueDetails) -> () {
        match self {
            RuntimeValue::Array(v) => v.options = new,
            RuntimeValue::Boolean(v) => v.options = new,
            RuntimeValue::Function(v) => v.options = new,
            RuntimeValue::NativeFunction(v) => v.options = new,
            RuntimeValue::Null(v) => v.options = new,
            RuntimeValue::Number(v) => v.options = new,
            RuntimeValue::Object(v) => v.options = new,
            RuntimeValue::Reference(v) => v.options = new,
            RuntimeValue::ZString(v) => v.options = new,
            RuntimeValue::EventEmitter(v) => v.options = new,
        };
    }

    /// Converts the value into a string (not display)
    pub fn to_string(&self) -> Result<String, ZephyrError> {
        Ok(match self {
            RuntimeValue::Boolean(v) => v.value.to_string(),
            RuntimeValue::Null(_) => "null".to_string(),
            RuntimeValue::Number(v) => v.value.to_string(),
            RuntimeValue::Reference(v) => format!("{:#?}", v.inner()),
            RuntimeValue::ZString(v) => v.value.clone(),
            v => {
                format!("{:#?}", v)
                /*return Err(ZephyrError {
                    code: ErrorCode::CannotCoerce,
                    message: format!("Cannot coerce {} into a string", self.type_name()),
                    location: None,
                })*/
            }
        })
    }

    /// Checks whether or not the value is "truthy" following set rules
    pub fn is_truthy(&self) -> bool {
        match self {
            RuntimeValue::Boolean(v) => v.value,
            RuntimeValue::ZString(v) => v.value.len() > 0,
            RuntimeValue::Number(v) => v.value > 0f64,
            _ => false,
        }
    }

    /// Simply adds the value to the object store
    pub fn as_ref(&self) -> usize {
        memory_store::allocate(self.clone())
    }

    /// Used for returning a tuple containing the inner reference (or current value), along with the reference ID  
    /// Looks like: (value, ref)
    pub fn as_ref_tuple(&self) -> Result<(RuntimeValue, Option<Reference>), ZephyrError> {
        match self {
            RuntimeValue::Reference(r) => Ok(((*r.inner()?).clone(), Some(r.clone()))),
            _ => Ok((self.clone(), None)),
        }
    }

    pub fn compare_with(
        &self,
        right: RuntimeValue,
        t: Comparison,
        location: Option<Location>,
    ) -> Result<bool, ZephyrError> {
        if let Comparison::Eq = t {
            if self.type_name() != right.type_name() {
                return Ok(false);
            }
        }

        if let Comparison::Neq = t {
            if self.type_name() != right.type_name() {
                return Ok(true);
            }
        }

        return Ok(match (self, right, t) {
            (RuntimeValue::Number(l), RuntimeValue::Number(r), ref t) => match t {
                Comparison::Eq => l.value == r.value,
                Comparison::Neq => l.value != r.value,
                Comparison::Gt => l.value > r.value,
                Comparison::Lt => l.value < r.value,
                Comparison::GtEq => l.value >= r.value,
                Comparison::LtEq => l.value <= r.value,
            },
            (RuntimeValue::ZString(l), RuntimeValue::ZString(r), Comparison::Eq) => {
                l.value == r.value
            }
            (RuntimeValue::ZString(l), RuntimeValue::ZString(r), Comparison::Neq) => {
                l.value != r.value
            }
            (RuntimeValue::Null(_), RuntimeValue::Null(_), Comparison::Eq) => true,
            (_, ref r, ref t) => {
                return Err(ZephyrError {
                    code: ErrorCode::InvalidOperation,
                    message: format!(
                        "Cannot perform {} {} {}",
                        self.type_name(),
                        t,
                        r.type_name()
                    ),
                    location,
                })
            }
        });
    }
}

#[derive(Debug, Clone)]
pub struct Number<'a> {
    pub options: RuntimeValueDetails<'a>,
    pub value: f64,
}

impl Number<'_> {
    pub fn new<'a>(value: f64) -> RuntimeValue<'a> {
        RuntimeValue::Number(Number {
            value,
            options: RuntimeValueDetails::with_proto(PrototypeStore::get("object".to_string())),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ZString<'a> {
    pub options: RuntimeValueDetails<'a>,
    pub value: String,
}

impl ZString<'_> {
    pub fn new<'a>(value: String) -> RuntimeValue<'a> {
        RuntimeValue::ZString(ZString {
            value,
            options: RuntimeValueDetails::with_proto(PrototypeStore::get("string".to_string())),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Null<'a> {
    pub options: RuntimeValueDetails<'a>,
}

impl Null<'_> {
    pub fn new<'a>() -> RuntimeValue<'a> {
        RuntimeValue::Null(Null {
            options: RuntimeValueDetails::default(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Boolean<'a> {
    pub options: RuntimeValueDetails<'a>,
    pub value: bool,
}

impl Boolean<'_> {
    pub fn new<'a>(value: bool) -> RuntimeValue<'a> {
        RuntimeValue::Boolean(Boolean {
            value,
            options: RuntimeValueDetails::default(),
        })
    }
}

#[derive(Debug, Clone)]
pub enum FunctionType<'a> {
    Function(Function<'a>),
    NativeFunction(NativeFunction<'a>),
}

impl FunctionType<'_> {
    pub fn from<'a>(val: RuntimeValue) -> Result<FunctionType<'a>, ZephyrError> {
        match val {
            RuntimeValue::Function(f) => Ok(FunctionType::Function(f)),
            RuntimeValue::NativeFunction(f) => Ok(FunctionType::NativeFunction(f)),
            _ => Err(ZephyrError {
                message: format!("{} is not a function", val.type_name()),
                code: ErrorCode::TypeError,
                location: None,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Function<'a> {
    pub options: RuntimeValueDetails<'a>,
    pub body: nodes::Block,
    pub name: Option<String>,
    pub arguments: Vec<String>,
    pub scope: Scope<'a>,
}

#[derive(Clone)]
pub struct NativeFunction<'a> {
    pub options: RuntimeValueDetails<'a>,
    pub func: Arc<dyn Fn(NativeExecutionContext) -> R<'a> + Send + Sync>,
}

impl NativeFunction<'_> {
    pub fn new<'a>(
        f: Arc<dyn Fn(NativeExecutionContext) -> R<'a> + Send + Sync>,
    ) -> RuntimeValue<'a> {
        RuntimeValue::NativeFunction(NativeFunction {
            func: f,
            options: RuntimeValueDetails::default(),
        })
    }
}

impl std::fmt::Debug for NativeFunction<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NativeFunction")
    }
}

#[derive(Debug, Clone)]
pub struct Array<'a> {
    pub options: RuntimeValueDetails<'a>,
    pub items: Vec<RuntimeValue<'a>>,
}

impl Array<'_> {
    pub fn new(items: Vec<RuntimeValue>) -> RuntimeValue {
        RuntimeValue::Array(Array {
            items,
            options: RuntimeValueDetails::default(),
        })
    }

    pub fn new_ref(items: Vec<RuntimeValue>) -> RuntimeValue {
        Reference::new_from(Array::new(items))
    }
}

#[derive(Debug, Clone)]
pub struct Object<'a> {
    pub options: RuntimeValueDetails<'a>,
    pub items: HashMap<String, RuntimeValue<'a>>,
}

impl Object<'_> {
    pub fn new(items: HashMap<String, RuntimeValue>) -> RuntimeValue {
        RuntimeValue::Object(Object {
            items,
            options: RuntimeValueDetails::default(),
        })
    }

    pub fn new_ref(items: HashMap<String, RuntimeValue>) -> RuntimeValue {
        Reference::new_from(Object::new(items))
    }
}

#[derive(Debug, Clone)]
pub struct EventEmitter<'a> {
    pub options: RuntimeValueDetails<'a>,
    pub defined_events: Vec<String>,
    pub listeners: Arc<Mutex<HashMap<String, Arc<Mutex<Vec<FunctionType<'a>>>>>>>,
}

impl EventEmitter<'_> {
    pub fn new(events: Vec<String>) -> Self {
        EventEmitter {
            options: RuntimeValueDetails::with_proto(PrototypeStore::get(
                "event_emitter".to_string(),
            )),
            defined_events: events,
            listeners: Arc::from(Mutex::from(HashMap::new())),
        }
    }

    pub fn emit_from_thread(
        &self,
        message: String,
        args: Vec<RuntimeValue>,
        sender: &mut MspcChannel,
    ) -> () {
        if let Some(listeners) = self.listeners.lock().unwrap().get(&message) {
            let parts = listeners.lock().unwrap();
            for part in parts.iter() {
                sender.thread_message(Job {
                    func: part.clone(),
                    args: args.clone(),
                });
            }
        }
    }

    pub fn add_listener(
        &self,
        message: String,
        func: FunctionType,
        location: Location,
    ) -> Result<(), ZephyrError> {
        if !self.defined_events.contains(&message) {
            return Err(ZephyrError {
                message: format!("Event emitter does not have a {} event", message),
                code: ErrorCode::UndefinedEventMessage,
                location: Some(location),
            });
        }

        let mut lock = self.listeners.lock().unwrap();

        if !lock.contains_key(&message) {
            lock.insert(message, Arc::from(Mutex::from(vec![func])));
        } else {
            lock.get(&message).unwrap().lock().unwrap().push(func);
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum ReferenceType<'a> {
    Basic(usize),
    ModuleExport((Scope<'a>, Option<String>)),
}

#[derive(Debug, Clone)]
pub struct Reference<'a> {
    pub options: RuntimeValueDetails<'a>,
    pub location: ReferenceType<'a>,
}

impl Reference<'_> {
    pub fn new<'a>(location: usize) -> RuntimeValue<'a> {
        RuntimeValue::Reference(Reference {
            location: ReferenceType::Basic(location),
            options: RuntimeValueDetails::default(),
        })
    }
    pub fn new_export<'a>(scope: Arc<Mutex<Scope>>, ident: Option<String>) -> RuntimeValue<'a> {
        RuntimeValue::Reference(Reference {
            location: ReferenceType::ModuleExport((scope, ident)),
            options: RuntimeValueDetails::default(),
        })
    }

    pub fn new_from(value: RuntimeValue) -> RuntimeValue {
        RuntimeValue::Reference(Reference {
            location: ReferenceType::Basic(memory_store::allocate(value)),
            options: RuntimeValueDetails::default(),
        })
    }

    pub fn inner(&self) -> Result<Arc<RuntimeValue>, ZephyrError> {
        match self.location {
            ReferenceType::Basic(loc) => match memory_store::get_lock().get(loc) {
                Some(ok) => {
                    let res = ok.as_ref().and_then(|x| Some(x.clone()));

                    Ok(Arc::clone(&res.unwrap()))
                }
                None => Err(ZephyrError {
                    code: ErrorCode::UnknownReference,
                    message: format!("Cannot find refernce &{}", loc),
                    location: None,
                }),
            },
            ReferenceType::ModuleExport((ref scope, ref name)) => {
                if let Some(name) = name {
                    match scope.lock().unwrap().lookup(name.clone(), None) {
                        Ok(ok) => Ok(Arc::from(ok)),
                        Err(err) => Err(ZephyrError {
                            message: format!("Exported variable {} has not been resolved. Please move this expression to the init block, or fix the cyclic dependency.", name),
                            code: ErrorCode::Unresolved,
                            location: None,
                        })
                    }
                } else {
                    panic!()
                }
            }
        }
    }
}
