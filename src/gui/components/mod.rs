//! Reusable GUI components.
//!
//! Keep Iced view code small by moving repeated UI patterns into focused modules.

pub mod section;
pub mod template_list;
pub mod history_list;
pub mod request_editor;
pub mod response_view;

// Re-export commonly used components (optional convenience).
pub use section::Section;
pub use template_list::TemplateList;
pub use history_list::HistoryList;
pub use request_editor::RequestEditor;
pub use response_view::ResponseView;
