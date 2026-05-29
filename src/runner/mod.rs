//! Collection runner: flatten a folder into an ordered request list, parse an optional data file
//! into per-iteration variable rows, and aggregate the run results.
//!
//! Sending and script execution are *not* done here: pm.* scripts run on the GUI thread (the
//! QuickJS context is `!Send`), so the GUI steps through a [`RunPlan`] one request at a time,
//! reusing the normal send pipeline, and records each [`RequestOutcome`] back into a [`RunReport`].

pub mod data;
pub mod plan;
pub mod report;

#[cfg(test)]
mod tests;

pub use data::{DataRow, parse_data_file};
pub use plan::{RunPlan, RunStep, flatten_requests};
pub use report::{RequestOutcome, RunReport};
