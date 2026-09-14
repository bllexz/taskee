//! Derive macros for [`taskee`](https://docs.rs/taskee)

use proc_macro::TokenStream;
use syn::{DeriveInput, Error, parse_macro_input};

mod payload;
mod status;
mod util;

/// Generates a typed task wrapper for a payload struct.
///
/// `struct Download` produces `struct DownloadTask`, which derefs to
/// `taskee::Task<Download>`. Options go in `#[taskee(..)]`:
///
/// - `progress` reads the `current`, `total` and `progress` fields.
/// - `status` reads the `status` field and brings in that enum's setters.
#[proc_macro_derive(Task, attributes(taskee))]
pub fn derive_task(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  payload::expand(input).unwrap_or_else(Error::into_compile_error).into()
}

/// Generates the task's state machine from a status enum.
///
/// Mark the variants that end the task with `#[terminal]`. A non-terminal
/// variant becomes `set_<variant>()` on the task. A terminal one becomes
/// `<variant>()` and takes the task by value. Variants with fields take the
/// same fields.
///
/// The methods arrive through a generated trait, `<Enum>Ext`, which callers in
/// another module have to import.
#[proc_macro_derive(TaskStatus, attributes(terminal))]
pub fn derive_task_status(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  status::expand(input).unwrap_or_else(Error::into_compile_error).into()
}
