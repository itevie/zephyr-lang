use crate::errors::{ErrorCode, ZephyrError};

use super::RuntimeValueDetails;

#[derive(Debug, Clone)]
pub struct RangeValue {
    pub options: RuntimeValueDetails,
    pub start: f64,
    pub end: f64,
    pub step: Option<f64>,
    pub inclusive_end: bool,
}

impl RangeValue {
    pub fn iter(&self) -> Result<Vec<f64>, ZephyrError> {
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
