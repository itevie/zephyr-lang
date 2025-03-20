use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::lexer::lex,
    parser::{
        nodes::{self, DeclareType, ExportType, ExposeType, Node},
        Parser,
    },
    runtime::{scope::Variable, values::Reference, Module},
};

use super::{
    scope::Scope,
    values::{self, RuntimeValueUtils},
    Interpreter, R,
};

impl Interpreter {
    pub fn run_export(&mut self, node: nodes::Export) -> R {
        match node.export {
            ExportType::Symbol(symbol) => {
                self.scope.insert_exported(symbol.value, node.export_as);
            }
            ExportType::Declaration(dec) => {
                self.run_declare(dec.clone())?;

                match dec.assignee {
                    DeclareType::Symbol(s) => {
                        self.scope.insert_exported(s.value.clone(), node.export_as)
                    }
                    _ => {
                        return Err(ZephyrError {
                            message: "Can only export declarations with symbol".to_string(),
                            code: ErrorCode::TypeError,
                            location: Some(dec.location.clone()),
                        })
                    }
                };
            }
            _ => panic!("Cannot handle this yet"),
        };

        Ok(values::Null::new().wrap())
    }

    pub fn run_import(&mut self, node: nodes::Import) -> R {
        // Resolve file location
        let path = &if PathBuf::from(node.import.clone()).is_absolute() {
            PathBuf::from(node.import.clone())
        } else {
            let _path = &PathBuf::from(self.scope.file_name())
                .parent()
                .unwrap()
                .join(node.import.clone());
            fs::canonicalize(_path).map_err(|err| ZephyrError {
                message: format!("Cannot resolve {}", _path.display().to_string()),
                code: ErrorCode::CannotResolve,
                location: Some(node.location.clone()),
            })?
        }
        .display()
        .to_string();

        let scope = if let Some(cache) = self.module_cache.get(path) {
            // The module is in module cache and is awaiting to be loaded
            (cache.lock().unwrap().scope.clone(), false)
        } else {
            let read = fs::read_to_string(path).map_err(|err| ZephyrError {
                message: format!("Cannot read {}: {}", path, err.kind()),
                code: ErrorCode::CannotResolve,
                location: Some(node.location.clone()),
            })?;

            let lexd = lex(&read, path.to_string())?;
            let ast = match Parser::new(lexd, path.to_string()).produce_ast()? {
                Node::Block(block) => Node::ExportedBlock(nodes::ExportedBlock {
                    nodes: block.nodes,
                    location: block.location,
                }),
                _ => unreachable!(),
            };
            let scope = Scope::new(Some(self.global_scope), self.global_scope.file_name());

            let module = Arc::from(Mutex::from(Module {
                scope: scope.clone(),
                wanted: vec![],
                exports: HashMap::new(),
            }));

            self.module_cache.insert(path.to_string(), module.clone());

            let old_scope = self.swap_scope(scope.clone());
            self.run(ast)?;
            self.swap_scope(old_scope);

            // Check all the places that have tried to import from this module
            let wanted = module.lock().unwrap().wanted.clone();
            for i in wanted {
                if !scope.get_exported_list().contains_key(&i.0) {
                    return Err(ZephyrError {
                        message: format!("Module {} does not export {}", path, i.0.clone()),
                        code: ErrorCode::NotExported,
                        location: Some(node.location.clone()),
                    });
                }
            }

            (scope, true)
        };

        // Define module export pointers
        for expose in node.exposing {
            let t = match expose {
                ExposeType::Identifier(i) => (Some(i.clone()), i.clone()),
                ExposeType::IdentifierAs(i, a) => (Some(i), a),
                ExposeType::StarAs(a) => (None, a),
                _ => panic!(),
            };

            // Check if module actually exports it
            if scope.1 {
                if let Some(ref name) = t.0 {
                    if !scope.0.get_exported_list().contains_key(name) {
                        return Err(ZephyrError {
                            message: format!("Module {} does not export {}", path, name.clone()),
                            code: ErrorCode::NotExported,
                            location: Some(node.location.clone()),
                        });
                    }
                }
            }

            if !scope.1 {
                self.module_cache
                    .get(path)
                    .unwrap()
                    .lock()
                    .unwrap()
                    .wanted
                    .push((t.1.clone(), node.location.clone()));
            }

            self.scope.insert(
                t.1,
                Variable::new(Reference::new_export(scope.0.clone(), t.0).wrap()),
                Some(node.location.clone()),
            )?;
        }

        return Ok(values::Null::new().wrap());
    }
}
