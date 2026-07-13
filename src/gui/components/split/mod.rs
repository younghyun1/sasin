//! Minimal resizable split pane for iced 0.14 (advanced widget API), with a
//! theme-styled divider. See [`widget::Split`].

mod geometry;
mod style;
mod widget;

pub use style::{Catalog, DividerStatus, DividerStyle};
pub use widget::{Split, SplitAxis};
