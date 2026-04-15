use std::sync::Arc;

use taffy::prelude::*;

use crate::ui::canvas::{Color, DrawContext, UiRect};
use crate::ui::component::{Component, ComponentUpdater};
use crate::ui::element::AnyElement;

use super::view::{Border, Direction, EdgeInsets, SizeValue};

// ─── ButtonProps ─────────────────────────────────────────────────────────────

pub struct ButtonProps {
    /// Обработчик нажатия. Arc позволяет клонировать в EventDispatcher.
    pub on_click: Option<Arc<dyn Fn() + Send + Sync>>,
    pub background: Option<Color>,
    pub pressed_background: Option<Color>,
    pub border: Option<Border>,
    // Layout (те же поля что у ViewProps)
    pub direction: Direction,
    pub gap: i32,
    pub padding: EdgeInsets,
    pub width: SizeValue,
    pub height: SizeValue,
    pub flex_grow: f32,
    pub align_items: Option<AlignItems>,
    pub justify_content: Option<JustifyContent>,
    pub children: Vec<AnyElement>,
}

impl Default for ButtonProps {
    fn default() -> Self {
        Self {
            on_click: None,
            background: None,
            pressed_background: None,
            border: None,
            direction: Direction::default(),
            gap: 0,
            padding: EdgeInsets::default(),
            width: SizeValue::default(),
            height: SizeValue::default(),
            flex_grow: 0.0,
            align_items: None,
            justify_content: None,
            children: Vec::new(),
        }
    }
}

// ─── Button ──────────────────────────────────────────────────────────────────

pub struct Button {
    background: Option<Color>,
    border: Option<Border>,
}

impl Component for Button {
    type Props = ButtonProps;

    fn new(props: &ButtonProps) -> Self {
        Self {
            background: props.background,
            border: props.border,
        }
    }

    fn update(&mut self, props: &mut ButtonProps, updater: &mut ComponentUpdater) {
        self.background = props.background;
        self.border = props.border;

        updater.set_layout_style(Style {
            display: Display::Flex,
            flex_direction: match props.direction {
                Direction::Row => FlexDirection::Row,
                Direction::Column => FlexDirection::Column,
            },
            gap: Size {
                width: LengthPercentage::Length(props.gap as f32),
                height: LengthPercentage::Length(props.gap as f32),
            },
            padding: Rect {
                left: LengthPercentage::Length(props.padding.left as f32),
                right: LengthPercentage::Length(props.padding.right as f32),
                top: LengthPercentage::Length(props.padding.top as f32),
                bottom: LengthPercentage::Length(props.padding.bottom as f32),
            },
            size: Size {
                width: props.width.to_taffy(),
                height: props.height.to_taffy(),
            },
            flex_grow: props.flex_grow,
            align_items: props.align_items,
            justify_content: props.justify_content,
            ..Style::default()
        });

        updater.set_children(std::mem::take(&mut props.children));

        // Регистрируем click handler — Arc клонируется, оригинал остаётся в props
        if let Some(ref handler) = props.on_click {
            updater.register_click_handler(Arc::clone(handler));
        }
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
