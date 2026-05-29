//! Editor field handlers: mutate the active HTTP request node directly from editor messages.

use std::collections::HashSet;

use iced::widget::text_editor;

use crate::gui::app::App;
use crate::gui::messages::{
    AuthChoice, AuthFieldKind, BodyModeChoice, EditorPanel, KvOp, KvTarget, SettingFlag,
};
use crate::gui::state;
use crate::model::{
    ApiKeyLoc, Auth, Body, Environment, FormKind, HttpRequest, Node, RawLang, find_node_mut,
};
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
    fn active_http_mut(&mut self) -> Option<&mut HttpRequest> {
        let path = self
            .active
            .and_then(|i| self.tabs.get(i))
            .map(|t| t.path.clone())?;
        match find_node_mut(&mut self.workspace.root, &path)? {
            Node::Http(r) => Some(r),
            _ => None,
        }
    }

    fn mark_active_dirty(&mut self) {
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

    pub(super) fn set_body_mode(&mut self, choice: BodyModeChoice) {
        self.mark_active_dirty();
        let new_body = match choice {
            BodyModeChoice::None => Body::None,
            BodyModeChoice::Raw => Body::Raw {
                language: RawLang::default(),
                text: String::new(),
            },
            BodyModeChoice::UrlEncoded => Body::UrlEncoded { fields: Vec::new() },
            BodyModeChoice::FormData => Body::FormData { parts: Vec::new() },
            BodyModeChoice::Binary => Body::Binary {
                file: String::new(),
            },
            BodyModeChoice::GraphQl => Body::GraphQl {
                query: String::new(),
                variables: String::new(),
            },
        };
        if let Some(r) = self.active_http_mut() {
            r.body = new_body.clone();
        }
        if let Some(i) = self.active
            && let Some(tab) = self.tabs.get_mut(i)
        {
            tab.reload_body(&new_body);
        }
    }

    pub(super) fn set_raw_lang(&mut self, lang: RawLang) {
        self.mark_active_dirty();
        if let Some(r) = self.active_http_mut()
            && let Body::Raw { language, .. } = &mut r.body
        {
            *language = lang;
        }
    }

    pub(super) fn set_form_kind(&mut self, index: usize, is_file: bool) {
        self.mark_active_dirty();
        if let Some(r) = self.active_http_mut()
            && let Body::FormData { parts } = &mut r.body
            && let Some(part) = parts.get_mut(index)
        {
            part.kind = if is_file {
                FormKind::File
            } else {
                FormKind::Text
            };
        }
    }

    pub(super) fn set_form_src(&mut self, index: usize, src: String) {
        self.mark_active_dirty();
        if let Some(r) = self.active_http_mut()
            && let Body::FormData { parts } = &mut r.body
            && let Some(part) = parts.get_mut(index)
        {
            part.src = src;
        }
    }

    pub(super) fn set_binary_file(&mut self, file: String) {
        self.mark_active_dirty();
        if let Some(r) = self.active_http_mut()
            && let Body::Binary { file: f } = &mut r.body
        {
            *f = file;
        }
    }

    pub(super) fn set_timeout(&mut self, text: String) {
        let parsed = text.trim().parse::<u64>().ok();
        if let Some(i) = self.active
            && let Some(tab) = self.tabs.get_mut(i)
        {
            tab.timeout_text = text;
            tab.dirty = true;
        }
        if let Some(ms) = parsed
            && let Some(r) = self.active_http_mut()
        {
            r.settings.timeout_ms = ms;
        }
    }

    pub(super) fn set_flag(&mut self, flag: SettingFlag, value: bool) {
        self.mark_active_dirty();
        if let Some(r) = self.active_http_mut() {
            match flag {
                SettingFlag::FollowRedirects => r.settings.follow_redirects = value,
                SettingFlag::VerifyTls => r.settings.verify_tls = value,
                SettingFlag::CookieJar => r.settings.use_cookie_jar = value,
            }
        }
    }

    pub(super) fn set_proxy(&mut self, proxy: String) {
        self.mark_active_dirty();
        if let Some(r) = self.active_http_mut() {
            r.settings.proxy = if proxy.trim().is_empty() {
                None
            } else {
                Some(proxy)
            };
        }
    }

    /// Perform a script-editor action on the pre-request or test buffer, then sync to the node.
    pub(super) fn script_action(&mut self, action: text_editor::Action, is_test: bool) {
        let Some(i) = self.active else {
            return;
        };
        let edited = action.is_edit();
        if is_test {
            self.tabs[i].test_script.perform(action);
        } else {
            self.tabs[i].pre_script.perform(action);
        }
        if !edited {
            return;
        }
        self.tabs[i].dirty = true;
        let path = self.tabs[i].path.clone();
        if let Some(node) = find_node_mut(&mut self.workspace.root, &path) {
            state::sync_scripts(&self.tabs[i], node);
        }
    }

    /// Perform a body-editor action on the right buffer, then sync it into the node.
    pub(super) fn body_action(&mut self, action: text_editor::Action, gql_vars: bool) {
        let Some(i) = self.active else {
            return;
        };
        let edited = action.is_edit();
        if gql_vars {
            self.tabs[i].gql_vars.perform(action);
        } else {
            self.tabs[i].body.perform(action);
        }
        if !edited {
            return;
        }
        self.tabs[i].dirty = true;
        let path = self.tabs[i].path.clone();
        if let Some(node) = find_node_mut(&mut self.workspace.root, &path) {
            state::sync_body(&self.tabs[i], node);
        }
    }
}
