pub mod canvas;
pub mod component;
pub mod components;
pub mod element;
pub mod events;
pub mod layout;

pub use canvas::{Color, DrawContext, FontSize, TextStyle, UiRect};
pub use components::view::{AlignItems, Border, Direction, EdgeInsets, JustifyContent, SizeValue};
pub use components::{
    Button, ButtonProps, Image, ImageProps, Text, TextAlign, TextProps, View, ViewProps,
};
pub use events::UiEvent;

use crate::display::EmbeddedDisplay;
use crate::ui::component::InstantiatedComponent;
use crate::ui::element::AnyElement;
use crate::ui::events::EventDispatcher;
use crate::ui::layout::LayoutEngine;

// ─── UiApp ───────────────────────────────────────────────────────────────────

pub struct UiApp {
    engine: LayoutEngine,
    dispatcher: EventDispatcher,
    display: EmbeddedDisplay,
    root: Option<InstantiatedComponent>,
}

impl UiApp {
    pub fn new(display: EmbeddedDisplay) -> Self {
        Self {
            engine: LayoutEngine::new(),
            dispatcher: EventDispatcher::new(),
            display,
            root: None,
        }
    }

    /// Полный цикл рендера: reconcile → layout → draw → flush.
    pub fn render(&mut self, mut element: AnyElement) {
        // ── PHASE 1: UPDATE ───────────────────────────────────────────────────
        if self.root.is_none() {
            let mut root = InstantiatedComponent::create(&element, &mut self.engine);
            root.update(&mut element, &mut self.engine);
            self.root = Some(root);
        } else {
            self.root.as_mut().unwrap().update(&mut element, &mut self.engine);
        }

        let root_node = self.root.as_ref().unwrap().node_id;

        // ── PHASE 2: LAYOUT ───────────────────────────────────────────────────
        self.engine.compute(
            root_node,
            self.display.width() as f32,
            self.display.height() as f32,
        );

        // ── PHASE 3: DRAW + COLLECT HITS ──────────────────────────────────────
        self.display.clear(embedded_graphics::pixelcolor::Gray4::new(15));
        self.dispatcher.clear();

        {
            let mut ctx = DrawContext::new(&mut self.display);
            if let Some(ref root) = self.root {
                root.draw_and_collect(&self.engine, 0, 0, &mut ctx, &mut self.dispatcher);
            }
        }

        // ── PHASE 4: FLUSH ────────────────────────────────────────────────────
        self.display.flush().ok();
    }

    /// Передать touch-событие. Возвращает true если обработано.
    pub fn handle_event(&mut self, event: UiEvent) -> bool {
        self.dispatcher.dispatch(&event)
    }
}
