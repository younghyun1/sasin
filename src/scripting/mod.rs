//! Postman-style `pm.*` scripting (feature `scripting`, backed by QuickJS via rquickjs).
//!
//! Scripts run synchronously on the UI thread — QuickJS contexts are not `Send`, and scripts are
//! short — with a wall-clock interrupt budget. Pre-request scripts may set variables (fed back into
//! interpolation); test scripts produce pass/fail results. Data crosses the JS boundary as JSON.
//!
//! When the feature is disabled the entry points are no-ops that report that scripting is off, so
//! the rest of the app builds and runs without a JS toolchain.

/// One `pm.test(...)` result.
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

/// The result of running a script.
#[derive(Debug, Clone, Default)]
pub struct ScriptOutcome {
    /// Variables the script set via `pm.environment.set` / `pm.variables.set`.
    pub var_sets: Vec<(String, String)>,
    /// Captured `console.*` lines.
    pub console: Vec<String>,
    /// `pm.test(...)` results.
    pub tests: Vec<TestResult>,
    /// A script/engine error (uncaught exception, timeout, or "scripting disabled").
    pub error: Option<String>,
}

impl ScriptOutcome {
    /// True when there is nothing to report (no sets, console, tests, or error).
    pub fn is_empty(&self) -> bool {
        self.var_sets.is_empty()
            && self.console.is_empty()
            && self.tests.is_empty()
            && self.error.is_none()
    }
}

#[cfg(feature = "scripting")]
mod engine;

#[cfg(feature = "scripting")]
pub use engine::{run_pre_request, run_test};

#[cfg(not(feature = "scripting"))]
pub fn run_pre_request(
    _script: &str,
    _env: &std::collections::HashMap<String, String>,
) -> ScriptOutcome {
    disabled()
}

#[cfg(not(feature = "scripting"))]
pub fn run_test(
    _script: &str,
    _env: &std::collections::HashMap<String, String>,
    _response: &crate::models::ResponseModel,
) -> ScriptOutcome {
    disabled()
}

#[cfg(not(feature = "scripting"))]
fn disabled() -> ScriptOutcome {
    ScriptOutcome {
        error: Some("scripting is disabled — rebuild with `--features scripting`".to_string()),
        ..Default::default()
    }
}
