use std::sync::Arc;

use taffy::prelude::*;

use crate::image::BmpImage;
use crate::ui::canvas::{DrawContext, UiRect};
use crate::ui::component::{Component, ComponentUpdater};

// ─── ImageProps ──────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct ImageProps {
    /// Arc позволяет шарить изображение между рендерами без копирования.
    pub image: Option<Arc<BmpImage>>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

// ─── Image ───────────────────────────────────────────────────────────────────

pub struct Image {
    image: Option<Arc<BmpImage>>,
}

impl Component for Image {
    type Props = ImageProps;

    fn new(props: &ImageProps) -> Self {
        Self {
            image: props.image.clone(),
        }
    }

    fn update(&mut self, props: &mut ImageProps, updater: &mut ComponentUpdater) {
        self.image = props.image.clone();

        let (w, h) = match &props.image {
            Some(img) => (
                props.width.unwrap_or(img.width as i32),
                props.height.unwrap_or(img.height as i32),
            ),
            None => (props.width.unwrap_or(0), props.height.unwrap_or(0)),
        };

        updater.set_layout_style(Style {
            size: Size {
                width: Dimension::Length(w as f32),
                height: Dimension::Length(h as f32),
            },
            ..Style::default()
        });
    }

    fn draw(&self, rect: UiRect, ctx: &mut DrawContext) {
        if let Some(ref img) = self.image {
            ctx.draw_image(img, rect.x, rect.y);
        }
    }
}
