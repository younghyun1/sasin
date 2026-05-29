#![allow(clippy::needless_return)]
//! Minimal resizable split pane for Iced 0.14 (advanced widget API).
//!
//! This module provides a small, dependency-free split pane widget with a draggable divider.
//!
//! ## Key points (Iced 0.14)
//! - Custom widgets are implemented via `iced::advanced::Widget`.
//! - Interaction is handled in `update` (not `on_event`).
//! - Layout uses public `layout::Node` and `Node::children()` accessors.
//!
//! ## Semantics
//! - `split_px` is the size (in pixels) of the *first* pane along the split axis.
//! - The divider consumes `divider_thickness` pixels.
//! - Dragging the divider publishes a `Message` produced by `on_drag(new_split_px)`.
//!
//! ## Example
//! ```ignore
//! use crate::gui::components::split::{Split, SplitAxis};
//! Split::new(SplitAxis::Horizontal)
//!     .first(sidebar)
//!     .second(main)
//!     .split_px(self.sidebar_width_px)
//!     .on_drag(|px| Message::SplitDragged(SplitId::Sidebar, px))
//!     .into()
//! ```

use iced::advanced::layout;
use iced::advanced::mouse;
use iced::advanced::renderer;
use iced::advanced::widget::Tree;
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector};

/// Split axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitAxis {
    /// Left / right split.
    Horizontal,
    /// Top / bottom split.
    Vertical,
}

/// A two-pane split view with a draggable divider.
///
/// Notes:
/// - `Element` is not `Clone`/`Debug`, so we do not derive these traits.
pub struct Split<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    axis: SplitAxis,

    first: Element<'a, Message, Theme, Renderer>,
    second: Element<'a, Message, Theme, Renderer>,

    // sizing
    split_px: f32,
    min_first_px: f32,
    min_second_px: f32,
    divider_thickness: f32,
    divider_hit: f32,

    // outer
    width: Length,
    height: Length,

    on_drag: Option<Box<dyn Fn(f32) -> Message + 'a>>,
}

impl<'a, Message, Theme, Renderer> Split<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    pub fn new(axis: SplitAxis) -> Self {
        // A Split must be provided both children via `first` and `second`.
        // `Element` is not `Clone`, so we create two independent placeholder elements.
        let first: Element<'a, Message, Theme, Renderer> = Element::new(iced::widget::Space::new());
        let second: Element<'a, Message, Theme, Renderer> =
            Element::new(iced::widget::Space::new());

        Self {
            axis,
            first,
            second,
            split_px: 240.0,
            min_first_px: 180.0,
            min_second_px: 220.0,
            divider_thickness: 1.0,
            divider_hit: 10.0,
            width: Length::Fill,
            height: Length::Fill,
            on_drag: None,
        }
    }

    pub fn first(mut self, element: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        self.first = element.into();
        self
    }

    pub fn second(mut self, element: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        self.second = element.into();
        self
    }

    pub fn split_px(mut self, px: f32) -> Self {
        self.split_px = px;
        self
    }

    pub fn min_first_px(mut self, px: f32) -> Self {
        self.min_first_px = px.max(0.0);
        self
    }

    pub fn min_second_px(mut self, px: f32) -> Self {
        self.min_second_px = px.max(0.0);
        self
    }

    pub fn divider_thickness(mut self, px: f32) -> Self {
        self.divider_thickness = px.max(0.0);
        self
    }

    pub fn divider_hit(mut self, px: f32) -> Self {
        self.divider_hit = px.max(self.divider_thickness).max(1.0);
        self
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn on_drag<F>(mut self, f: F) -> Self
    where
        F: Fn(f32) -> Message + 'a,
    {
        self.on_drag = Some(Box::new(f));
        self
    }
}

