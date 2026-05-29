//! The ordered run plan: which requests run, how many iterations, and the data row per iteration.

use crate::model::{Node, NodePath};

use super::data::DataRow;

/// Collect the paths of every HTTP request under `nodes`, depth-first pre-order (the same order
/// they appear in the tree). `prefix` is the path of the folder containing `nodes`. WebSocket
/// nodes are skipped — they are interactive sessions, not batch-runnable.
pub fn flatten_requests(nodes: &[Node], prefix: &NodePath) -> Vec<NodePath> {
    let mut out = Vec::new();
    collect(nodes, prefix, &mut out);
    out
}

fn collect(nodes: &[Node], prefix: &NodePath, out: &mut Vec<NodePath>) {
    for node in nodes {
        let mut path = prefix.clone();
        path.push(node.slug().to_string());
        match node {
            Node::Http(_) => out.push(path),
            Node::Folder(f) => collect(&f.children, &path, out),
            Node::Ws(_) => {}
        }
    }
}

/// One unit of work: send the request at `path` during iteration `iteration`, applying `data` (the
/// data-file row for that iteration, if any) as variable overrides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStep<'a> {
    pub path: &'a NodePath,
    pub iteration: usize,
    pub data: Option<&'a DataRow>,
}

/// An immutable description of a run: the requests, the iteration count, and the data rows. The
/// GUI keeps a cursor over `total_steps()` and calls [`RunPlan::step`] to drive each send.
#[derive(Debug, Clone)]
pub struct RunPlan {
    /// Path of the folder (or root, empty) this run covers — used for display.
    pub root: NodePath,
    /// HTTP request paths in execution order.
    pub requests: Vec<NodePath>,
    /// How many times to run the whole request list (>= 1).
    pub iterations: usize,
    /// Optional data rows; row `i` feeds iteration `i`.
    pub data: Vec<DataRow>,
}

impl RunPlan {
    /// Build a plan. `iterations` is clamped to at least 1; when a data file is present and the
    /// caller did not ask for a specific count, pass `data.len()` as `iterations`.
    pub fn new(
        root: NodePath,
        requests: Vec<NodePath>,
        iterations: usize,
        data: Vec<DataRow>,
    ) -> Self {
        Self {
            root,
            requests,
            iterations: iterations.max(1),
            data,
        }
    }

    /// Total number of sends this run performs.
    pub fn total_steps(&self) -> usize {
        self.requests.len().saturating_mul(self.iterations)
    }

    /// The work item at flat step index `n` (row-major: all requests of iteration 0, then 1, …).
    pub fn step(&self, n: usize) -> Option<RunStep<'_>> {
        if self.requests.is_empty() {
            return None;
        }
        let iteration = n / self.requests.len();
        if iteration >= self.iterations {
            return None;
        }
        let req_idx = n % self.requests.len();
        let path = self.requests.get(req_idx)?;
        Some(RunStep {
            path,
            iteration,
            data: self.data.get(iteration),
        })
    }
}
