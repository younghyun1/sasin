//! Collection-runner driver. Builds a [`RunPlan`] from a folder, then steps through it reusing the
//! normal send path. pm.* scripts run here on the UI thread (the QuickJS context is `!Send`), so
//! the runner cannot live entirely inside an async task — each request is sent as its own task and
//! the next step is issued when `RunnerFinished` arrives.

use iced::Task;

use crate::gui::Message;
use crate::gui::app::App;
use crate::gui::runner_state::{CurrentRun, RunnerState};
use crate::model::{HttpRequest, Node, NodePath, find_node, resolve_auth};
use crate::models::ResponseModel;
use crate::runner::{RequestOutcome, RunPlan, flatten_requests, parse_data_file};
use crate::runtime::{self, VarContext};
use crate::scripting;

impl App {
    /// Open the runner for the folder at `path` (empty path = the whole workspace root).
    pub(super) fn open_runner(&mut self, path: NodePath) {
        let (requests, name) = if path.is_empty() {
            (
                flatten_requests(&self.workspace.root, &path),
                "Workspace".to_string(),
            )
        } else {
            match find_node(&self.workspace.root, &path) {
                Some(Node::Folder(f)) => {
                    let name = if f.name.is_empty() {
                        f.slug.clone()
                    } else {
                        f.name.clone()
                    };
                    (flatten_requests(&f.children, &path), name)
                }
                _ => return,
            }
        };
        self.runner = Some(RunnerState::new(path, name, requests));
    }

    /// Build the plan from the current config and kick off the first send.
    pub(super) fn runner_start(&mut self) -> Task<Message> {
        if let Some(r) = &mut self.runner {
            let iterations = r
                .iterations_text
                .trim()
                .parse::<usize>()
                .unwrap_or(1)
                .max(1);
            r.plan = RunPlan::new(
                r.root.clone(),
                r.requests.clone(),
                iterations,
                r.data.clone(),
            );
            r.report = crate::runner::RunReport::default();
            r.cursor = 0;
            r.running = true;
            r.finished = false;
            r.current = None;
        }
        self.runner_send_current()
    }

    /// Stop advancing (an in-flight send still completes and is recorded, but no further steps run).
    pub(super) fn runner_stop(&mut self) {
        if let Some(r) = &mut self.runner {
            r.running = false;
            r.finished = true;
        }
    }

    /// Parse the data file at the current path into rows; default the iteration count to the row
    /// count so a data-driven run covers every row once.
    pub(super) fn load_runner_data(&mut self) {
        let path = match &self.runner {
            Some(r) => r.data_path.trim().to_string(),
            None => return,
        };
        if path.is_empty() {
            if let Some(r) = &mut self.runner {
                r.data.clear();
            }
            return;
        }
        match parse_data_file(std::path::Path::new(&path)) {
            Ok(rows) => {
                let count = rows.len();
                if let Some(r) = &mut self.runner {
                    r.data = rows;
                    if count > 0 {
                        r.iterations_text = count.to_string();
                    }
                }
                self.status = Some(format!("Loaded {count} data row(s)."));
            }
            Err(e) => {
                // Clear stale rows so a failed reload can't leave a previous file's data active.
                if let Some(r) = &mut self.runner {
                    r.data.clear();
                }
                self.status = Some(format!("Data file error: {e}"));
            }
        }
    }

    /// Record a completed (or failed) send and advance to the next step.
    pub(super) fn runner_finished(
        &mut self,
        send_id: u64,
        result: Result<ResponseModel, String>,
    ) -> Task<Message> {
        let current = match &self.runner {
            Some(r) if r.gen_id == send_id => r.current.clone(),
            _ => return Task::none(),
        };
        let Some(current) = current else {
            return Task::none();
        };

        let (status, error, tests, duration_ms) = match result {
            Ok(resp) => {
                let test_src = match find_node(&self.workspace.root, &current.path) {
                    Some(Node::Http(r)) => r.scripts.test.clone(),
                    _ => String::new(),
                };
                let (tests, test_error) = if test_src.trim().is_empty() {
                    (Vec::new(), None)
                } else {
                    let outcome = scripting::run_test(&test_src, &current.snapshot, &resp);
                    (outcome.tests, outcome.error)
                };
                (
                    Some(resp.status.code),
                    current.pre_error.clone().or(test_error),
                    tests,
                    resp.duration.as_millis(),
                )
            }
            Err(e) => {
                // Fold in a pre-request script error so it isn't lost when the send also fails.
                let combined = match &current.pre_error {
                    Some(pre) => format!("{pre}; {e}"),
                    None => e,
                };
                (None, Some(combined), Vec::new(), 0)
            }
        };

        if let Some(r) = &mut self.runner {
            r.report.push(RequestOutcome {
                path: current.path,
                name: current.name,
                iteration: current.iteration,
                status,
                error,
                tests,
                duration_ms,
            });
            r.current = None;
        }
        self.runner_advance()
    }

