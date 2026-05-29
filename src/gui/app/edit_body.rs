//! Editor field handlers for body modes, per-request settings, and scripts.

use iced::widget::text_editor;

use crate::gui::app::App;
use crate::gui::messages::{BodyModeChoice, SettingFlag};
use crate::gui::state;
use crate::model::{Body, FormKind, RawLang, find_node_mut};

impl App {
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
