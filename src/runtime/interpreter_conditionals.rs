use crate::parser::nodes::{self, MatchCaseType};

use super::{
    values::{self, RuntimeValueUtils},
    Interpreter, R,
};

impl Interpreter {
    pub fn run_if(&mut self, expr: nodes::If) -> R {
        let result = self.run(*expr.test)?;

        if result.is_truthy() {
            self.run(*expr.succss)
        } else if let Some(alt) = expr.alternate {
            self.run(*alt)
        } else {
            Ok(values::Null::new().wrap())
        }
    }

    pub fn run_match(&mut self, expr: nodes::Match) -> R {
        let value = self.run(*expr.test)?;

        for test in expr.cases {
            match test {
                MatchCaseType::MatchCase(l) => {
                    if value.compare_with(self.run(*l.value)?, l.op, Some(expr.location.clone()))? {
                        return self.run(*l.success);
                    }
                }
                MatchCaseType::Else(r) => {
                    return self.run(*r);
                }
            }
        }

        Ok(values::Null::new().wrap())
    }
}