    /// Issue the send for the step at the current cursor, or finish when there are none left.
    fn runner_send_current(&mut self) -> Task<Message> {
        let next = match &self.runner {
            Some(r) if r.running => r
                .plan
                .step(r.cursor)
                .map(|s| (s.path.clone(), s.iteration, s.data.cloned())),
            _ => return Task::none(),
        };
        let Some((path, iteration, data_row)) = next else {
            if let Some(r) = &mut self.runner {
                r.running = false;
                r.finished = true;
            }
            return Task::none();
        };

        let mut request = match find_node(&self.workspace.root, &path) {
            Some(Node::Http(r)) => r.clone(),
            _ => {
                self.runner_record_skip(&path, iteration, "not an HTTP request");
                return self.runner_advance();
            }
        };
        let name = display_name(&request);
        request.auth = resolve_auth(&self.workspace.root, &path);

        let env = self
            .active_env
            .and_then(|i| self.workspace.environments.get(i));
        let mut ctx = VarContext::from_scopes(&self.workspace.globals, env);
        if let Some(row) = &data_row {
            for (k, v) in row {
                ctx.set(k.clone(), v.clone());
            }
        }
        let mut pre_error = None;
        if !request.scripts.pre_request.trim().is_empty() {
            let outcome = scripting::run_pre_request(&request.scripts.pre_request, &ctx.snapshot());
            for (k, v) in outcome.var_sets {
                ctx.set(k, v);
            }
            pre_error = outcome.error;
        }
        let resolved = runtime::resolve_request(&request, &ctx);
        let snapshot = ctx.snapshot();

        self.send_gen += 1;
        let send_id = self.send_gen;
        if let Some(r) = &mut self.runner {
            r.gen_id = send_id;
            r.current = Some(CurrentRun {
                path,
                name,
                iteration,
                pre_error,
                snapshot,
            });
        }

        let cfg = self.http_config.clone();
        let base = self.workspace_dir.clone();
        Task::perform(
            async move {
                match crate::http::execute(&cfg, &resolved, &base).await {
                    Ok(resp) => Message::RunnerFinished(send_id, Ok(resp)),
                    Err(e) => Message::RunnerFinished(send_id, Err(e)),
                }
            },
            |m| m,
        )
    }

    /// Advance the cursor; send the next step or finish when the plan is exhausted.
    fn runner_advance(&mut self) -> Task<Message> {
        let finished = match &mut self.runner {
            Some(r) if r.running => {
                r.cursor += 1;
                r.cursor >= r.plan.total_steps()
            }
            _ => return Task::none(),
        };
        if finished {
            if let Some(r) = &mut self.runner {
                r.running = false;
                r.finished = true;
            }
            Task::none()
        } else {
            self.runner_send_current()
        }
    }

    fn runner_record_skip(&mut self, path: &NodePath, iteration: usize, msg: &str) {
        if let Some(r) = &mut self.runner {
            r.report.push(RequestOutcome {
                path: path.clone(),
                name: path.last().cloned().unwrap_or_default(),
                iteration,
                status: None,
                error: Some(msg.to_string()),
                tests: Vec::new(),
                duration_ms: 0,
            });
        }
    }
}

fn display_name(req: &HttpRequest) -> String {
    if req.name.is_empty() {
        req.slug.clone()
    } else {
        req.name.clone()
    }
}
