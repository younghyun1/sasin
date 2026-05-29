//! Reusable GUI components.
//!
//! `tree`, `tabs`, and `editor` are free-function view modules; the rest are builder types.

pub mod editor;
pub mod response_view;
pub mod section;
pub mod split;
pub mod tabs;
pub mod tree;

pub use response_view::ResponseView;
pub use section::Section;
pub use split::{Split, SplitAxis};