#[derive(Debug, Default)]
struct SplitState {
    dragging: bool,
    /// Pointer offset relative to split position at drag start, to avoid jump.
    drag_offset: f32,
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Split<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        iced::advanced::widget::tree::Tag::of::<SplitState>()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(SplitState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.first), Tree::new(&self.second)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.first, &self.second]);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);
        let size = limits.max();

        let divider = self.divider_thickness;
        let split_px = clamp_split(
            self.axis,
            self.split_px,
            size,
            self.min_first_px,
            self.min_second_px,
        );

        // Layout the two children with fixed constraints derived from split.
        let (first_size, second_size) = match self.axis {
            SplitAxis::Horizontal => {
                let w1 = split_px;
                let w2 = (size.width - divider - split_px).max(0.0);
                (Size::new(w1, size.height), Size::new(w2, size.height))
            }
            SplitAxis::Vertical => {
                let h1 = split_px;
                let h2 = (size.height - divider - split_px).max(0.0);
                (Size::new(size.width, h1), Size::new(size.width, h2))
            }
        };

        let first_node = self.first.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, first_size),
        );

        let second_node = self.second.as_widget_mut().layout(
            &mut tree.children[1],
            renderer,
            &layout::Limits::new(Size::ZERO, second_size),
        );

        // Position child nodes.
        let mut first_node = first_node;
        let mut second_node = second_node;

        match self.axis {
            SplitAxis::Horizontal => {
                first_node.move_to_mut(Point::new(0.0, 0.0));
                second_node.move_to_mut(Point::new(split_px + divider, 0.0));
            }
            SplitAxis::Vertical => {
                first_node.move_to_mut(Point::new(0.0, 0.0));
                second_node.move_to_mut(Point::new(0.0, split_px + divider));
            }
        }

        layout::Node::with_children(size, vec![first_node, second_node])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        // Forward to children first so they can react normally.
        {
            let mut children = layout.children();
            let first_layout = children.next().expect("Split must have first child layout");
            let second_layout = children
                .next()
                .expect("Split must have second child layout");

            self.first.as_widget_mut().update(
                &mut tree.children[0],
                event,
                first_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );

            self.second.as_widget_mut().update(
                &mut tree.children[1],
                event,
                second_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }

        // Handle divider drag.
        let Some(on_drag) = &self.on_drag else {
            return;
        };

        let state = tree.state.downcast_mut::<SplitState>();
        let bounds = layout.bounds();
        let split_px = clamp_split(
            self.axis,
            self.split_px,
            bounds.size(),
            self.min_first_px,
            self.min_second_px,
        );
        let hit_rect = divider_hit_rect(self.axis, bounds, split_px, self.divider_hit);

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if cursor.is_over(hit_rect) {
                    state.dragging = true;

                    if let Some(pos) = cursor.position_in(bounds) {
                        state.drag_offset = match self.axis {
                            SplitAxis::Horizontal => pos.x - split_px,
                            SplitAxis::Vertical => pos.y - split_px,
                        };
                    } else {
                        state.drag_offset = 0.0;
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.dragging = false;
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.dragging
                    && let Some(pos) = cursor.position_in(bounds)
                {
                    let raw = match self.axis {
                        SplitAxis::Horizontal => pos.x - state.drag_offset,
                        SplitAxis::Vertical => pos.y - state.drag_offset,
                    };

                    let clamped = clamp_split(
                        self.axis,
                        raw,
                        bounds.size(),
                        self.min_first_px,
                        self.min_second_px,
                    );

                    self.split_px = clamped;
                    shell.publish(on_drag(clamped));
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        // Divider hover interaction if resizable.
        if self.on_drag.is_some() {
            let bounds = layout.bounds();
            let split_px = clamp_split(
                self.axis,
                self.split_px,
                bounds.size(),
                self.min_first_px,
                self.min_second_px,
            );
            let hit_rect = divider_hit_rect(self.axis, bounds, split_px, self.divider_hit);

            if cursor.is_over(hit_rect) {
                return match self.axis {
                    SplitAxis::Horizontal => mouse::Interaction::ResizingHorizontally,
                    SplitAxis::Vertical => mouse::Interaction::ResizingVertically,
                };
            }
        }

        // Otherwise, take the "max" interaction of the children.
        let mut children = layout.children();

        let first = self.first.as_widget().mouse_interaction(
            &tree.children[0],
            children.next().unwrap(),
            cursor,
            viewport,
            renderer,
        );

        let second = self.second.as_widget().mouse_interaction(
            &tree.children[1],
            children.next().unwrap(),
            cursor,
            viewport,
            renderer,
        );

        if first > second { first } else { second }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let mut children = layout.children();

        self.first.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            children.next().unwrap(),
            cursor,
            viewport,
        );

        self.second.as_widget().draw(
            &tree.children[1],
            renderer,
            theme,
            style,
            children.next().unwrap(),
            cursor,
            viewport,
        );

        // Divider line.
        let bounds = layout.bounds();
        let split_px = clamp_split(
            self.axis,
            self.split_px,
            bounds.size(),
            self.min_first_px,
            self.min_second_px,
        );
        let divider = divider_line_rect(self.axis, bounds, split_px, self.divider_thickness);

        renderer.fill_quad(
            renderer::Quad {
                bounds: divider,
                border: iced::Border {
                    radius: 0.0.into(),
                    width: 0.0,
                    color: iced::Color::TRANSPARENT,
                },
                shadow: iced::Shadow::default(),
                snap: true,
            },
            iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08),
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        let mut children = layout.children();

        self.first.as_widget_mut().operate(
            &mut tree.children[0],
            children.next().unwrap(),
            renderer,
            operation,
        );

        self.second.as_widget_mut().operate(
            &mut tree.children[1],
            children.next().unwrap(),
            renderer,
            operation,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        let mut children = layout.children();

        // We need two disjoint mutable borrows of `tree.children`.
        let (first_tree, second_tree) = tree.children.split_at_mut(1);
        let first_tree = &mut first_tree[0];
        let second_tree = &mut second_tree[0];

        let first = self.first.as_widget_mut().overlay(
            first_tree,
            children.next().unwrap(),
            renderer,
            viewport,
            translation,
        );

        let second = self.second.as_widget_mut().overlay(
            second_tree,
            children.next().unwrap(),
            renderer,
            viewport,
            translation,
        );

        first.or(second)
    }
}

impl<'a, Message, Theme, Renderer> From<Split<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(value: Split<'a, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}

fn clamp_split(axis: SplitAxis, split_px: f32, size: Size, min_first: f32, min_second: f32) -> f32 {
    match axis {
        SplitAxis::Horizontal => {
            let max_first = (size.width - min_second).max(min_first);
            split_px.clamp(min_first, max_first)
        }
        SplitAxis::Vertical => {
            let max_first = (size.height - min_second).max(min_first);
            split_px.clamp(min_first, max_first)
        }
    }
}

fn divider_hit_rect(axis: SplitAxis, bounds: Rectangle, split_px: f32, hit: f32) -> Rectangle {
    match axis {
        SplitAxis::Horizontal => Rectangle {
            x: bounds.x + split_px - hit * 0.5,
            y: bounds.y,
            width: hit,
            height: bounds.height,
        },
        SplitAxis::Vertical => Rectangle {
            x: bounds.x,
            y: bounds.y + split_px - hit * 0.5,
            width: bounds.width,
            height: hit,
        },
    }
}

fn divider_line_rect(
    axis: SplitAxis,
    bounds: Rectangle,
    split_px: f32,
    thickness: f32,
) -> Rectangle {
    match axis {
        SplitAxis::Horizontal => Rectangle {
            x: bounds.x + split_px,
            y: bounds.y,
            width: thickness,
            height: bounds.height,
        },
        SplitAxis::Vertical => Rectangle {
            x: bounds.x,
            y: bounds.y + split_px,
            width: bounds.width,
            height: thickness,
        },
    }
}
