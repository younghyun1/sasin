//! Reusable GUI components.
//!
//! Keep Iced view code small by moving repeated UI patterns into focused modules.

pub mod history_list;
pub mod request_editor;
pub mod response_view;
pub mod section;
pub mod split;
pub mod template_list;

// Re-export commonly used components (optional convenience).
pub use history_list::HistoryList;
pub use request_editor::RequestEditor;
pub use response_view::ResponseView;
pub use section::Section;
pub use split::{Split, SplitAxis};
pub use template_list::TemplateList;
