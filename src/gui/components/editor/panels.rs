//! The five editor sub-panels. Structured fields are read from the node; edits emit messages that
//! mutate the node directly (body text is the exception, buffered in the tab).

use iced::alignment::Vertical;
use iced::widget::{
    Space, button, checkbox, column, pick_list, row, text, text_editor, text_input,
};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::components::kv_table;
use crate::gui::messages::{
    AuthChoice, AuthFieldKind, BodyModeChoice, EditorPanel, KvOp, KvTarget, SettingFlag,
};
use crate::gui::state::Tab;
use crate::gui::theme::fonts;
use crate::model::{ApiKeyLoc, Auth, Body, FormKind, FormPart, HttpRequest, RawLang};

/// Editor highlight theme, synced with the app theme by the caller.
type HlTheme = iced::highlighter::Theme;

/// Map a raw-body language to the syntect token the highlighter resolves.
fn syntax_token(lang: RawLang) -> &'static str {
    match lang {
        RawLang::Json => "json",
        RawLang::Xml => "xml",
        RawLang::Html => "html",
        RawLang::Javascript => "js",
        RawLang::Text => "txt",
    }
}

/// Render the active panel.
pub fn view<'a>(req: &'a HttpRequest, tab: &'a Tab, hl: HlTheme) -> Element<'a, Message> {
    match tab.panel {
        EditorPanel::Params => kv_table::view(KvTarget::Params, &req.params, "key", "value"),
        EditorPanel::Headers => kv_table::view(KvTarget::Headers, &req.headers, "Header", "Value"),
        EditorPanel::Auth => auth_panel(&req.auth),
        EditorPanel::Body => body_panel(req, tab, hl),
        EditorPanel::Scripts => scripts_panel(tab, hl),
        EditorPanel::Settings => settings_panel(req, tab),
    }
}

fn scripts_panel(tab: &Tab, hl: HlTheme) -> Element<'_, Message> {
    column![
        text("Pre-request script").size(13),
        text_editor(&tab.pre_script)
            .placeholder("pm.environment.set('ts', Date.now())")
            .on_action(Message::PreScriptAction)
            .height(Length::Fixed(150.0))
            .font(fonts::MONO)
            .highlight("js", hl),
        text("Test script").size(13),
        text_editor(&tab.test_script)
            .placeholder("pm.test('status ok', () => pm.response.to.have.status(200))")
            .on_action(Message::TestScriptAction)
            .height(Length::Fixed(150.0))
            .font(fonts::MONO)
            .highlight("js", hl),
    ]
    .spacing(6)
    .width(Length::Fill)
    .into()
}

fn labeled_input<'a>(label: &'a str, value: &'a str, kind: AuthFieldKind) -> Element<'a, Message> {
    row![
        text(label).size(13).width(Length::Fixed(90.0)),
        text_input("", value)
            .on_input(move |s| Message::AuthField(kind, s))
            .padding(6)
            .size(13)
            .width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Vertical::Center)
    .into()
}

fn auth_panel(auth: &Auth) -> Element<'_, Message> {
    let selector = pick_list(
        AuthChoice::all(),
        Some(auth_choice(auth)),
        Message::AuthChanged,
    )
    .padding(8);
    let fields: Element<'_, Message> = match auth {
        Auth::Basic { user, pass } => column![
            labeled_input("Username", user, AuthFieldKind::User),
            labeled_input("Password", pass, AuthFieldKind::Pass),
        ]
        .spacing(6)
        .into(),
        Auth::Bearer { token } | Auth::OAuth2 { token } => {
            labeled_input("Token", token, AuthFieldKind::Token)
        }
        Auth::ApiKey { key, value, add_to } => column![
            labeled_input("Key", key, AuthFieldKind::ApiKeyName),
            labeled_input("Value", value, AuthFieldKind::ApiKeyValue),
            checkbox(matches!(add_to, ApiKeyLoc::Header))
                .label("Add to header (otherwise query string)")
                .on_toggle(Message::AuthApiKeyInHeader),
        ]
        .spacing(6)
        .into(),
        Auth::None => text("No authentication.").size(13).into(),
        Auth::Inherit => text("Inherits auth from the enclosing folder.")
            .size(13)
            .into(),
    };
    column![selector, Space::new().height(Length::Fixed(8.0)), fields]
        .spacing(8)
        .width(Length::Fill)
        .into()
}

