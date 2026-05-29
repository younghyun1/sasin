//! QuickJS-backed implementation of the scripting entry points (feature `scripting`).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use rquickjs::{Context, Ctx, Runtime};
use serde::Deserialize;

use crate::models::ResponseModel;
use crate::scripting::{ScriptOutcome, TestResult};

/// Wall-clock budget for a single script run.
const SCRIPT_TIMEOUT: Duration = Duration::from_millis(2000);

/// The `pm.*` + `console` surface, defined in JS over host-injected globals
/// (`__env`, `__sets`, `__console`, `__tests`, `__resp`).
const PM_PRELUDE: &str = r#"
var console = {
  log: function () { __console.push(Array.prototype.slice.call(arguments).map(String).join(' ')); }
};
console.info = console.log; console.warn = console.log; console.error = console.log;

function __setVar(k, v) { __env[k] = String(v); __sets[k] = String(v); }
function __expect(actual) {
  function fail(msg) { throw new Error(msg); }
  var api = { to: {} };
  function equal(exp) {
    if (JSON.stringify(actual) !== JSON.stringify(exp))
      fail('expected ' + JSON.stringify(actual) + ' to equal ' + JSON.stringify(exp));
  }
  api.to.equal = equal;
  api.to.eql = equal;
  api.to.include = function (sub) {
    if (String(actual).indexOf(String(sub)) < 0) fail('expected ' + actual + ' to include ' + sub);
  };
  api.to.be = {
    above: function (n) { if (!(actual > n)) fail('expected ' + actual + ' to be above ' + n); },
    below: function (n) { if (!(actual < n)) fail('expected ' + actual + ' to be below ' + n); }
  };
  return api;
}

var pm = {
  environment: { get: function (k) { return __env[k]; }, set: __setVar },
  variables: { get: function (k) { return __env[k]; }, set: __setVar },
  globals: { get: function (k) { return __env[k]; }, set: __setVar },
  expect: __expect,
  test: function (name, fn) {
    try { fn(); __tests.push({ name: name, passed: true, error: null }); }
    catch (e) { __tests.push({ name: name, passed: false, error: String(e && e.message ? e.message : e) }); }
  }
};

if (__resp) {
  pm.response = {
    code: __resp.code,
    status: __resp.status,
    responseTime: __resp.responseTime,
    headers: __resp.headers,
    text: function () { return __resp.body; },
    json: function () { return JSON.parse(__resp.body); },
    to: {
      have: {
        status: function (c) { if (__resp.code !== c) throw new Error('expected status ' + __resp.code + ' to be ' + c); },
        header: function (h) { if (!(h in __resp.headers)) throw new Error('missing header ' + h); }
      }
    }
  };
}
"#;

#[derive(Deserialize)]
struct TestJson {
    name: String,
    passed: bool,
    error: Option<String>,
}

pub fn run_pre_request(script: &str, env: &HashMap<String, String>) -> ScriptOutcome {
    run(script, env, None)
}

pub fn run_test(
    script: &str,
    env: &HashMap<String, String>,
    response: &ResponseModel,
) -> ScriptOutcome {
    run(script, env, Some(response))
}

fn run(
    script: &str,
    env: &HashMap<String, String>,
    response: Option<&ResponseModel>,
) -> ScriptOutcome {
    let runtime = match Runtime::new() {
        Ok(r) => r,
        Err(e) => return error_outcome(format!("runtime: {e}")),
    };
    let deadline = Instant::now() + SCRIPT_TIMEOUT;
    runtime.set_interrupt_handler(Some(Box::new(move || Instant::now() >= deadline)));

    let context = match Context::full(&runtime) {
        Ok(c) => c,
        Err(e) => return error_outcome(format!("context: {e}")),
    };

    let env_json = serde_json::to_string(env).unwrap_or_else(|_| "{}".to_string());
    let resp_setup = response.map_or_else(|| "globalThis.__resp = null;".to_string(), response_js);

    context.with(|ctx| {
        let setup = format!(
            "globalThis.__env={env_json};globalThis.__sets={{}};globalThis.__console=[];globalThis.__tests=[];{resp_setup}\n{PM_PRELUDE}"
        );
        if let Err(e) = ctx.eval::<(), _>(setup) {
            return error_outcome(format!("prelude error: {e}"));
        }
        let script_error = ctx.eval::<(), _>(script.to_string()).err().map(|e| format!("{e}"));
        let mut outcome = collect(&ctx);
        outcome.error = script_error;
        outcome
    })
}

fn collect(ctx: &Ctx<'_>) -> ScriptOutcome {
    let sets = eval_json(ctx, "__sets");
    let console_raw = eval_json(ctx, "__console");
    let tests_raw = eval_json(ctx, "__tests");

    let var_map: HashMap<String, String> = serde_json::from_str(&sets).unwrap_or_default();
    let console: Vec<String> = serde_json::from_str(&console_raw).unwrap_or_default();
    let tests: Vec<TestJson> = serde_json::from_str(&tests_raw).unwrap_or_default();

    ScriptOutcome {
        var_sets: var_map.into_iter().collect(),
        console,
        tests: tests
            .into_iter()
            .map(|t| TestResult {
                name: t.name,
                passed: t.passed,
                error: t.error,
            })
            .collect(),
        error: None,
    }
}

fn eval_json(ctx: &Ctx<'_>, global: &str) -> String {
    ctx.eval::<String, _>(format!("JSON.stringify(globalThis.{global})"))
        .unwrap_or_else(|_| "null".to_string())
}

fn response_js(r: &ResponseModel) -> String {
    let body = serde_json::to_string(&r.body).unwrap_or_else(|_| "\"\"".to_string());
    let status = serde_json::to_string(&r.status.reason).unwrap_or_else(|_| "\"\"".to_string());
    let headers: HashMap<String, String> = r
        .headers
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let headers_json = serde_json::to_string(&headers).unwrap_or_else(|_| "{}".to_string());
    format!(
        "globalThis.__resp = {{ code: {}, status: {}, body: {}, headers: {}, responseTime: {} }};",
        r.status.code,
        status,
        body,
        headers_json,
        r.duration.as_millis()
    )
}

fn error_outcome(message: String) -> ScriptOutcome {
    ScriptOutcome {
        error: Some(message),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn pre_request_sets_vars_and_logs() {
        let out = run_pre_request(
            "pm.environment.set('x', 42); console.log('hello', 1);",
            &HashMap::new(),
        );
        assert!(out.error.is_none(), "error: {:?}", out.error);
        assert!(out.var_sets.iter().any(|(k, v)| k == "x" && v == "42"));
        assert!(out.console.iter().any(|l| l.contains("hello")));
    }

    #[test]
    fn test_script_runs_assertions() {
        let resp = ResponseModel::new(
            200,
            "OK",
            reqwest::header::HeaderMap::new(),
            "{\"id\":7}",
            Duration::from_millis(5),
        );
        let script = "pm.test('status', () => pm.response.to.have.status(200));\
            pm.test('id', () => pm.expect(pm.response.json().id).to.equal(7));\
            pm.test('fails', () => pm.expect(1).to.equal(2));";
        let out = run_test(script, &HashMap::new(), &resp);
        assert!(out.error.is_none(), "error: {:?}", out.error);
        assert_eq!(out.tests.len(), 3);
        let passed = |name: &str| out.tests.iter().find(|t| t.name == name).map(|t| t.passed);
        assert_eq!(passed("status"), Some(true));
        assert_eq!(passed("id"), Some(true));
        assert_eq!(passed("fails"), Some(false));
    }
}
