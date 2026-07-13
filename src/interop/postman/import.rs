//! Map a Postman Collection v2.1 JSON document onto the sasin workspace model.
//!
//! The whole collection becomes one top-level [`Folder`] (collection-level auth, variables,
//! and scripts map straight onto folder inheritance). Unsupported constructs degrade with a
//! warning instead of failing the import.

use std::collections::HashSet;

use crate::interop::postman::schema::{
    AuthDef, BodyDef, Collection, Description, EventDef, Item, RequestUnion, UrlUnion, auth_param,
};
use crate::model::{
    ApiKeyLoc, Auth, Body, Folder, FormKind, FormPart, HttpRequest, KvEntry, Node, RawLang,
    Scripts, Variable,
};
use crate::storage::layout::unique_slug;

/// Result of an import: the ready-to-insert folder plus human-readable warnings.
#[derive(Debug, Clone)]
pub struct PostmanImport {
    pub folder: Folder,
    pub warnings: Vec<String>,
    pub request_count: usize,
}

/// Parse a Postman Collection v2.1 export into a workspace folder.
pub fn from_postman(json: &str) -> Result<PostmanImport, String> {
    let collection: Collection =
        serde_json::from_str(json).map_err(|e| format!("not a Postman collection: {e}"))?;
    if collection.info.name.is_empty() && collection.item.is_empty() {
        return Err("not a Postman collection: no info.name and no items".to_string());
    }

    let mut warnings = Vec::new();
    let mut count = 0usize;

    let name = if collection.info.name.is_empty() {
        "Imported collection".to_string()
    } else {
        collection.info.name.clone()
    };
    let mut taken = HashSet::new();
    let mut folder = Folder::new(unique_slug(&name, &mut taken), name);
    folder.description = collection.info.description.as_ref().map(desc_text);
    folder.auth = collection
        .auth
        .as_ref()
        .map(|a| map_auth(a, &mut warnings))
        .unwrap_or_default();
    folder.variables = collection
        .variable
        .iter()
        .map(|v| Variable {
            key: v.key.clone(),
            value: v
                .value
                .as_ref()
                .map(|val| match val {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .unwrap_or_default(),
            enabled: true,
            secret: false,
            description: None,
        })
        .collect();
    folder.scripts = map_events(&collection.event, "collection", &mut warnings);
    folder.children = map_items(&collection.item, &mut warnings, &mut count);

    Ok(PostmanImport {
        folder,
        warnings,
        request_count: count,
    })
}

fn desc_text(d: &Description) -> String {
    d.text().to_string()
}

fn map_items(items: &[Item], warnings: &mut Vec<String>, count: &mut usize) -> Vec<Node> {
    let mut taken: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for item in items {
        let display = if item.name.is_empty() {
            "untitled"
        } else {
            &item.name
        };
        let slug = unique_slug(display, &mut taken);
        if let Some(children) = &item.item {
            let mut folder = Folder::new(slug, item.name.clone());
            folder.description = item.description.as_ref().map(desc_text);
            folder.auth = item
                .auth
                .as_ref()
                .map(|a| map_auth(a, warnings))
                .unwrap_or_default();
            folder.scripts = map_events(&item.event, display, warnings);
            folder.children = map_items(children, warnings, count);
            out.push(Node::Folder(folder));
        } else if let Some(request) = &item.request {
            let mut req = map_request(request, warnings, display);
            req.slug = slug;
            req.name = item.name.clone();
            if req.description.is_none() {
                req.description = item.description.as_ref().map(desc_text);
            }
            req.scripts = map_events(&item.event, display, warnings);
            *count += 1;
            out.push(Node::Http(req));
        } else {
            warnings.push(format!("skipped `{display}`: neither folder nor request"));
        }
    }
    out
}

fn map_request(request: &RequestUnion, warnings: &mut Vec<String>, ctx: &str) -> HttpRequest {
    let mut req = HttpRequest::default();
    let obj = match request {
        RequestUnion::Url(url) => {
            req.url = url.clone();
            return req;
        }
        RequestUnion::Object(obj) => obj,
    };
    req.method = obj
        .method
        .as_deref()
        .unwrap_or("GET")
        .trim()
        .to_ascii_uppercase();
    req.description = obj.description.as_ref().map(desc_text);

    match &obj.url {
        Some(UrlUnion::Raw(raw)) => req.url = raw.clone(),
        Some(UrlUnion::Object(url)) => {
            // Params come from query[]; strip any query string from raw so the send path
            // (which re-appends enabled params) does not double them.
            req.url = match url.raw.split_once('?') {
                Some((base, _)) if !url.query.is_empty() => base.to_string(),
                _ => url.raw.clone(),
            };
            req.params = url
                .query
                .iter()
                .map(|q| KvEntry {
                    key: q.key.clone(),
                    value: q.value.clone().unwrap_or_default(),
                    enabled: !q.disabled,
                })
                .collect();
            if !url.variable.is_empty() {
                warnings.push(format!(
                    "`{ctx}`: path variables (:name) are not supported; left literal in the URL"
                ));
            }
        }
        None => {}
    }

    req.headers = obj
        .header
        .iter()
        .map(|h| KvEntry {
            key: h.key.clone(),
            value: h.value.clone(),
            enabled: !h.disabled,
        })
        .collect();

    req.auth = obj
        .auth
        .as_ref()
        .map(|a| map_auth(a, warnings))
        .unwrap_or_default();
    if let Some(body) = &obj.body {
        req.body = map_body(body, warnings, ctx);
    }
    req
}

fn map_body(body: &BodyDef, warnings: &mut Vec<String>, ctx: &str) -> Body {
    match body.mode.as_deref() {
        None => Body::None,
        Some("raw") => Body::Raw {
            language: map_raw_lang(body, warnings, ctx),
            text: body.raw.clone().unwrap_or_default(),
        },
        Some("urlencoded") => Body::UrlEncoded {
            fields: body
                .urlencoded
                .iter()
                .map(|f| KvEntry {
                    key: f.key.clone(),
                    value: f.value.clone(),
                    enabled: !f.disabled,
                })
                .collect(),
        },
        Some("formdata") => Body::FormData {
            parts: body
                .formdata
                .iter()
                .map(|f| map_form_part(f, warnings, ctx))
                .collect(),
        },
        Some("file") => {
            let file = body
                .file
                .as_ref()
                .and_then(|f| f.src.clone())
                .unwrap_or_default();
            if !file.is_empty() {
                warnings.push(format!(
                    "`{ctx}`: binary body path `{file}` is machine-local; adjust to a \
                     workspace-relative path"
                ));
            }
            Body::Binary { file }
        }
        Some("graphql") => {
            let gql = body.graphql.as_ref();
            Body::GraphQl {
                query: gql.and_then(|g| g.query.clone()).unwrap_or_default(),
                variables: gql.and_then(|g| g.variables.clone()).unwrap_or_default(),
            }
        }
        Some(other) => {
            warnings.push(format!("`{ctx}`: unsupported body mode `{other}`; dropped"));
            Body::None
        }
    }
}

fn map_raw_lang(body: &BodyDef, warnings: &mut Vec<String>, ctx: &str) -> RawLang {
    let lang = body
        .options
        .as_ref()
        .and_then(|o| o.raw.as_ref())
        .and_then(|r| r.language.as_deref());
    match lang.map(str::to_ascii_lowercase).as_deref() {
        None | Some("json") => RawLang::Json,
        Some("text") => RawLang::Text,
        Some("xml") => RawLang::Xml,
        Some("html") => RawLang::Html,
        Some("javascript" | "js") => RawLang::Javascript,
        Some(other) => {
            warnings.push(format!(
                "`{ctx}`: unknown raw language `{other}`; treated as text"
            ));
            RawLang::Text
        }
    }
}

fn map_form_part(
    part: &crate::interop::postman::schema::FormDataDef,
    warnings: &mut Vec<String>,
    ctx: &str,
) -> FormPart {
    let is_file = part.kind.as_deref() == Some("file");
    let src = match &part.src {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Array(list)) => {
            if list.len() > 1 {
                warnings.push(format!(
                    "`{ctx}`: multi-file form field `{}`; only the first file was kept",
                    part.key
                ));
            }
            list.first()
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()
        }
        _ => String::new(),
    };
    if is_file && !src.is_empty() {
        warnings.push(format!(
            "`{ctx}`: form file `{}` points at machine-local `{src}`; adjust to a \
             workspace-relative path",
            part.key
        ));
    }
    FormPart {
        key: part.key.clone(),
        kind: if is_file {
            FormKind::File
        } else {
            FormKind::Text
        },
        value: part.value.clone().unwrap_or_default(),
        src,
        enabled: !part.disabled,
    }
}

fn map_auth(auth: &AuthDef, warnings: &mut Vec<String>) -> Auth {
    match auth.kind.as_str() {
        "noauth" => Auth::None,
        "basic" => Auth::Basic {
            user: auth_param(&auth.basic, "username").unwrap_or_default(),
            pass: auth_param(&auth.basic, "password").unwrap_or_default(),
        },
        "bearer" => Auth::Bearer {
            token: auth_param(&auth.bearer, "token").unwrap_or_default(),
        },
        "apikey" => Auth::ApiKey {
            key: auth_param(&auth.apikey, "key").unwrap_or_default(),
            value: auth_param(&auth.apikey, "value").unwrap_or_default(),
            add_to: match auth_param(&auth.apikey, "in").as_deref() {
                Some("query") => ApiKeyLoc::Query,
                _ => ApiKeyLoc::Header,
            },
        },
        "oauth2" => {
            let token = auth_param(&auth.oauth2, "accessToken")
                .or_else(|| auth_param(&auth.oauth2, "access_token"))
                .unwrap_or_default();
            if token.is_empty() {
                warnings.push(
                    "oauth2 auth imported without an access token (grant config is not \
                     supported); paste a token into the Auth panel"
                        .to_string(),
                );
            }
            Auth::OAuth2 { token }
        }
        "" => Auth::Inherit,
        other => {
            warnings.push(format!("unsupported auth type `{other}`; set to inherit"));
            Auth::Inherit
        }
    }
}

fn map_events(events: &[EventDef], ctx: &str, warnings: &mut Vec<String>) -> Scripts {
    let mut scripts = Scripts::default();
    for event in events {
        let Some(script) = &event.script else {
            continue;
        };
        let source = script.exec.joined();
        for unsupported in ["pm.sendRequest", "pm.cookies", "postman.setNextRequest"] {
            if source.contains(unsupported) {
                warnings.push(format!(
                    "`{ctx}`: script uses `{unsupported}`, which sasin's pm.* subset does not \
                     support"
                ));
            }
        }
        match event.listen.as_str() {
            "prerequest" => scripts.pre_request = source,
            "test" => scripts.test = source,
            other => warnings.push(format!("`{ctx}`: unknown event `{other}` ignored")),
        }
    }
    scripts
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A representative v2.1 export: nested folder, both URL forms, several body modes,
    /// auth types, events, and collection variables.
    const SAMPLE: &str = r#"{
      "info": { "name": "Demo API", "description": "Sample", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json" },
      "auth": { "type": "bearer", "bearer": [ { "key": "token", "value": "{{token}}", "type": "string" } ] },
      "variable": [ { "key": "base", "value": "https://api.example.com" } ],
      "event": [ { "listen": "prerequest", "script": { "exec": ["pm.environment.set('t', 1);"] } } ],
      "item": [
        {
          "name": "Users",
          "item": [
            {
              "name": "List users",
              "event": [ { "listen": "test", "script": { "exec": ["pm.test('ok', () => pm.response.to.have.status(200));"] } } ],
              "request": {
                "method": "GET",
                "url": { "raw": "{{base}}/users?page=1", "query": [ { "key": "page", "value": "1" }, { "key": "debug", "value": "true", "disabled": true } ] },
                "header": [ { "key": "Accept", "value": "application/json" } ]
              }
            },
            {
              "name": "Create user",
              "request": {
                "method": "POST",
                "url": "{{base}}/users",
                "auth": { "type": "basic", "basic": [ { "key": "username", "value": "u" }, { "key": "password", "value": "p" } ] },
                "body": { "mode": "raw", "raw": "{\"name\":\"x\"}", "options": { "raw": { "language": "json" } } }
              }
            }
          ]
        },
        {
          "name": "Upload",
          "request": {
            "method": "POST",
            "url": "{{base}}/upload",
            "body": { "mode": "formdata", "formdata": [
              { "key": "meta", "type": "text", "value": "v" },
              { "key": "file", "type": "file", "src": "/home/me/a.png" }
            ] }
          }
        },
        {
          "name": "GraphQL",
          "request": {
            "method": "POST",
            "url": "{{base}}/graphql",
            "auth": { "type": "apikey", "apikey": [ { "key": "key", "value": "X-Key" }, { "key": "value", "value": "abc" }, { "key": "in", "value": "query" } ] },
            "body": { "mode": "graphql", "graphql": { "query": "query { me }", "variables": "{}" } }
          }
        }
      ]
    }"#;

    fn find<'a>(nodes: &'a [Node], slug: &str) -> &'a Node {
        match nodes.iter().find(|n| n.slug() == slug) {
            Some(n) => n,
            None => panic!("missing node `{slug}`"),
        }
    }

    #[test]
    fn imports_representative_collection() {
        let import = match from_postman(SAMPLE) {
            Ok(i) => i,
            Err(e) => panic!("import failed: {e}"),
        };
        assert_eq!(import.request_count, 4);
        let folder = &import.folder;
        assert_eq!(folder.name, "Demo API");
        assert_eq!(
            folder.auth,
            Auth::Bearer {
                token: "{{token}}".into()
            }
        );
        assert_eq!(folder.variables.len(), 1);
        assert!(folder.scripts.pre_request.contains("pm.environment.set"));

        let users = match find(&folder.children, "users") {
            Node::Folder(f) => f,
            other => panic!("expected folder, got {other:?}"),
        };
        let list = match find(&users.children, "list-users") {
            Node::Http(r) => r,
            other => panic!("expected request, got {other:?}"),
        };
        assert_eq!(list.method, "GET");
        assert_eq!(list.url, "{{base}}/users");
        assert_eq!(list.params.len(), 2);
        assert!(!list.params[1].enabled);
        assert!(list.scripts.test.contains("pm.test"));
        assert_eq!(list.auth, Auth::Inherit);

        let create = match find(&users.children, "create-user") {
            Node::Http(r) => r,
            other => panic!("expected request, got {other:?}"),
        };
        assert_eq!(
            create.auth,
            Auth::Basic {
                user: "u".into(),
                pass: "p".into()
            }
        );
        assert!(
            matches!(&create.body, Body::Raw { language: RawLang::Json, text } if text.contains("name"))
        );

        let upload = match find(&folder.children, "upload") {
            Node::Http(r) => r,
            other => panic!("expected request, got {other:?}"),
        };
        match &upload.body {
            Body::FormData { parts } => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[1].kind, FormKind::File);
                assert_eq!(parts[1].src, "/home/me/a.png");
            }
            other => panic!("expected formdata, got {other:?}"),
        }
        // Machine-local file path produces a warning.
        assert!(import.warnings.iter().any(|w| w.contains("machine-local")));

        let gql = match find(&folder.children, "graphql") {
            Node::Http(r) => r,
            other => panic!("expected request, got {other:?}"),
        };
        assert_eq!(
            gql.auth,
            Auth::ApiKey {
                key: "X-Key".into(),
                value: "abc".into(),
                add_to: ApiKeyLoc::Query
            }
        );
        assert!(matches!(&gql.body, Body::GraphQl { query, .. } if query.contains("me")));
    }

    #[test]
    fn rejects_non_collections() {
        assert!(from_postman("not json").is_err());
        assert!(from_postman("{}").is_err());
    }

    #[test]
    fn bare_string_request_becomes_get() {
        let json = r#"{ "info": { "name": "C" }, "item": [ { "name": "ping", "request": "https://x.dev/ping" } ] }"#;
        let import = match from_postman(json) {
            Ok(i) => i,
            Err(e) => panic!("{e}"),
        };
        match find(&import.folder.children, "ping") {
            Node::Http(r) => {
                assert_eq!(r.method, "GET");
                assert_eq!(r.url, "https://x.dev/ping");
            }
            other => panic!("expected request, got {other:?}"),
        }
    }
}
