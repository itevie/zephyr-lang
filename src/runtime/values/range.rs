use crate::{
    errors::{ErrorCode, ZephyrError},
    util::colors,
};

use super::{Number, RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

#[derive(Debug, Clone)]
pub struct RangeValue {
    pub options: RuntimeValueDetails,
    pub start: f64,
    pub end: f64,
    pub step: Option<f64>,
    pub inclusive_end: bool,
}

impl RangeValue {
    pub fn iter_f64(&self) -> Result<Vec<f64>, ZephyrError> {
        let step = self
            .step
            .unwrap_or(if self.end < self.start { -1.0 } else { 1.0 });

        let end = if self.inclusive_end {
            self.end
        } else {
            self.end - step
        };

        if (self.start > self.end && step > 0.0) || (self.start < self.end && step < 0.0) {
            return Err(ZephyrError {
                message: "This range would result in an infinite loop".to_string(),
                code: ErrorCode::RangeError,
                location: None,
            });
        }

        let values: Vec<f64> = (0..)
            .map(|i| self.start + i as f64 * step)
            .take_while(|&x| (step > 0.0 && x <= end) || (step < 0.0 && x >= end))
            .collect();

        Ok(values.iter().map(|z| *z).collect())
    }
}

impl RuntimeValueUtils for RangeValue {
    fn type_name(&self) -> &str {
        "range"
    }

    fn iter(&self) -> Result<Vec<RuntimeValue>, ZephyrError> {
        Ok(self.iter_f64()?.iter().map(|x| Number::new(*x)).collect())
    }

    fn to_string(&self, is_display: bool, color: bool) -> Result<String, ZephyrError> {
        let string = format!(
            "({}{}{}{})",
            self.start,
            if self.inclusive_end { "..=" } else { ".." },
            self.end,
            if let Some(step) = self.step {
                format!(":{}", step)
            } else {
                "".to_string()
            }
        );

        Ok(match color {
            true => format!("{}{}{}", colors::FG_YELLOW, string, colors::COLOR_RESET),
            false => string,
        })
    }
}
