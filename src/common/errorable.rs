use std::fmt::Debug;

use anyhow::Result;

pub trait Errorable<T = ()> {
    fn into_result(self) -> Result<T>;
    fn format_multiple_errors<E: Debug>(&self, errors: &Vec<E>) -> String {
        errors
            .iter()
            .map(|e| format!("- {:?}", e))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
