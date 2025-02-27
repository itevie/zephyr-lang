use crate::parser::nodes;

use super::{scope::Scope, values, Interpreter, R};

impl Interpreter {
    pub fn run_block(&mut self, expr: nodes::Block) -> R {
        let old_scope = self.swap_scope(Box::from(Scope::new_from_parent(self.scope.clone())));

        let mut last_executed = values::Null::new();

        for i in expr.nodes {
            last_executed = self.run(*i)?;
        }

        self.swap_scope(old_scope);
        Ok(last_executed)
    }

    pub fn run_exported(&mut self, expr: nodes::ExportedBlock) -> R {
        let mut last_executed = values::Null::new();

        for i in expr.nodes {
            last_executed = self.run(*i)?;
        }

        Ok(last_executed)
    }
}
