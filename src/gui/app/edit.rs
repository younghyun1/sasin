//! Editor field handlers (environments, method/url, KV tables, auth). Body/settings/script
//! handlers live in [`super::edit_body`]; both mutate the active node via the shared accessors.

use std::collections::HashSet;

use crate::gui::app::App;
use crate::gui::messages::{AuthChoice, AuthFieldKind, EditorPanel, KvOp, KvTarget};
use crate::gui::state;
use crate::model::{ApiKeyLoc, Auth, Body, Environment, HttpRequest, Node, find_node_mut};
use crate::models::HttpMethod;
use crate::storage::layout::unique_slug;

impl App {
    pub(super) fn select_env(&mut self, idx: usize) {
        if idx < self.workspace.environments.len() {
            self.active_env = Some(idx);
        }
    }

    pub(super) fn new_env(&mut self) {
        let mut taken: HashSet<String> = self
            .workspace
            .environments
            .iter()
            .map(|e| e.slug.clone())
            .collect();
        let slug = unique_slug("env", &mut taken);
        self.workspace.environments.push(Environment {
            slug: slug.clone(),
            name: slug,
            variables: Vec::new(),
        });
        self.active_env = Some(self.workspace.environments.len() - 1);
    }

    pub(super) fn apply_env_var(&mut self, op: KvOp) {
        if let Some(idx) = self.active_env
            && let Some(env) = self.workspace.environments.get_mut(idx)
        {
            state::apply_kv_variable(&mut env.variables, op);
        }
    }

    /// `&mut HttpRequest` for the active tab's node, if it is an HTTP request.
    pub(super) fn active_http_mut(&mut self) -> Option<&mut HttpRequest> {
        let path = self
            .active
            .and_then(|i| self.tabs.get(i))
            .map(|t| t.path.clone())?;
        match find_node_mut(&mut self.workspace.root, &path)? {
            Node::Http(r) => Some(r),
            _ => None,
        }
    }

    pub(super) fn mark_active_dirty(&mut self) {
        if let Some(i) = self.active
            && let Some(tab) = self.tabs.get_mut(i)
        {
            tab.dirty = true;
        }
    }

    pub(super) fn select_panel(&mut self, panel: EditorPanel) {
        if let Some(i) = self.active
            && let Some(tab) = self.tabs.get_mut(i)
        {
            tab.panel = panel;
        }
    }

    pub(super) fn set_method(&mut self, method: HttpMethod) {
        self.mark_active_dirty();
        if let Some(r) = self.active_http_mut() {
            r.method = method.as_str().to_string();
        }
    }

    pub(super) fn set_url(&mut self, url: String) {
        self.mark_active_dirty();
        if let Some(r) = self.active_http_mut() {
            r.url = url;
        }
    }

    pub(super) fn apply_kv(&mut self, target: KvTarget, op: KvOp) {
        self.mark_active_dirty();
        let Some(r) = self.active_http_mut() else {
            return;
        };
        match target {
            KvTarget::Params => state::apply_kv_entry(&mut r.params, op),
            KvTarget::Headers => state::apply_kv_entry(&mut r.headers, op),
            KvTarget::UrlEncoded => {
                if let Body::UrlEncoded { fields } = &mut r.body {
                    state::apply_kv_entry(fields, op);
                }
            }
            KvTarget::FormData => {
                if let Body::FormData { parts } = &mut r.body {
                    state::apply_kv_formpart(parts, op);
                }
            }
        }
    }

    pub(super) fn set_auth_choice(&mut self, choice: AuthChoice) {
        self.mark_active_dirty();
        if let Some(r) = self.active_http_mut() {
            r.auth = match choice {
                AuthChoice::None => Auth::None,
                AuthChoice::Inherit => Auth::Inherit,
                AuthChoice::Basic => Auth::Basic {
                    user: String::new(),
                    pass: String::new(),
                },
                AuthChoice::Bearer => Auth::Bearer {
                    token: String::new(),
                },
                AuthChoice::ApiKey => Auth::ApiKey {
                    key: String::new(),
                    value: String::new(),
                    add_to: ApiKeyLoc::Header,
                },
                AuthChoice::OAuth2 => Auth::OAuth2 {
                    token: String::new(),
                },
            };
        }
    }

    pub(super) fn set_auth_field(&mut self, kind: AuthFieldKind, value: String) {
        self.mark_active_dirty();
        let Some(r) = self.active_http_mut() else {
            return;
        };
        match (&mut r.auth, kind) {
            (Auth::Basic { user, .. }, AuthFieldKind::User) => *user = value,
            (Auth::Basic { pass, .. }, AuthFieldKind::Pass) => *pass = value,
            (Auth::Bearer { token }, AuthFieldKind::Token)
            | (Auth::OAuth2 { token }, AuthFieldKind::Token) => *token = value,
            (Auth::ApiKey { key, .. }, AuthFieldKind::ApiKeyName) => *key = value,
            (Auth::ApiKey { value: v, .. }, AuthFieldKind::ApiKeyValue) => *v = value,
            _ => {}
        }
    }

    pub(super) fn set_apikey_in_header(&mut self, in_header: bool) {
        self.mark_active_dirty();
        if let Some(r) = self.active_http_mut()
            && let Auth::ApiKey { add_to, .. } = &mut r.auth
        {
            *add_to = if in_header {
                ApiKeyLoc::Header
            } else {
                ApiKeyLoc::Query
            };
        }
    }
}
