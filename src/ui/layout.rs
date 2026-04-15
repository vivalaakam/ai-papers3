use taffy::prelude::*;
use taffy::TaffyTree;

use crate::ui::canvas::UiRect;

// ─── NodeContext ──────────────────────────────────────────────────────────────

/// Пользовательские данные, хранящиеся в каждом узле taffy.
/// Нужны только для Text-подобных компонентов, у которых размер
/// зависит от содержимого.
#[derive(Default)]
pub struct NodeContext {
    /// f(available_width_px) → (width_px, height_px)
    pub measure_fn: Option<Box<dyn Fn(f32) -> (f32, f32) + Send>>,
}

// ─── LayoutEngine ─────────────────────────────────────────────────────────────

pub struct LayoutEngine {
    tree: TaffyTree<NodeContext>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self { tree: TaffyTree::new() }
    }

    /// Создать новый листовой узел с заданным стилем.
    pub fn new_leaf(&mut self, style: Style) -> NodeId {
        self.tree
            .new_leaf_with_context(style, NodeContext::default())
            .expect("taffy: new_leaf failed")
    }

    /// Обновить стиль существующего узла.
    pub fn set_style(&mut self, node: NodeId, style: Style) {
        self.tree.set_style(node, style).expect("taffy: set_style failed");
    }

    /// Установить список дочерних NodeId.
    pub fn set_children(&mut self, parent: NodeId, children: &[NodeId]) {
        self.tree
            .set_children(parent, children)
            .expect("taffy: set_children failed");
    }

    /// Установить measure-функцию для узла (для Text и т.п.).
    pub fn set_measure_fn(
        &mut self,
        node: NodeId,
        f: impl Fn(f32) -> (f32, f32) + Send + 'static,
    ) {
        if let Some(ctx) = self.tree.get_node_context_mut(node) {
            ctx.measure_fn = Some(Box::new(f));
        }
    }

    /// Убрать measure-функцию (если узел перестал быть Text).
    pub fn clear_measure_fn(&mut self, node: NodeId) {
        if let Some(ctx) = self.tree.get_node_context_mut(node) {
            ctx.measure_fn = None;
        }
    }

    /// Запустить расчёт layout для всего дерева с корнем `root`.
    /// `screen_width`, `screen_height` — размер дисплея в пикселях.
    pub fn compute(&mut self, root: NodeId, screen_width: f32, screen_height: f32) {
        self.tree
            .compute_layout_with_measure(
                root,
                Size {
                    width: AvailableSpace::Definite(screen_width),
                    height: AvailableSpace::Definite(screen_height),
                },
                |known_dimensions, available_space, _node_id, node_context, _style| {
                    // node_context здесь Option<&mut NodeContext>
                    let measure_result = node_context
                        .and_then(|ctx| ctx.measure_fn.as_ref())
                        .map(|measure_fn| {
                            let avail_w = match available_space.width {
                                AvailableSpace::Definite(w) => w,
                                AvailableSpace::MaxContent | AvailableSpace::MinContent => {
                                    f32::MAX
                                }
                            };
                            measure_fn(avail_w)
                        });
                    match measure_result {
                        Some((w, h)) => Size {
                            width: known_dimensions.width.unwrap_or(w),
                            height: known_dimensions.height.unwrap_or(h),
                        },
                        None => Size {
                            width: known_dimensions.width.unwrap_or(0.0),
                            height: known_dimensions.height.unwrap_or(0.0),
                        },
                    }
                },
            )
            .expect("taffy: compute_layout failed");
    }

    /// Получить вычисленный прямоугольник узла ОТНОСИТЕЛЬНО родителя.
    /// Чтобы получить абсолютные координаты — накапливайте смещение
    /// при рекурсивном обходе дерева компонентов.
    pub fn layout_relative(&self, node: NodeId) -> UiRect {
        let layout = self.tree.layout(node).expect("taffy: layout failed");
        UiRect {
            x: layout.location.x as i32,
            y: layout.location.y as i32,
            width: layout.size.width as i32,
            height: layout.size.height as i32,
        }
    }

    /// Удалить узел из дерева taffy.
    /// Дочерние узлы должны быть удалены заранее.
    pub fn remove_node(&mut self, node: NodeId) {
        self.tree.set_children(node, &[]).ok();
        self.tree.remove(node).ok();
    }
}
