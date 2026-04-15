use taffy::prelude::*;

use crate::ui::canvas::{Color, DrawContext, FontSize, TextStyle, UiRect};
use crate::ui::component::{Component, ComponentUpdater};

// ─── TextAlign ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

// ─── TextProps ───────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct TextProps {
    pub content: String,
    pub font_size: FontSize,
    pub color: Color,
    pub align: TextAlign,
}

// ─── Text ────────────────────────────────────────────────────────────────────

pub struct Text {
    content: String,
    style: TextStyle,
    align: TextAlign,
}

impl Component for Text {
    type Props = TextProps;

    fn new(props: &TextProps) -> Self {
        Self {
            content: props.content.clone(),
            style: TextStyle { font_size: props.font_size, color: props.color },
            align: props.align,
        }
    }

    fn update(&mut self, props: &mut TextProps, updater: &mut ComponentUpdater) {
        self.content = std::mem::take(&mut props.content);
        self.style = TextStyle { font_size: props.font_size, color: props.color };
        self.align = props.align;

        // Text не имеет детей — только measure_fn для расчёта размера
        updater.set_layout_style(Style {
            size: Size { width: Dimension::Auto, height: Dimension::Auto },
            ..Style::default()
        });

        let content = self.content.clone();
        let style = self.style;
        updater.set_measure_fn(move |_avail_w| {
            let (w, h) = DrawContext::measure_text(&content, style);
            (w as f32, h as f32)
        });
    }

    fn draw(&self, rect: UiRect, ctx: &mut DrawContext) {
        let (text_w, _) = DrawContext::measure_text(&self.content, self.style);
        let x = match self.align {
            TextAlign::Left => rect.x,
            TextAlign::Center => rect.x + (rect.width - text_w) / 2,
            TextAlign::Right => rect.x + rect.width - text_w,
        };
        ctx.draw_text(&self.content, x, rect.y, self.style);
    }
}
