---
name: UI System Implementation Plan
description: Детальный план реализации React-подобного UI-движка для e-ink дисплея ESP32-S3
type: project
---

Разрабатываем UI-движок по образцу iocraft (папка iocraft/), адаптированный для пиксельного e-ink дисплея 960×540, Gray4.

**Why:** Пользователь хочет React-like компоненты (View, Text, Button, Image) + движок + click-события для todo-листа на устройстве.

**How to apply:** При любой работе над UI-системой ориентироваться на план в `docs/ui-plan.md`. Структура — `src/ui/`, зависимость — `taffy = "0.5"`.

Ключевые решения:
- Layout: taffy (1 unit = 1 pixel)
- Canvas: обёртка над EmbeddedDisplay (embedded-graphics)
- Text: embedded-graphics MonoFont (ASCII), кириллица — позже через u8g2-fonts
- Events: EventDispatcher с HitTarget (rect + FnMut()), dispatch по touch-координатам
- Reconciliation: по TypeId + индекс (без ключей, без async)
- State: поля в Component struct, on_click через Box<dyn FnMut()>

Порядок задач: Cargo.toml → canvas → layout → element → component → events → компоненты → UiApp → интеграция.
