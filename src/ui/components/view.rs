use taffy::prelude::*;

// Явный реэкспорт — `use prelude::*` делает импорт приватным, поэтому явно
pub use taffy::style::AlignItems;
pub use taffy::style::JustifyContent;

use crate::ui::canvas::{Color, DrawContext, UiRect};
use crate::ui::component::{Component, ComponentUpdater};
use crate::ui::element::AnyElement;

// ─── Direction ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Direction {
    #[default]
    Row,
    Column,
}

impl Direction {
    fn to_taffy(self) -> FlexDirection {
        match self {
            Direction::Row => FlexDirection::Row,
            Direction::Column => FlexDirection::Column,
        }
    }
}

// ─── SizeValue ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SizeValue {
    #[default]
    Auto,
    Fixed(i32),
    Percent(f32),
}

impl SizeValue {
    pub(crate) fn to_taffy(self) -> Dimension {
        match self {
            SizeValue::Auto => Dimension::Auto,
            SizeValue::Fixed(px) => Dimension::Length(px as f32),
            SizeValue::Percent(p) => Dimension::Percent(p / 100.0),
        }
    }
}

// ─── EdgeInsets ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default)]
pub struct EdgeInsets {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

impl EdgeInsets {
    pub fn all(v: i32) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }
    pub fn horizontal(h: i32) -> Self {
        Self {
            top: 0,
            right: h,
            bottom: 0,
            left: h,
        }
    }
    pub fn vertical(v: i32) -> Self {
        Self {
            top: v,
            right: 0,
            bottom: v,
            left: 0,
        }
    }
    pub fn lrtb(left: i32, right: i32, top: i32, bottom: i32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    fn to_taffy(self) -> Rect<LengthPercentage> {
        Rect {
            left: LengthPercentage::Length(self.left as f32),
            right: LengthPercentage::Length(self.right as f32),
            top: LengthPercentage::Length(self.top as f32),
            bottom: LengthPercentage::Length(self.bottom as f32),
        }
    }
}

// ─── Border ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct Border {
    pub color: Color,
    pub thickness: u32,
}

// ─── ViewProps ───────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct ViewProps {
    pub direction: Direction,
    pub gap: i32,
    pub padding: EdgeInsets,
    pub background: Option<Color>,
    pub border: Option<Border>,
    pub width: SizeValue,
    pub height: SizeValue,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub align_items: Option<AlignItems>,
    pub justify_content: Option<JustifyContent>,
    pub children: Vec<AnyElement>,
}

fn build_style(
    direction: Direction,
    gap: i32,
    padding: EdgeInsets,
    width: SizeValue,
    height: SizeValue,
    flex_grow: f32,
    flex_shrink: f32,
    align_items: Option<AlignItems>,
    justify_content: Option<JustifyContent>,
) -> Style {
    Style {
        display: Display::Flex,
        flex_direction: direction.to_taffy(),
        gap: Size {
            width: LengthPercentage::Length(gap as f32),
            height: LengthPercentage::Length(gap as f32),
        },
        padding: padding.to_taffy(),
        size: Size {
            width: width.to_taffy(),
            height: height.to_taffy(),
        },
        flex_grow,
        flex_shrink,
        align_items,
        justify_content,
        ..Style::default()
    }
}

// ─── View ────────────────────────────────────────────────────────────────────

pub struct View {
    background: Option<Color>,
    border: Option<Border>,
}

impl Component for View {
    type Props = ViewProps;

    fn new(props: &ViewProps) -> Self {
        Self {
            background: props.background,
            border: props.border,
        }
    }

    fn update(&mut self, props: &mut ViewProps, updater: &mut ComponentUpdater) {
        self.background = props.background;
        self.border = props.border;

        updater.set_layout_style(build_style(
            props.direction,
            props.gap,
            props.padding,
            props.width,
            props.height,
            props.flex_grow,
            props.flex_shrink,
            props.align_items,
            props.justify_content,
        ));

        // mem::take забирает Vec из props без клонирования
        updater.set_children(std::mem::take(&mut props.children));
    }

    fn draw(&self, rect: UiRect, ctx: &mut DrawContext) {
        if let Some(bg) = self.background {
            ctx.fill_rect(rect, bg);
        }
        if let Some(b) = self.border {
            ctx.stroke_rect(rect, b.color, b.thickness);
        }
    }
}
