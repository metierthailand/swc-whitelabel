use std::fmt::{Debug, Write};

use anyhow::Result;

pub trait Errorable<T = ()> {
    fn into_result(self) -> Result<T>;
    fn format_multiple_errors<E: Debug>(&self, errors: &[E]) -> String {
        let mut result = String::new();
        for (i, e) in errors.iter().enumerate() {
            if i > 0 {
                result.push('\n');
            }
            let _ = write!(result, "- {:?}", e);
        }
        result
    }
}
