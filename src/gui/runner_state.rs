//! GUI state for the collection runner: the editable config, the built plan, a cursor over its
//! steps, the accumulated report, and the request currently in flight.

use std::collections::HashMap;

use crate::model::NodePath;
use crate::runner::{DataRow, RunPlan, RunReport};

/// The request currently being sent by the runner, kept so its test script can run (on the UI
/// thread) once the response arrives and the outcome can be attributed correctly.
#[derive(Debug, Clone)]
pub struct CurrentRun {
    pub path: NodePath,
    pub name: String,
    pub iteration: usize,
    /// Error from the pre-request script, if any (folded into the outcome).
    pub pre_error: Option<String>,
    /// Variable snapshot after the pre-request script, used to run the test script.
    pub snapshot: HashMap<String, String>,
}

/// Live runner session: configuration plus progress over a built [`RunPlan`].
#[derive(Debug)]
pub struct RunnerState {
    /// Folder path being run (empty = whole workspace).
    pub root: NodePath,
    pub root_name: String,
    /// Flattened HTTP request paths, fixed when the runner opens.
    pub requests: Vec<NodePath>,
    pub iterations_text: String,
    pub data_path: String,
    pub data: Vec<DataRow>,
    /// The plan built at the last [`start`](Self::start); drives stepping.
    pub plan: RunPlan,
    /// Index of the next step to send.
    pub cursor: usize,
    pub report: RunReport,
    pub running: bool,
    pub finished: bool,
    /// Correlation id of the in-flight send (matched against `RunnerFinished`).
    pub gen_id: u64,
    pub current: Option<CurrentRun>,
}

impl RunnerState {
    /// Open a runner for a folder's flattened request list (not yet started).
    pub fn new(root: NodePath, root_name: String, requests: Vec<NodePath>) -> Self {
        let plan = RunPlan::new(root.clone(), requests.clone(), 1, Vec::new());
        Self {
            root,
            root_name,
            requests,
            iterations_text: "1".to_string(),
            data_path: String::new(),
            data: Vec::new(),
            plan,
            cursor: 0,
            report: RunReport::default(),
            running: false,
            finished: false,
            gen_id: 0,
            current: None,
        }
    }

    /// Total sends the current plan performs (after start; before start reflects one iteration).
    pub fn total(&self) -> usize {
        self.plan.total_steps()
    }
}