fn body_panel<'a>(req: &'a HttpRequest, tab: &'a Tab, hl: HlTheme) -> Element<'a, Message> {
    let selector = pick_list(
        BodyModeChoice::all(),
        Some(body_choice(&req.body)),
        Message::BodyModeChanged,
    )
    .padding(8);
    let content: Element<'a, Message> = match &req.body {
        Body::None => text("No body.").size(13).into(),
        Body::Raw { language, .. } => column![
            pick_list(RawLang::all(), Some(*language), Message::RawLangChanged).padding(6),
            text_editor(&tab.body)
                .placeholder("Raw body…")
                .on_action(Message::BodyAction)
                .height(Length::Fixed(240.0))
                .font(fonts::MONO)
                .highlight(syntax_token(*language), hl),
        ]
        .spacing(6)
        .into(),
        Body::UrlEncoded { fields } => kv_table::view(KvTarget::UrlEncoded, fields, "key", "value"),
        Body::FormData { parts } => formdata_table(parts),
        Body::Binary { file } => row![
            text("File").size(13).width(Length::Fixed(60.0)),
            text_input("relative/path.bin", file)
                .on_input(Message::BinaryFileChanged)
                .padding(6)
                .size(13)
                .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Vertical::Center)
        .into(),
        Body::GraphQl { .. } => column![
            text("Query").size(13),
            text_editor(&tab.body)
                .placeholder("query { … }")
                .on_action(Message::BodyAction)
                .height(Length::Fixed(160.0))
                .font(fonts::MONO)
                .highlight("graphql", hl),
            text("Variables (JSON)").size(13),
            text_editor(&tab.gql_vars)
                .placeholder("{}")
                .on_action(Message::GqlVarsAction)
                .height(Length::Fixed(100.0))
                .font(fonts::MONO)
                .highlight("json", hl),
        ]
        .spacing(6)
        .into(),
    };
    column![selector, Space::new().height(Length::Fixed(8.0)), content]
        .spacing(8)
        .width(Length::Fill)
        .into()
}

fn formdata_table(parts: &[FormPart]) -> Element<'_, Message> {
    let mut rows: Vec<Element<'_, Message>> = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        let is_file = matches!(part.kind, FormKind::File);
        let enabled = checkbox(part.enabled)
            .on_toggle(move |b| Message::Kv(KvTarget::FormData, KvOp::Toggle(i, b)));
        let key = text_input("field", &part.key)
            .on_input(move |s| Message::Kv(KvTarget::FormData, KvOp::Key(i, s)))
            .padding(6)
            .size(13)
            .width(Length::FillPortion(2));
        let kind = button(text(if is_file { "File" } else { "Text" }).size(12))
            .padding(6)
            .on_press(Message::FormPartFile(i, !is_file));
        let value: Element<'_, Message> = if is_file {
            text_input("path", &part.src)
                .on_input(move |s| Message::FormPartSrc(i, s))
                .padding(6)
                .size(13)
                .width(Length::FillPortion(3))
                .into()
        } else {
            text_input("value", &part.value)
                .on_input(move |s| Message::Kv(KvTarget::FormData, KvOp::Value(i, s)))
                .padding(6)
                .size(13)
                .width(Length::FillPortion(3))
                .into()
        };
        let delete = button(text("✕").size(12))
            .padding(6)
            .on_press(Message::Kv(KvTarget::FormData, KvOp::Remove(i)));
        rows.push(row![enabled, key, kind, value, delete].spacing(6).into());
    }
    rows.push(
        button(text("+ Add").size(12))
            .padding(6)
            .on_press(Message::Kv(KvTarget::FormData, KvOp::Add))
            .into(),
    );
    column(rows).spacing(6).width(Length::Fill).into()
}

fn settings_panel<'a>(req: &'a HttpRequest, tab: &'a Tab) -> Element<'a, Message> {
    let s = &req.settings;
    column![
        row![
            text("Timeout (ms)").size(13).width(Length::Fixed(140.0)),
            text_input("30000", &tab.timeout_text)
                .on_input(Message::SettingTimeout)
                .padding(6)
                .size(13)
                .width(Length::Fixed(140.0)),
        ]
        .spacing(8)
        .align_y(Vertical::Center),
        checkbox(s.follow_redirects)
            .label("Follow redirects")
            .on_toggle(|b| Message::SettingFlagChanged(SettingFlag::FollowRedirects, b)),
        checkbox(s.verify_tls)
            .label("Verify TLS certificates")
            .on_toggle(|b| Message::SettingFlagChanged(SettingFlag::VerifyTls, b)),
        checkbox(s.use_cookie_jar)
            .label("Use cookie jar")
            .on_toggle(|b| Message::SettingFlagChanged(SettingFlag::CookieJar, b)),
        row![
            text("Proxy").size(13).width(Length::Fixed(140.0)),
            text_input("http://host:port", s.proxy.as_deref().unwrap_or(""))
                .on_input(Message::SettingProxy)
                .padding(6)
                .size(13)
                .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Vertical::Center),
    ]
    .spacing(10)
    .width(Length::Fill)
    .into()
}

fn auth_choice(auth: &Auth) -> AuthChoice {
    match auth {
        Auth::None => AuthChoice::None,
        Auth::Inherit => AuthChoice::Inherit,
        Auth::Basic { .. } => AuthChoice::Basic,
        Auth::Bearer { .. } => AuthChoice::Bearer,
        Auth::ApiKey { .. } => AuthChoice::ApiKey,
        Auth::OAuth2 { .. } => AuthChoice::OAuth2,
    }
}

fn body_choice(body: &Body) -> BodyModeChoice {
    match body {
        Body::None => BodyModeChoice::None,
        Body::Raw { .. } => BodyModeChoice::Raw,
        Body::UrlEncoded { .. } => BodyModeChoice::UrlEncoded,
        Body::FormData { .. } => BodyModeChoice::FormData,
        Body::Binary { .. } => BodyModeChoice::Binary,
        Body::GraphQl { .. } => BodyModeChoice::GraphQl,
    }
}
