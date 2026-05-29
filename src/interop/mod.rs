//! Interoperability: import requests from and export them to `curl` commands.

pub mod curl_export;
pub mod curl_import;

#[cfg(test)]
mod tests;

pub use curl_export::to_curl;
pub use curl_import::from_curl;
