//! Unit tests for the runner core: flattening, step indexing, data parsing, and aggregation.

use crate::model::{Folder, HttpRequest, Node, WsRequest};
use crate::scripting::TestResult;

use super::data::{parse_csv, parse_json};
use super::plan::{RunPlan, flatten_requests};
use super::report::{RequestOutcome, RunReport};

fn folder(slug: &str, children: Vec<Node>) -> Node {
    Node::Folder(Folder {
        children,
        ..Folder::new(slug, slug)
    })
}

fn http(slug: &str) -> Node {
    Node::Http(HttpRequest::new(slug, slug, "GET", "https://x"))
}

#[test]
fn flatten_is_preorder_and_skips_websockets() {
    let tree = vec![
        http("a"),
        folder(
            "grp",
            vec![
                http("b"),
                Node::Ws(WsRequest::new("sock", "sock", "wss://x")),
                http("c"),
            ],
        ),
        http("d"),
    ];
    let paths = flatten_requests(&tree, &Vec::new());
    let joined: Vec<String> = paths.iter().map(|p| p.join("/")).collect();
    assert_eq!(joined, vec!["a", "grp/b", "grp/c", "d"]);
}

#[test]
fn flatten_respects_a_base_prefix() {
    let tree = vec![http("x")];
    let paths = flatten_requests(&tree, &vec!["root".to_string()]);
    assert_eq!(paths[0], vec!["root".to_string(), "x".to_string()]);
}

#[test]
fn plan_steps_are_row_major_over_iterations() {
    let reqs = vec![vec!["a".to_string()], vec!["b".to_string()]];
    let plan = RunPlan::new(Vec::new(), reqs, 3, Vec::new());
    assert_eq!(plan.total_steps(), 6);
    // iteration 0: a, b ; iteration 1: a, b ; ...
    assert_eq!(
        plan.step(0).map(|s| (s.iteration, s.path.join("/"))),
        Some((0, "a".into()))
    );
    assert_eq!(
        plan.step(1).map(|s| (s.iteration, s.path.join("/"))),
        Some((0, "b".into()))
    );
    assert_eq!(
        plan.step(2).map(|s| (s.iteration, s.path.join("/"))),
        Some((1, "a".into()))
    );
    assert_eq!(
        plan.step(5).map(|s| (s.iteration, s.path.join("/"))),
        Some((2, "b".into()))
    );
    assert!(plan.step(6).is_none());
}

#[test]
fn plan_iterations_floor_at_one_and_empty_requests_yield_no_steps() {
    let empty = RunPlan::new(Vec::new(), Vec::new(), 5, Vec::new());
    assert_eq!(empty.total_steps(), 0);
    assert!(empty.step(0).is_none());

    let clamped = RunPlan::new(Vec::new(), vec![vec!["a".to_string()]], 0, Vec::new());
    assert_eq!(clamped.iterations, 1);
}

#[test]
fn plan_step_exposes_matching_data_row() {
    let data = parse_csv("user,pw\nalice,1\nbob,2\n").expect("csv parses");
    let plan = RunPlan::new(Vec::new(), vec![vec!["login".to_string()]], 2, data);
    assert_eq!(
        plan.step(0)
            .and_then(|s| s.data)
            .and_then(|d| d.get("user"))
            .cloned(),
        Some("alice".to_string())
    );
    assert_eq!(
        plan.step(1)
            .and_then(|s| s.data)
            .and_then(|d| d.get("user"))
            .cloned(),
        Some("bob".to_string())
    );
}

#[test]
fn csv_parses_header_and_rows() {
    let rows = parse_csv("a,b\n1,2\n3,4\n").expect("parses");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get("a").cloned(), Some("1".to_string()));
    assert_eq!(rows[1].get("b").cloned(), Some("4".to_string()));
}

#[test]
fn json_parses_array_of_objects_and_stringifies_scalars() {
    let rows = parse_json(r#"[{"s":"x","n":7,"b":true,"z":null}]"#).expect("parses");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("s").cloned(), Some("x".to_string()));
    assert_eq!(rows[0].get("n").cloned(), Some("7".to_string()));
    assert_eq!(rows[0].get("b").cloned(), Some("true".to_string()));
    assert_eq!(rows[0].get("z").cloned(), Some(String::new()));
}

#[test]
fn json_rejects_non_array_and_non_object_rows() {
    assert!(parse_json("{}").is_err());
    assert!(parse_json("[1,2]").is_err());
}

#[test]
fn report_rolls_up_pass_fail_counts() {
    let mut report = RunReport::default();
    report.push(RequestOutcome {
        path: vec!["a".to_string()],
        name: "a".to_string(),
        iteration: 0,
        status: Some(200),
        error: None,
        tests: vec![TestResult {
            name: "ok".to_string(),
            passed: true,
            error: None,
        }],
        duration_ms: 5,
    });
    report.push(RequestOutcome {
        path: vec!["b".to_string()],
        name: "b".to_string(),
        iteration: 0,
        status: Some(500),
        error: None,
        tests: vec![TestResult {
            name: "fail".to_string(),
            passed: false,
            error: Some("nope".to_string()),
        }],
        duration_ms: 9,
    });
    report.push(RequestOutcome {
        path: vec!["c".to_string()],
        name: "c".to_string(),
        iteration: 0,
        status: None,
        error: Some("timeout".to_string()),
        tests: Vec::new(),
        duration_ms: 0,
    });
    assert_eq!(report.requests(), 3);
    assert_eq!(report.passed_requests(), 1);
    assert_eq!(report.failed_requests(), 2);
    assert_eq!(report.total_assertions(), 2);
    assert_eq!(report.passed_assertions(), 1);
    assert!(!report.all_passed());
}
