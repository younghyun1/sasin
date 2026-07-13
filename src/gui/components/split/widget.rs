//! The `iced::advanced::Widget` implementation for [`Split`].
//!
//! Key points (iced 0.14): interaction is handled in `update` (not `on_event`); layout uses
//! public `layout::Node` accessors. `split_px` is the size in pixels of the *first* pane along
//! the split axis; dragging the divider publishes `on_drag(new_split_px)`.

use iced::advanced::layout;
use iced::advanced::mouse;
use iced::advanced::renderer;
use iced::advanced::widget::Tree;
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector};

use super::geometry::{clamp_split, divider_draw_rect, divider_hit_rect};
use super::style::{Catalog, DividerStatus};

/// Split axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitAxis {
    /// Left / right split.
    Horizontal,
    /// Top / bottom split.
    Vertical,
}

/// A two-pane split view with a draggable, theme-styled divider.
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
        // `Element` is not `Clone`, so create two independent placeholder elements.
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

    pub fn on_drag<F>(mut self, f: F) -> Self
    where
        F: Fn(f32) -> Message + 'a,
    {
        self.on_drag = Some(Box::new(f));
        self
    }

    fn clamped_split(&self, size: Size) -> f32 {
        clamp_split(
            self.axis,
            self.split_px,
            size,
            self.min_first_px,
            self.min_second_px,
        )
    }
}

#[derive(Debug, Default)]
struct SplitState {
    dragging: bool,
    /// Whether the cursor is over the divider hit rect (drives the hover highlight).
    hovered: bool,
    /// Pointer offset relative to split position at drag start, to avoid jump.
    drag_offset: f32,
}

impl SplitState {
    fn status(&self) -> DividerStatus {
        if self.dragging {
            DividerStatus::Dragging
        } else if self.hovered {
            DividerStatus::Hovered
        } else {
            DividerStatus::Idle
        }
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Split<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: Catalog + 'a,
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
        let split_px = self.clamped_split(size);

        // Layout the two children with fixed constraints derived from the split.
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

        let mut first_node = self.first.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, first_size),
        );

        let mut second_node = self.second.as_widget_mut().layout(
            &mut tree.children[1],
            renderer,
            &layout::Limits::new(Size::ZERO, second_size),
        );

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
            let (Some(first_layout), Some(second_layout)) = (children.next(), children.next())
            else {
                return;
            };

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

        // Handle divider hover + drag.
        let Some(on_drag) = &self.on_drag else {
            return;
        };

        let state = tree.state.downcast_mut::<SplitState>();
        let bounds = layout.bounds();
        let split_px = self.clamped_split(bounds.size());
        let hit_rect = divider_hit_rect(self.axis, bounds, split_px, self.divider_hit);

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if cursor.is_over(hit_rect) {
                    state.dragging = true;
                    state.drag_offset = match cursor.position_in(bounds) {
                        Some(pos) => match self.axis {
                            SplitAxis::Horizontal => pos.x - split_px,
                            SplitAxis::Vertical => pos.y - split_px,
                        },
                        None => 0.0,
                    };
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.dragging {
                    state.dragging = false;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let hovered = cursor.is_over(hit_rect);
                if hovered != state.hovered {
                    state.hovered = hovered;
                    shell.request_redraw();
                }
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
        if self.on_drag.is_some() {
            let state = tree.state.downcast_ref::<SplitState>();
            let bounds = layout.bounds();
            let split_px = self.clamped_split(bounds.size());
            let hit_rect = divider_hit_rect(self.axis, bounds, split_px, self.divider_hit);

            // Keep the resize cursor while dragging, even if the pointer outruns the hit rect.
            if state.dragging || cursor.is_over(hit_rect) {
                return match self.axis {
                    SplitAxis::Horizontal => mouse::Interaction::ResizingHorizontally,
                    SplitAxis::Vertical => mouse::Interaction::ResizingVertically,
                };
            }
        }

        // Otherwise, take the "max" interaction of the children.
        let mut children = layout.children();
        let (Some(first_layout), Some(second_layout)) = (children.next(), children.next()) else {
            return mouse::Interaction::default();
        };

        let first = self.first.as_widget().mouse_interaction(
            &tree.children[0],
            first_layout,
            cursor,
            viewport,
            renderer,
        );

        let second = self.second.as_widget().mouse_interaction(
            &tree.children[1],
            second_layout,
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
        let (Some(first_layout), Some(second_layout)) = (children.next(), children.next()) else {
            return;
        };

        self.first.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            first_layout,
            cursor,
            viewport,
        );

        self.second.as_widget().draw(
            &tree.children[1],
            renderer,
            theme,
            style,
            second_layout,
            cursor,
            viewport,
        );

        // Divider strip, styled by the theme according to interaction state.
        let state = tree.state.downcast_ref::<SplitState>();
        let divider_style = theme.divider(state.status());
        let bounds = layout.bounds();
        let split_px = self.clamped_split(bounds.size());
        let divider = divider_draw_rect(
            self.axis,
            bounds,
            split_px,
            self.divider_thickness,
            divider_style.width,
        );

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
            divider_style.color,
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
        let (Some(first_layout), Some(second_layout)) = (children.next(), children.next()) else {
            return;
        };

        self.first.as_widget_mut().operate(
            &mut tree.children[0],
            first_layout,
            renderer,
            operation,
        );

        self.second.as_widget_mut().operate(
            &mut tree.children[1],
            second_layout,
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
        let (Some(first_layout), Some(second_layout)) = (children.next(), children.next()) else {
            return None;
        };

        // Two disjoint mutable borrows of `tree.children`.
        let (first_tree, second_tree) = tree.children.split_at_mut(1);
        let first_tree = &mut first_tree[0];
        let second_tree = &mut second_tree[0];

        let first = self.first.as_widget_mut().overlay(
            first_tree,
            first_layout,
            renderer,
            viewport,
            translation,
        );

        let second = self.second.as_widget_mut().overlay(
            second_tree,
            second_layout,
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
    Theme: Catalog + 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(value: Split<'a, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}
