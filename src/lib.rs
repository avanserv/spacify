//! Vertical rhythm for Python source: blank lines around logical blocks, inside the bounds that
//! `ruff format` keeps, so the two tools reach a fixed point in either order.

pub mod config;
mod edit;
pub mod python;

pub use config::Rules;
pub use python::{FormatError, format_source};
