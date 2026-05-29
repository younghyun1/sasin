//! Apply a resolved [`Auth`] to a reqwest request builder.
//!
//! Inheritance is resolved upstream ([`crate::model::resolve_auth`]); this receives the effective
//! auth. Invalid API-key header names surface as a reqwest build error at send time, not a panic.

use reqwest::RequestBuilder;

use crate::model::{ApiKeyLoc, Auth};

/// Apply `auth` to `rb`, returning the modified builder.
pub fn apply_auth(rb: RequestBuilder, auth: &Auth) -> RequestBuilder {
    match auth {
        Auth::None | Auth::Inherit => rb,
        Auth::Basic { user, pass } => rb.basic_auth(user, Some(pass)),
        Auth::Bearer { token } | Auth::OAuth2 { token } => rb.bearer_auth(token),
        Auth::ApiKey { key, value, add_to } => match add_to {
            ApiKeyLoc::Header => rb.header(key.as_str(), value.as_str()),
            ApiKeyLoc::Query => rb.query(&[(key.as_str(), value.as_str())]),
        },
    }
}
