//! Output rendering helpers: human-readable + `--json`, color via owo-colors.

use serde::Serialize;

pub fn todo(label: &str) {
    eprintln!("nvg: `{label}` is not yet implemented");
}

/// Print a serializable value as compact JSON to stdout.
pub fn print_json<T: Serialize>(value: &T) -> anyhow::Result<()> {
    let s = serde_json::to_string_pretty(value)?;
    println!("{s}");
    Ok(())
}
