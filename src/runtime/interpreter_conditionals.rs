use either::Either;

use crate::parser::nodes;

use super::{values, Interpreter, R};

impl Interpreter {
    pub fn run_if(&mut self, expr: nodes::If) -> R {
        let result = self.run(*expr.test)?;

        if result.is_truthy() {
            self.run(*expr.succss)
        } else if let Some(alt) = expr.alternate {
            self.run(*alt)
        } else {
            Ok(values::Null::new())
        }
    }

    pub fn run_match(&mut self, expr: nodes::Match) -> R {
        let value = self.run(*expr.test)?;

        for test in expr.cases {
            match test {
                Either::Left(l) => {
                    if value.compare_with(self.run(*l.value)?, l.op, Some(expr.location.clone()))? {
                        return self.run(*l.success);
                    }
                }
                Either::Right(r) => {
                    return self.run(*r);
                }
            }
        }

        Ok(values::Null::new())
    }
}
