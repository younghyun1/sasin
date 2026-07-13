//! Picker-choice enums (mirrors of model enums without payloads) and their labels.

use std::fmt;

/// Body-mode picker choice (mirrors [`crate::model::Body`] without payloads).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyModeChoice {
    None,
    Raw,
    UrlEncoded,
    FormData,
    Binary,
    GraphQl,
}

/// Auth-type picker choice (mirrors [`crate::model::Auth`] without payloads).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthChoice {
    None,
    Inherit,
    Basic,
    Bearer,
    ApiKey,
    OAuth2,
}

impl BodyModeChoice {
    pub const fn all() -> &'static [BodyModeChoice] {
        &[
            BodyModeChoice::None,
            BodyModeChoice::Raw,
            BodyModeChoice::UrlEncoded,
            BodyModeChoice::FormData,
            BodyModeChoice::Binary,
            BodyModeChoice::GraphQl,
        ]
    }

    const fn label(self) -> &'static str {
        match self {
            BodyModeChoice::None => "None",
            BodyModeChoice::Raw => "Raw",
            BodyModeChoice::UrlEncoded => "URL-encoded",
            BodyModeChoice::FormData => "Form-data",
            BodyModeChoice::Binary => "Binary",
            BodyModeChoice::GraphQl => "GraphQL",
        }
    }
}

impl fmt::Display for BodyModeChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl AuthChoice {
    pub const fn all() -> &'static [AuthChoice] {
        &[
            AuthChoice::Inherit,
            AuthChoice::None,
            AuthChoice::Basic,
            AuthChoice::Bearer,
            AuthChoice::ApiKey,
            AuthChoice::OAuth2,
        ]
    }

    const fn label(self) -> &'static str {
        match self {
            AuthChoice::None => "No Auth",
            AuthChoice::Inherit => "Inherit",
            AuthChoice::Basic => "Basic",
            AuthChoice::Bearer => "Bearer",
            AuthChoice::ApiKey => "API Key",
            AuthChoice::OAuth2 => "OAuth2",
        }
    }
}

impl fmt::Display for AuthChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}
