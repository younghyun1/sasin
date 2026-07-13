//! Postman Collection v2.1 interop (import only).

mod import;
mod schema;

pub use import::{PostmanImport, from_postman};
