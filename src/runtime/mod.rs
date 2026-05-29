//! Runtime: resolve a stored request for execution (variable interpolation now; scripting in P6).

pub mod vars;

pub use vars::{VarContext, interpolate, resolve_request};
