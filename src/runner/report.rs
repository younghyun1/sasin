//! Per-request outcomes and the aggregate run report.

use crate::model::NodePath;
use crate::scripting::TestResult;

/// The result of sending one request once (one iteration).
#[derive(Debug, Clone)]
pub struct RequestOutcome {
    pub path: NodePath,
    pub name: String,
    pub iteration: usize,
    /// HTTP status code, when the send completed.
    pub status: Option<u16>,
    /// Transport/script error, when the send failed.
    pub error: Option<String>,
    pub tests: Vec<TestResult>,
    pub duration_ms: u128,
}

impl RequestOutcome {
    /// A request passes when it sent without error and every assertion passed.
    pub fn passed(&self) -> bool {
        self.error.is_none() && self.tests.iter().all(|t| t.passed)
    }
}

/// Accumulated outcomes for a whole run, with rollup helpers for the summary view.
#[derive(Debug, Default, Clone)]
pub struct RunReport {
    pub outcomes: Vec<RequestOutcome>,
}

impl RunReport {
    pub fn push(&mut self, outcome: RequestOutcome) {
        self.outcomes.push(outcome);
    }

    pub fn requests(&self) -> usize {
        self.outcomes.len()
    }

    pub fn passed_requests(&self) -> usize {
        self.outcomes.iter().filter(|o| o.passed()).count()
    }

    pub fn failed_requests(&self) -> usize {
        self.requests() - self.passed_requests()
    }

    pub fn total_assertions(&self) -> usize {
        self.outcomes.iter().map(|o| o.tests.len()).sum()
    }

    pub fn passed_assertions(&self) -> usize {
        self.outcomes
            .iter()
            .flat_map(|o| &o.tests)
            .filter(|t| t.passed)
            .count()
    }

    /// True once every assertion across every request passed (and nothing errored).
    pub fn all_passed(&self) -> bool {
        self.outcomes.iter().all(RequestOutcome::passed)
    }
}
