use crate::touch::TouchPoint;
use crate::ui::canvas::UiRect;

// ─── UiEvent ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum UiEvent {
    Tap(TouchPoint),
    Slide { from: TouchPoint, to: TouchPoint },
}

// ─── EventDispatcher ─────────────────────────────────────────────────────────

struct HitTarget {
    rect: UiRect,
    handler: Box<dyn Fn()>,
}

/// Хранит все кликабельные области текущего кадра.
/// Обновляется при каждом `render()` и используется для dispatch touch-событий.
pub struct EventDispatcher {
    targets: Vec<HitTarget>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            targets: Vec::new(),
        }
    }

    /// Зарегистрировать кликабельную область. Вызывается из draw_and_collect().
    pub fn register(&mut self, rect: UiRect, handler: Box<dyn Fn()>) {
        self.targets.push(HitTarget { rect, handler });
    }

    /// Очистить все цели. Вызывать ПЕРЕД каждым render().
    pub fn clear(&mut self) {
        self.targets.clear();
    }

    /// Обработать событие. Возвращает true если попало в цель.
    /// Перебирает с конца — последний добавленный (дочерний) компонент
    /// имеет приоритет над родительским.
    pub fn dispatch(&mut self, event: &UiEvent) -> bool {
        let (px, py) = match event {
            UiEvent::Tap(p) => (p.x as i32, p.y as i32),
            UiEvent::Slide { .. } => return false,
        };
        for target in self.targets.iter().rev() {
            if target.rect.contains(px, py) {
                (target.handler)();
                return true;
            }
        }
        false
    }
}
