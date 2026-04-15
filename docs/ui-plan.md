# UI-движок для e-ink дисплея — Пошаговый план реализации

## Обзор

Реализуем React-подобный UI-движок для ESP32-S3 с e-ink дисплеем (960×540, Gray4).
Вдохновлён `iocraft/`, но адаптирован для пиксельного дисплея без async и без терминала.

### Как работает система (5 фаз за один вызов `render()`):

```
Пользователь вызывает app.render(element!(View { ... }))
          │
          ▼
  ┌─── PHASE 1: UPDATE ─────────────────────────────────────┐
  │  Рекурсивно обходим AnyElement-дерево.                  │
  │  Для каждого элемента:                                   │
  │    - если тип совпал со старым — переиспользуем         │
  │    - если нет — создаём новый InstantiatedComponent      │
  │  Каждый компонент вызывает updater.set_layout_style()    │
  │  и updater.set_children()                                │
  └──────────────────────────────────────────────────────────┘
          │
          ▼
  ┌─── PHASE 2: LAYOUT ─────────────────────────────────────┐
  │  taffy::compute_layout() обходит taffy-дерево и         │
  │  рассчитывает x, y, width, height для каждого узла.     │
  │  Для Text вызывается measure_fn чтобы узнать размер.     │
  └──────────────────────────────────────────────────────────┘
          │
          ▼
  ┌─── PHASE 3: DRAW + COLLECT HITS ────────────────────────┐
  │  Рекурсивный обход InstantiatedComponent-дерева.        │
  │  Для каждого компонента:                                 │
  │    - берём layout (x, y, w, h) из taffy                 │
  │    - вызываем component.draw(rect, ctx)                 │
  │    - если у компонента есть click_handler —             │
  │      регистрируем его в EventDispatcher                 │
  │    - рекурсивно обходим детей                           │
  └──────────────────────────────────────────────────────────┘
          │
          ▼
  ┌─── PHASE 4: FLUSH ──────────────────────────────────────┐
  │  display.flush() → papers3_display_present()            │
  └──────────────────────────────────────────────────────────┘
          │
          ▼
  ┌─── PHASE 5: EVENTS (отдельно, когда приходит тач) ──────┐
  │  app.handle_event(UiEvent::Tap(point))                   │
  │  EventDispatcher проверяет все rect-ы, вызывает handler │
  └──────────────────────────────────────────────────────────┘
```

### Структура файлов (что создаём):

```
src/
├── lib.rs                    ← добавить `mod ui;`
└── ui/
    ├── mod.rs                ← UiApp — главный публичный API
    ├── canvas.rs             ← Color, UiRect, DrawContext
    ├── layout.rs             ← LayoutEngine (обёртка taffy)
    ├── element.rs            ← AnyElement, ComponentHelper, element! macro
    ├── component.rs          ← Component trait, ComponentUpdater,
    │                            InstantiatedComponent, reconciliation
    ├── events.rs             ← UiEvent, HitTarget, EventDispatcher
    └── components/
        ├── mod.rs
        ├── view.rs
        ├── text.rs
        ├── button.rs
        └── image.rs
```

---

## ШАГ 1 — Добавить зависимость `taffy` в `Cargo.toml`

### Что делать

Открыть `Cargo.toml`, найти секцию `[dependencies]` и добавить строку:

```toml
taffy = { version = "0.5", default-features = false, features = ["std"] }
```

### Почему именно так

`taffy` — это Rust-реализация CSS Flexbox (та же что используется в React Native и iocraft).
Она умеет рассчитывать позиции и размеры элементов по правилам flex-контейнеров.
`default-features = false` убирает ненужные фичи, `features = ["std"]` включает нужные
(у нас esp-idf — это std-среда, не no_std).

### Как проверить

```bash
cargo build 2>&1 | head -20
```

Должен скомпилироваться без ошибок. Если `taffy` не найден — проверить spelling.

---

## ШАГ 2 — `src/ui/canvas.rs` — примитивы рисования

Это самый нижний слой. DrawContext — тонкая обёртка над `EmbeddedDisplay`,
которая предоставляет удобный API для компонентов.

### 2.1 Создать файл `src/ui/canvas.rs` с типами данных

```rust
use embedded_graphics::mono_font::ascii::{FONT_10X20, FONT_6X13, FONT_9X18};
use embedded_graphics::mono_font::{MonoFont, MonoTextStyle};
use embedded_graphics::pixelcolor::Gray4;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::{Baseline, Text};

use crate::display::EmbeddedDisplay;
use crate::image::BmpImage;

// ─── Color ──────────────────────────────────────────────────────────────────

/// Оттенок серого. 0 = чёрный, 15 = белый.
/// Соответствует nibble в Gray4-буфере дисплея.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Color(pub u8);

impl Color {
    pub const BLACK: Color = Color(0);
    pub const WHITE: Color = Color(15);
    pub const DARK_GRAY: Color = Color(4);
    pub const GRAY: Color = Color(8);
    pub const LIGHT_GRAY: Color = Color(11);

    pub(crate) fn to_gray4(self) -> Gray4 {
        Gray4::new(self.0 & 0x0F)
    }
}

// ─── UiRect ─────────────────────────────────────────────────────────────────

/// Прямоугольник в пикселях. x, y — верхний левый угол.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl UiRect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }

    /// Проверить, попадает ли точка (px, py) внутрь прямоугольника.
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && px < self.x + self.width
            && py >= self.y
            && py < self.y + self.height
    }
}

// ─── FontSize и TextStyle ────────────────────────────────────────────────────

/// Выбор шрифта из встроенных bitmap-шрифтов embedded-graphics.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum FontSize {
    Small,   // 6×13 px на символ
    #[default]
    Medium,  // 9×18 px на символ
    Large,   // 10×20 px на символ
}

impl FontSize {
    /// Возвращает ссылку на статический MonoFont.
    pub fn font(self) -> &'static MonoFont<'static> {
        match self {
            FontSize::Small  => &FONT_6X13,
            FontSize::Medium => &FONT_9X18,
            FontSize::Large  => &FONT_10X20,
        }
    }

    /// Ширина одного символа в пикселях.
    pub fn char_width(self) -> i32 {
        self.font().character_size.width as i32
    }

    /// Высота строки в пикселях (включая межстрочный интервал).
    pub fn line_height(self) -> i32 {
        self.font().character_size.height as i32
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TextStyle {
    pub font_size: FontSize,
    pub color: Color,
}

// ─── DrawContext ─────────────────────────────────────────────────────────────

/// Контекст рисования. Компоненты получают его в методе draw().
/// Не владеет дисплеем — берёт мутабельную ссылку на время рендера.
pub struct DrawContext<'a> {
    display: &'a mut EmbeddedDisplay,
}

impl<'a> DrawContext<'a> {
    pub fn new(display: &'a mut EmbeddedDisplay) -> Self {
        Self { display }
    }

    /// Залить прямоугольник цветом (заливка без обводки).
    pub fn fill_rect(&mut self, rect: UiRect, color: Color) {
        if rect.width <= 0 || rect.height <= 0 {
            return;
        }
        let style = PrimitiveStyleBuilder::new()
            .fill_color(color.to_gray4())
            .build();
        Rectangle::new(
            Point::new(rect.x, rect.y),
            Size::new(rect.width as u32, rect.height as u32),
        )
        .into_styled(style)
        .draw(self.display)
        .ok();
    }

    /// Нарисовать рамку вокруг прямоугольника (без заливки).
    /// `thickness` — толщина линии в пикселях.
    pub fn stroke_rect(&mut self, rect: UiRect, color: Color, thickness: u32) {
        if rect.width <= 0 || rect.height <= 0 || thickness == 0 {
            return;
        }
        let style = PrimitiveStyleBuilder::new()
            .stroke_color(color.to_gray4())
            .stroke_width(thickness)
            .build();
        Rectangle::new(
            Point::new(rect.x, rect.y),
            Size::new(rect.width as u32, rect.height as u32),
        )
        .into_styled(style)
        .draw(self.display)
        .ok();
    }

    /// Нарисовать однострочный текст.
    /// `x`, `y` — верхний левый угол области текста.
    pub fn draw_text(&mut self, text: &str, x: i32, y: i32, style: TextStyle) {
        if text.is_empty() {
            return;
        }
        let font = style.font_size.font();
        let text_style = MonoTextStyle::new(font, style.color.to_gray4());

        // embedded-graphics рисует текст от baseline.
        // Нам нужен верхний левый угол, поэтому сдвигаем y на baseline.
        let baseline_y = y + font.baseline as i32;

        Text::with_baseline(text, Point::new(x, baseline_y), text_style, Baseline::Alphabetic)
            .draw(self.display)
            .ok();
    }

    /// Измерить размер текста БЕЗ рисования.
    /// Возвращает (width, height) в пикселях.
    /// Многострочный текст: каждая строка разделена '\n'.
    pub fn measure_text(text: &str, style: TextStyle) -> (i32, i32) {
        let font = style.font_size.font();
        let char_w = font.character_size.width as i32;
        let line_h = font.character_size.height as i32;

        let mut max_w = 0i32;
        let mut lines = 0i32;
        for line in text.split('\n') {
            max_w = max_w.max(line.len() as i32 * char_w);
            lines += 1;
        }
        if lines == 0 {
            lines = 1;
        }
        (max_w, lines * line_h)
    }

    /// Нарисовать BmpImage с верхним левым углом в (x, y).
    pub fn draw_image(&mut self, image: &BmpImage, x: i32, y: i32) {
        self.display.draw_bitmap(image, x, y);
    }
}
```

### 2.2 Добавить `mod ui;` в `src/lib.rs`

```rust
// src/lib.rs — в конце блока с mod-объявлениями добавить:
mod ui;
pub use ui::UiApp;
```

И создать пока пустой `src/ui/mod.rs`:

```rust
// src/ui/mod.rs
mod canvas;
mod component;
mod element;
mod events;
mod layout;

pub mod components;

pub use canvas::{Color, DrawContext, FontSize, TextStyle, UiRect};
pub use events::{EventDispatcher, UiEvent};

// UiApp будет добавлен в конце (шаг 11)
```

### Как проверить шаг 2

```bash
cargo build 2>&1 | grep "error"
```

Ошибок быть не должно. Предупреждения о неиспользуемых импортах — нормально на этом этапе.

---

## ШАГ 3 — `src/ui/layout.rs` — обёртка над taffy

Layout engine принимает стили компонентов и рассчитывает конкретные пиксельные позиции.

### 3.1 Понимание taffy

taffy работает с деревом узлов (`NodeId`). Для каждого узла задаётся:
- `Style` — flex-стиль (direction, gap, padding, size, flex_grow, ...)
- опционально — `measure_fn` — функция, которая говорит taffy какого размера узел
  (нужно для Text, чтобы размер зависел от контента)

После вызова `compute_layout()` каждый узел имеет вычисленный `Layout`:
- `layout.location.x`, `layout.location.y` — **относительно родителя** (важно!)
- `layout.size.width`, `layout.size.height`

### 3.2 Создать `src/ui/layout.rs`

```rust
use taffy::prelude::*;

use crate::ui::canvas::UiRect;

// Контекст узла: опциональная функция измерения.
// Хранится в taffy-дереве рядом с каждым NodeId.
// Нужна только для Text (и подобных компонентов с контент-зависимым размером).
#[derive(Default)]
pub struct NodeContext {
    /// f(available_width_px) → (width_px, height_px)
    pub measure_fn: Option<Box<dyn Fn(f32) -> (f32, f32) + Send>>,
}

pub struct LayoutEngine {
    tree: TaffyTree<NodeContext>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
        }
    }

    /// Создать новый узел-лист с заданным стилем.
    /// "Лист" = узел без детей (или с measure_fn вместо детей).
    pub fn new_leaf(&mut self, style: Style) -> NodeId {
        self.tree
            .new_leaf_with_context(style, NodeContext::default())
            .expect("taffy: new_leaf failed")
    }

    /// Установить новый стиль для существующего узла.
    pub fn set_style(&mut self, node: NodeId, style: Style) {
        self.tree.set_style(node, style).expect("taffy: set_style failed");
    }

    /// Установить список дочерних NodeId для узла.
    /// Вызывается после того как все дочерние узлы уже созданы.
    pub fn set_children(&mut self, parent: NodeId, children: &[NodeId]) {
        self.tree
            .set_children(parent, children)
            .expect("taffy: set_children failed");
    }

    /// Установить measure-функцию для узла.
    /// После установки taffy будет вызывать её при расчёте layout.
    pub fn set_measure_fn(
        &mut self,
        node: NodeId,
        f: impl Fn(f32) -> (f32, f32) + Send + 'static,
    ) {
        if let Some(ctx) = self.tree.get_node_context_mut(node) {
            ctx.measure_fn = Some(Box::new(f));
        }
    }

    /// Сбросить measure-функцию (для узлов, которые перестали быть Text).
    pub fn clear_measure_fn(&mut self, node: NodeId) {
        if let Some(ctx) = self.tree.get_node_context_mut(node) {
            ctx.measure_fn = None;
        }
    }

    /// Запустить расчёт layout для всего дерева.
    /// `root` — корневой NodeId.
    /// `screen_width`, `screen_height` — размер дисплея в пикселях.
    pub fn compute(&mut self, root: NodeId, screen_width: f32, screen_height: f32) {
        self.tree
            .compute_layout_with_measure(
                root,
                Size {
                    width: AvailableSpace::Definite(screen_width),
                    height: AvailableSpace::Definite(screen_height),
                },
                // Эта функция вызывается taffy для узлов с measure_fn
                |known_dimensions, available_space, _node_id, node_context, _style| {
                    let Some(ref measure_fn) = node_context.measure_fn else {
                        // Нет measure_fn → возвращаем known size или 0
                        return Size {
                            width: known_dimensions.width.unwrap_or(0.0),
                            height: known_dimensions.height.unwrap_or(0.0),
                        };
                    };
                    // available_space.width → конкретное число пикселей или MAX
                    let avail_w = match available_space.width {
                        AvailableSpace::Definite(w) => w,
                        AvailableSpace::MaxContent | AvailableSpace::MinContent => f32::MAX,
                    };
                    let (w, h) = measure_fn(avail_w);
                    Size {
                        width: known_dimensions.width.unwrap_or(w),
                        height: known_dimensions.height.unwrap_or(h),
                    }
                },
            )
            .expect("taffy: compute_layout failed");
    }

    /// Получить вычисленные размеры узла.
    /// ВАЖНО: location — ОТНОСИТЕЛЬНО РОДИТЕЛЯ, не абсолютная.
    /// Абсолютные координаты считаются при обходе дерева (шаг 5).
    pub fn layout_relative(&self, node: NodeId) -> UiRect {
        let layout = self.tree.layout(node).expect("taffy: layout failed");
        UiRect {
            x: layout.location.x as i32,
            y: layout.location.y as i32,
            width: layout.size.width as i32,
            height: layout.size.height as i32,
        }
    }

    /// Удалить узел из taffy (без детей — они уже должны быть удалены).
    pub fn remove_node(&mut self, node: NodeId) {
        // Сначала отсоединяем все дочерние узлы, иначе taffy не даст удалить
        self.tree.set_children(node, &[]).ok();
        self.tree.remove(node).ok();
    }
}

// ─── Вспомогательные функции для создания taffy-стилей ─────────────────────

/// Фиксированный размер в пикселях.
pub fn px(value: f32) -> Dimension {
    Dimension::Length(value)
}

/// Размер "заполни оставшееся" (как flex-grow: 1 + width: auto).
/// Используется через flex_grow: 1.0 в Style.
pub fn auto() -> Dimension {
    Dimension::Auto
}

/// Создать `LengthPercentage` из пикселей (для gap, padding).
pub fn length_px(value: f32) -> LengthPercentage {
    LengthPercentage::Length(value)
}

/// Создать `Rect<LengthPercentage>` одинаковый со всех сторон.
pub fn padding_all(value: f32) -> Rect<LengthPercentage> {
    let v = length_px(value);
    Rect { left: v, right: v, top: v, bottom: v }
}

/// Создать `Rect<LengthPercentage>` с разными значениями.
pub fn padding_lrtb(left: f32, right: f32, top: f32, bottom: f32) -> Rect<LengthPercentage> {
    Rect {
        left: length_px(left),
        right: length_px(right),
        top: length_px(top),
        bottom: length_px(bottom),
    }
}
```

### Как проверить шаг 3

```bash
cargo build 2>&1 | grep "error"
```

---

## ШАГ 4 — `src/ui/element.rs` — система элементов

Элемент — это описание того, что нужно нарисовать (тип компонента + его props).
Аналог React-элемента: `<Button onClick={...}>текст</Button>`.

### 4.1 Почему нужен `AnyElement` (type erasure)

Дерево компонентов содержит разные типы: `View`, `Text`, `Button`, `Image`.
Rust требует, чтобы типы в `Vec` были одинаковыми. Решение — стереть тип через `dyn Any`
и хранить в `AnyElement`. Для работы с конкретным типом используется `ComponentHelper`.

### 4.2 Создать `src/ui/element.rs`

```rust
use std::any::{Any, TypeId};

use crate::ui::canvas::{DrawContext, UiRect};
use crate::ui::layout::LayoutEngine;

// Forward declaration (реализован в component.rs)
use crate::ui::component::ComponentUpdater;

// ─── ComponentHelper — статический vtable ───────────────────────────────────

/// Набор операций над компонентом конкретного типа, скрытый за dyn trait.
/// Для каждого типа C создаётся одна статическая реализация ComponentHelperImpl<C>.
pub trait ComponentHelper: Send + Sync + 'static {
    /// TypeId компонента, которым управляет этот helper.
    fn component_type_id(&self) -> TypeId;

    /// Создать новый экземпляр компонента из props.
    /// Вызывается при первом появлении элемента этого типа.
    fn create(&self, props: &dyn Any) -> Box<dyn AnyComponent>;

    /// Обновить существующий компонент новыми props.
    /// Вызывается при каждом render() если компонент уже существует.
    fn update(
        &self,
        component: &mut dyn AnyComponent,
        props: &dyn Any,
        updater: &mut ComponentUpdater,
    );

    /// Нарисовать компонент в DrawContext.
    fn draw(
        &self,
        component: &dyn AnyComponent,
        rect: UiRect,
        ctx: &mut DrawContext,
    );
}

// ─── AnyComponent — type-erased компонент ───────────────────────────────────

/// Трейт-объект для Component<Props=...>.
/// Нужен чтобы хранить Box<dyn AnyComponent> без знания конкретного типа.
pub trait AnyComponent: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// ─── ComponentHelperImpl<C> ──────────────────────────────────────────────────

/// Конкретная реализация ComponentHelper для типа C.
/// ZST (zero-sized type) — не занимает памяти.
pub(crate) struct ComponentHelperImpl<C: crate::ui::component::Component + 'static>(
    pub std::marker::PhantomData<C>,
);

unsafe impl<C: crate::ui::component::Component + 'static> Send for ComponentHelperImpl<C> {}
unsafe impl<C: crate::ui::component::Component + 'static> Sync for ComponentHelperImpl<C> {}

impl<C: crate::ui::component::Component + 'static> ComponentHelper for ComponentHelperImpl<C> {
    fn component_type_id(&self) -> TypeId {
        TypeId::of::<C>()
    }

    fn create(&self, props: &dyn Any) -> Box<dyn AnyComponent> {
        let props = props
            .downcast_ref::<C::Props>()
            .expect("ComponentHelper::create: wrong props type");
        Box::new(C::new(props))
    }

    fn update(
        &self,
        component: &mut dyn AnyComponent,
        props: &dyn Any,
        updater: &mut ComponentUpdater,
    ) {
        let component = component
            .as_any_mut()
            .downcast_mut::<C>()
            .expect("ComponentHelper::update: wrong component type");
        let props = props
            .downcast_ref::<C::Props>()
            .expect("ComponentHelper::update: wrong props type");
        component.update(props, updater);
    }

    fn draw(&self, component: &dyn AnyComponent, rect: UiRect, ctx: &mut DrawContext) {
        let component = component
            .as_any()
            .downcast_ref::<C>()
            .expect("ComponentHelper::draw: wrong component type");
        component.draw(rect, ctx);
    }
}

// ─── AnyElement ─────────────────────────────────────────────────────────────

/// Type-erased элемент (тип + props) для хранения в Vec.
pub struct AnyElement {
    type_id: TypeId,
    props: Box<dyn Any>,
    pub(crate) helper: &'static dyn ComponentHelper,
}

impl AnyElement {
    /// Создать AnyElement для компонента типа C с заданными props.
    pub fn new<C: crate::ui::component::Component + 'static>(props: C::Props) -> Self {
        // Box::leak создаёт статическую ссылку на helper.
        // Вызывается по одному разу на каждый уникальный тип C.
        // Для 4-5 типов компонентов это утечка ~несколько байт — приемлемо.
        let helper: &'static dyn ComponentHelper =
            Box::leak(Box::new(ComponentHelperImpl::<C>(std::marker::PhantomData)));
        Self {
            type_id: TypeId::of::<C>(),
            props: Box::new(props),
            helper,
        }
    }

    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub fn props(&self) -> &dyn Any {
        self.props.as_ref()
    }
}

// ─── element! macro ──────────────────────────────────────────────────────────

/// Удобный макрос для создания AnyElement.
///
/// Примеры:
/// ```
/// element!(View { direction: Direction::Column, background: Some(Color::WHITE) })
/// element!(Text { content: "Hello".into(), font_size: FontSize::Large })
/// element!(Button { on_click: Some(Box::new(|| { ... })) })
/// ```
///
/// Незаполненные поля берутся из Default::default().
#[macro_export]
macro_rules! element {
    ($type:ty { $($field:ident: $value:expr),* $(,)? }) => {
        $crate::ui::element::AnyElement::new::<$type>(
            <$type as $crate::ui::component::Component>::Props {
                $($field: $value,)*
                ..Default::default()
            }
        )
    };
}
```

### Как проверить шаг 4

Файл должен компилироваться. Пока `ComponentUpdater` не реализован — будет ошибка forward reference.
Это нормально, продолжаем к шагу 5.

---

## ШАГ 5 — `src/ui/component.rs` — ядро движка

Самый сложный файл. Здесь:
- `Component` trait — интерфейс для пользовательских компонентов
- `ComponentUpdater` — объект, через который компонент сообщает движку свои настройки
- `InstantiatedComponent` — живой компонент с NodeId в taffy и дочерними компонентами
- Reconciliation — логика переиспользования компонентов между рендерами

### 5.1 Создать `src/ui/component.rs`

```rust
use std::any::{Any, TypeId};
use std::sync::Arc;

use taffy::Style;

use crate::ui::canvas::{DrawContext, UiRect};
use crate::ui::element::{AnyComponent, AnyElement, ComponentHelper};
use crate::ui::events::EventDispatcher;
use crate::ui::layout::LayoutEngine;

// ─── Component trait ─────────────────────────────────────────────────────────

/// Основной трейт для всех UI-компонентов.
/// Реализуйте его для каждого нового компонента (View, Text, Button, Image).
pub trait Component: Any + Sized + 'static {
    /// Тип props этого компонента. Должен реализовывать Default.
    type Props: Default + 'static;

    /// Создать новый экземпляр компонента из props.
    /// Вызывается один раз при первом появлении компонента.
    fn new(props: &Self::Props) -> Self;

    /// Обновить состояние компонента и зарегистрировать layout.
    /// Вызывается при каждом render().
    ///
    /// Здесь компонент должен:
    /// - обновить свои поля из props
    /// - вызвать updater.set_layout_style() с flex-стилем
    /// - вызвать updater.set_children() если есть дочерние элементы
    /// - вызвать updater.set_measure_fn() если размер зависит от контента (Text)
    /// - вызвать updater.register_click_handler() если компонент кликабелен (Button)
    fn update(&mut self, props: &Self::Props, updater: &mut ComponentUpdater);

    /// Нарисовать компонент в DrawContext.
    /// `rect` — вычисленный taffy прямоугольник в абсолютных пикселях.
    ///
    /// НЕ нужно рисовать дочерние компоненты — движок сделает это автоматически.
    fn draw(&self, rect: UiRect, ctx: &mut DrawContext) {
        let _ = (rect, ctx); // по умолчанию ничего не рисует
    }
}

// Реализация AnyComponent для всех Component
impl<C: Component + Any> AnyComponent for C {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// ─── ComponentUpdater ────────────────────────────────────────────────────────

/// Объект, передаваемый в Component::update().
/// Компонент использует его чтобы "сообщить" движку свои настройки.
pub struct ComponentUpdater<'a> {
    pub(crate) node_id: taffy::NodeId,
    pub(crate) engine: &'a mut LayoutEngine,
    pub(crate) pending_children: Vec<AnyElement>,
    pub(crate) click_handler: Option<Arc<dyn Fn() + Send + Sync>>,
    pub(crate) has_measure_fn: bool,
}

impl<'a> ComponentUpdater<'a> {
    pub(crate) fn new(node_id: taffy::NodeId, engine: &'a mut LayoutEngine) -> Self {
        Self {
            node_id,
            engine,
            pending_children: Vec::new(),
            click_handler: None,
            has_measure_fn: false,
        }
    }

    /// Установить flex-стиль этого узла в taffy.
    /// Должен вызываться в каждом update().
    pub fn set_layout_style(&mut self, style: Style) {
        self.engine.set_style(self.node_id, style);
        if !self.has_measure_fn {
            self.engine.clear_measure_fn(self.node_id);
        }
    }

    /// Передать список дочерних элементов.
    /// Движок сам создаст/переиспользует/удалит компоненты для них.
    pub fn set_children(&mut self, children: Vec<AnyElement>) {
        self.pending_children = children;
    }

    /// Установить функцию измерения размера (только для Text и подобных).
    /// `f` принимает доступную ширину в пикселях, возвращает (ширина, высота).
    pub fn set_measure_fn(&mut self, f: impl Fn(f32) -> (f32, f32) + Send + 'static) {
        self.has_measure_fn = true;
        self.engine.set_measure_fn(self.node_id, f);
    }

    /// Зарегистрировать обработчик нажатия (только для Button).
    /// Handler будет вызван когда пользователь тапнет по этому компоненту.
    pub fn register_click_handler(&mut self, handler: impl Fn() + Send + Sync + 'static) {
        self.click_handler = Some(Arc::new(handler));
    }
}

// ─── InstantiatedComponent ───────────────────────────────────────────────────

/// Живой компонент в UI-дереве.
/// Хранит:
/// - node_id: соответствующий узел в taffy
/// - component: фактический объект компонента (View/Text/Button/Image)
/// - children: дочерние InstantiatedComponent
/// - click_handler: Arc на функцию клика (если есть)
pub struct InstantiatedComponent {
    pub(crate) node_id: taffy::NodeId,
    type_id: TypeId,
    component: Box<dyn AnyComponent>,
    pub(crate) children: Vec<InstantiatedComponent>,
    helper: &'static dyn ComponentHelper,
    click_handler: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl InstantiatedComponent {
    /// Создать новый компонент из AnyElement.
    /// Выделяет новый NodeId в taffy.
    fn create(element: &AnyElement, engine: &mut LayoutEngine) -> Self {
        let node_id = engine.new_leaf(Style::default());
        let component = element.helper.create(element.props());
        Self {
            node_id,
            type_id: element.type_id(),
            component,
            children: Vec::new(),
            helper: element.helper,
            click_handler: None,
        }
    }

    /// Обновить компонент новыми props и рекурсивно reconcile дочерние компоненты.
    /// Вызывается при каждом render().
    pub fn update(&mut self, element: &AnyElement, engine: &mut LayoutEngine) {
        // Запустить Component::update() через helper (type-erased вызов)
        let mut updater = ComponentUpdater::new(self.node_id, engine);
        self.helper.update(&mut *self.component, element.props(), &mut updater);

        // Сохранить click_handler (Arc → можно клонировать в EventDispatcher)
        self.click_handler = updater.click_handler;

        // Reconcile дочерние компоненты
        let incoming = updater.pending_children;
        self.reconcile_children(incoming, updater.engine);
    }

    /// Логика reconciliation: сопоставляем входящие AnyElement с существующими детьми.
    ///
    /// Стратегия: сравниваем по индексу + TypeId.
    ///   - Тип совпал → переиспользуем, вызываем update()
    ///   - Тип не совпал → удаляем старый, создаём новый
    ///   - Детей стало меньше → удаляем лишние с конца
    ///   - Детей стало больше → создаём новые
    fn reconcile_children(
        &mut self,
        incoming: Vec<AnyElement>,
        engine: &mut LayoutEngine,
    ) {
        let old_len = self.children.len();
        let new_len = incoming.len();

        // Шаг 1: Обработать позиции от 0 до min(old_len, new_len)
        for (i, child_element) in incoming.iter().enumerate().take(old_len.min(new_len)) {
            if self.children[i].type_id != child_element.type_id() {
                // Тип изменился → заменяем компонент
                // Создаём новый (временно в отдельной переменной)
                let new_child = InstantiatedComponent::create(child_element, engine);
                // Заменяем старый на новый, получаем старый
                let old_child = std::mem::replace(&mut self.children[i], new_child);
                // Удаляем старый из taffy
                old_child.remove_from_engine(engine);
            }
            // Обновляем (создан только что или переиспользован)
            self.children[i].update(child_element, engine);
        }

        // Шаг 2: Если детей стало меньше — удалить лишние с конца
        while self.children.len() > new_len {
            let removed = self.children.pop().unwrap();
            removed.remove_from_engine(engine);
        }

        // Шаг 3: Если детей стало больше — добавить новые
        for child_element in incoming.iter().skip(old_len) {
            let mut new_child = InstantiatedComponent::create(child_element, engine);
            new_child.update(child_element, engine);
            self.children.push(new_child);
        }

        // Шаг 4: Сообщить taffy о новом списке детей
        let child_ids: Vec<taffy::NodeId> = self.children.iter().map(|c| c.node_id).collect();
        engine.set_children(self.node_id, &child_ids);
    }

    /// Рекурсивно нарисовать компонент и всех его детей.
    /// `parent_x`, `parent_y` — абсолютные координаты родителя.
    ///
    /// ВАЖНО: taffy хранит координаты ОТНОСИТЕЛЬНО РОДИТЕЛЯ.
    /// Поэтому мы накапливаем смещение по мере обхода дерева.
    pub fn draw_and_collect(
        &self,
        engine: &LayoutEngine,
        parent_x: i32,
        parent_y: i32,
        ctx: &mut DrawContext,
        dispatcher: &mut EventDispatcher,
    ) {
        // Получить relative layout и пересчитать в абсолютные координаты
        let rel = engine.layout_relative(self.node_id);
        let abs_rect = UiRect {
            x: parent_x + rel.x,
            y: parent_y + rel.y,
            width: rel.width,
            height: rel.height,
        };

        // Нарисовать этот компонент
        self.helper.draw(&*self.component, abs_rect, ctx);

        // Если есть click_handler — зарегистрировать в EventDispatcher
        if let Some(ref handler) = self.click_handler {
            let handler_clone = Arc::clone(handler);
            dispatcher.register(abs_rect, Box::new(move || handler_clone()));
        }

        // Рекурсивно нарисовать детей
        // Дети позиционируются относительно нас, поэтому передаём abs_rect.x/y
        for child in &self.children {
            child.draw_and_collect(engine, abs_rect.x, abs_rect.y, ctx, dispatcher);
        }
    }

    /// Удалить этот компонент и всё его поддерево из taffy.
    /// Вызывается при reconciliation когда компонент заменяется другим типом.
    fn remove_from_engine(self, engine: &mut LayoutEngine) {
        // Сначала рекурсивно удалить детей
        for child in self.children {
            child.remove_from_engine(engine);
        }
        // Затем удалить сам узел
        engine.remove_node(self.node_id);
    }
}
```

### Как проверить шаг 5

```bash
cargo build 2>&1 | grep "error"
```

Могут быть ошибки про `events.rs` — создадим в следующем шаге.

---

## ШАГ 6 — `src/ui/events.rs` — система событий

```rust
use crate::ui::canvas::UiRect;
use crate::touch::TouchPoint;

// ─── UiEvent ─────────────────────────────────────────────────────────────────

/// События от тач-контроллера GT911.
#[derive(Debug, Clone)]
pub enum UiEvent {
    /// Одиночное касание.
    Tap(TouchPoint),
    /// Скользящий жест.
    Slide { from: TouchPoint, to: TouchPoint },
}

// ─── HitTarget ───────────────────────────────────────────────────────────────

/// Один кликабельный прямоугольник с обработчиком.
/// Создаётся в draw_and_collect() для каждого Button.
pub struct HitTarget {
    pub rect: UiRect,
    /// Функция, вызываемая при попадании тача в rect.
    handler: Box<dyn Fn()>,
}

// ─── EventDispatcher ─────────────────────────────────────────────────────────

/// Хранит все HitTarget-ы текущего кадра.
/// Обновляется при каждом render() и используется для dispatch touch-событий.
pub struct EventDispatcher {
    targets: Vec<HitTarget>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self { targets: Vec::new() }
    }

    /// Добавить кликабельную область. Вызывается из draw_and_collect().
    pub fn register(&mut self, rect: UiRect, handler: Box<dyn Fn()>) {
        self.targets.push(HitTarget { rect, handler });
    }

    /// Очистить все цели. Вызывать ПЕРЕД каждым render().
    pub fn clear(&mut self) {
        self.targets.clear();
    }

    /// Обработать событие. Возвращает true если событие попало в цель.
    ///
    /// Перебирает targets в порядке добавления (т.е. от первого отрисованного
    /// к последнему). Для правильного z-order сначала добавляются родители,
    /// потом дети — поэтому тач попадёт в дочерний Button раньше.
    ///
    /// Если нужен обратный порядок (ближний к пользователю приоритетнее) —
    /// итерировать targets.iter().rev().
    pub fn dispatch(&mut self, event: &UiEvent) -> bool {
        let (px, py) = match event {
            UiEvent::Tap(p) => (p.x as i32, p.y as i32),
            UiEvent::Slide { .. } => return false, // скользящий жест пока не обрабатываем
        };

        // Ищем с конца (последний добавленный = самый "верхний" по z-order)
        for target in self.targets.iter().rev() {
            if target.rect.contains(px, py) {
                (target.handler)();
                return true;
            }
        }
        false
    }
}
```

---

## ШАГ 7 — `src/ui/components/view.rs` — контейнер

View — это flex-контейнер, аналог `<div>` в HTML. Умеет:
- располагать детей в строку (`Row`) или колонку (`Column`)
- задавать отступы, промежутки, выравнивание
- рисовать фон и рамку

### 7.1 Вспомогательные типы (добавить в начало `view.rs` или в отдельный `types.rs`)

```rust
use taffy::prelude::*;
use crate::ui::canvas::{Color, DrawContext, UiRect};
use crate::ui::component::{Component, ComponentUpdater};
use crate::ui::element::AnyElement;
use crate::ui::layout::{padding_all, padding_lrtb, px, auto};

// ─── Direction ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Direction {
    #[default]
    Row,     // дети идут слева направо
    Column,  // дети идут сверху вниз
}

// ─── SizeValue ───────────────────────────────────────────────────────────────

/// Способ задать размер (width или height) для View.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SizeValue {
    #[default]
    Auto,         // taffy сам решает (wrap content или по flex правилам)
    Fixed(i32),   // фиксированный размер в пикселях
    Percent(f32), // процент от родителя
}

impl SizeValue {
    pub(crate) fn to_taffy(self) -> Dimension {
        match self {
            SizeValue::Auto => Dimension::Auto,
            SizeValue::Fixed(px_val) => Dimension::Length(px_val as f32),
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
    pub fn all(value: i32) -> Self {
        Self { top: value, right: value, bottom: value, left: value }
    }
    pub fn horizontal(h: i32) -> Self {
        Self { top: 0, right: h, bottom: 0, left: h }
    }
    pub fn vertical(v: i32) -> Self {
        Self { top: v, right: 0, bottom: v, left: 0 }
    }
    pub fn lrtb(left: i32, right: i32, top: i32, bottom: i32) -> Self {
        Self { top, right, bottom, left }
    }
}

// ─── Border ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct Border {
    pub color: Color,
    pub thickness: u32,
}

// ─── AlignItems и JustifyContent ─────────────────────────────────────────────

// Используем taffy-типы напрямую — они уже имеют нужные варианты.
// Re-export для удобства:
pub use taffy::AlignItems;
pub use taffy::JustifyContent;
```

### 7.2 Структура View и её Component impl

```rust
// ─── ViewProps ───────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct ViewProps {
    /// Направление main-оси flex-контейнера
    pub direction: Direction,
    /// Промежуток между детьми в пикселях
    pub gap: i32,
    /// Внутренние отступы
    pub padding: EdgeInsets,
    /// Цвет фона (None = прозрачный)
    pub background: Option<Color>,
    /// Рамка (None = нет)
    pub border: Option<Border>,
    /// Ширина (Auto по умолчанию)
    pub width: SizeValue,
    /// Высота (Auto по умолчанию)
    pub height: SizeValue,
    /// flex-grow: насколько View растягивается в свободном пространстве
    /// 0.0 = не растягивается, 1.0 = занимает всё доступное
    pub flex_grow: f32,
    /// Выравнивание детей по cross-оси
    pub align_items: Option<AlignItems>,
    /// Выравнивание детей по main-оси
    pub justify_content: Option<JustifyContent>,
    /// Дочерние элементы
    pub children: Vec<AnyElement>,
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

    fn update(&mut self, props: &ViewProps, updater: &mut ComponentUpdater) {
        self.background = props.background;
        self.border = props.border;

        // Перевести props во flex-стиль taffy
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

        // Передать дочерние элементы движку
        // std::mem::take забирает Vec из props (не клонирует содержимое)
        updater.set_children(std::mem::take(&mut { props.children.clone() }));
        // Примечание: props.children содержит Box<dyn Fn()> которые не Clone.
        // Поэтому ViewProps::children при создании должны быть fresh каждый render.
    }

    fn draw(&self, rect: UiRect, ctx: &mut DrawContext) {
        // Нарисовать фон
        if let Some(bg) = self.background {
            ctx.fill_rect(rect, bg);
        }
        // Нарисовать рамку
        if let Some(b) = self.border {
            ctx.stroke_rect(rect, b.color, b.thickness);
        }
        // Дочерние элементы рисуются движком автоматически — здесь ничего не делать.
    }
}
```

**Важный нюанс с children:** `ViewProps::children: Vec<AnyElement>` не реализует `Clone`
если в нём есть `Button` с `Box<dyn Fn()>`. Поэтому при каждом `render()` пользователь
должен создавать `ViewProps` заново (не клонировать). Это нормально — как в React.

---

## ШАГ 8 — `src/ui/components/text.rs` — текстовый компонент

```rust
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
    /// Содержимое. Многострочный текст: разделять символом '\n'.
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

    fn update(&mut self, props: &TextProps, updater: &mut ComponentUpdater) {
        self.content = props.content.clone();
        self.style = TextStyle { font_size: props.font_size, color: props.color };
        self.align = props.align;

        // Text не имеет детей — задаём только стиль и measure_fn.
        // Стиль минимальный: auto size (taffy будет использовать measure_fn).
        updater.set_layout_style(Style {
            // Нет явного размера — taffy вызовет measure_fn
            size: Size { width: Dimension::Auto, height: Dimension::Auto },
            ..Style::default()
        });

        // measure_fn: taffy вызывает это чтобы узнать размер Text.
        // Замыкание захватывает content и style — они нужны для расчёта.
        let content = self.content.clone();
        let style = self.style;
        updater.set_measure_fn(move |_avail_width| {
            let (w, h) = DrawContext::measure_text(&content, style);
            (w as f32, h as f32)
        });
    }

    fn draw(&self, rect: UiRect, ctx: &mut DrawContext) {
        // Вычислить x с учётом выравнивания
        let (text_w, _) = DrawContext::measure_text(&self.content, self.style);
        let x = match self.align {
            TextAlign::Left => rect.x,
            TextAlign::Center => rect.x + (rect.width - text_w) / 2,
            TextAlign::Right => rect.x + rect.width - text_w,
        };
        ctx.draw_text(&self.content, x, rect.y, self.style);
    }
}
```

---

## ШАГ 9 — `src/ui/components/button.rs` — кликабельный контейнер

Button = View + регистрация click_handler. Самый важный компонент для UX.

```rust
use taffy::prelude::*;

use crate::ui::canvas::{Color, DrawContext, UiRect};
use crate::ui::component::{Component, ComponentUpdater};
use crate::ui::element::AnyElement;

use super::view::{Border, Direction, EdgeInsets, SizeValue, AlignItems, JustifyContent};

// ─── ButtonProps ─────────────────────────────────────────────────────────────

pub struct ButtonProps {
    /// Функция, вызываемая при нажатии.
    /// Option потому что Default требует None.
    pub on_click: Option<Box<dyn Fn() + Send + Sync>>,
    pub background: Option<Color>,
    /// Цвет фона в момент нажатия (визуальный отклик).
    pub pressed_background: Option<Color>,
    pub border: Option<Border>,
    // Layout (те же что у View)
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

// Ручная реализация Default т.к. Box<dyn Fn()> не реализует Default автоматически
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
    pressed_background: Option<Color>,
    border: Option<Border>,
}

impl Component for Button {
    type Props = ButtonProps;

    fn new(props: &ButtonProps) -> Self {
        Self {
            background: props.background,
            pressed_background: props.pressed_background,
            border: props.border,
        }
    }

    fn update(&mut self, props: &ButtonProps, updater: &mut ComponentUpdater) {
        self.background = props.background;
        self.pressed_background = props.pressed_background;
        self.border = props.border;

        // Layout идентичен View
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

        updater.set_children(props.children.clone());

        // Главное отличие от View: регистрируем click_handler.
        // Движок сохранит его в InstantiatedComponent.click_handler
        // и зарегистрирует в EventDispatcher при draw_and_collect.
        if let Some(ref handler) = props.on_click {
            // Нам нужно создать новый Arc каждый раз, т.к. on_click — &Box<dyn Fn()>
            // Решение: передать замыкание через двойной Arc
            // Сначала оборачиваем handler в общий указатель
            // Тут проблема: Box<dyn Fn()> не Clone.
            // Лучшее решение: пользователь передаёт Arc<dyn Fn()> напрямую.
            // Или: мы вызываем handler через unsafe ptr — нет, плохо.
            // Правильное решение: в ButtonProps хранить Arc<dyn Fn() + Send + Sync>.
        }
        // ↑ см. примечание ниже о ButtonProps::on_click
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
```

### Важное примечание о `on_click`

`Box<dyn Fn()>` не реализует `Clone`. При каждом `render()` ButtonProps создаётся заново
с новым замыканием. Это нормально — React тоже создаёт новые колбэки каждый рендер.

Проблема в том, что `ComponentUpdater::register_click_handler` принимает `impl Fn()`,
а нам нужно передать замыкание из ButtonProps. Решение — изменить ButtonProps:

```rust
// ВМЕСТО: pub on_click: Option<Box<dyn Fn() + Send + Sync>>
// ИСПОЛЬЗОВАТЬ:
pub on_click: Option<Arc<dyn Fn() + Send + Sync>>,
```

Тогда `update()` делает:
```rust
if let Some(handler) = props.on_click.clone() {
    updater.register_click_handler_arc(handler);
}
```

И добавить в `ComponentUpdater`:
```rust
pub fn register_click_handler_arc(&mut self, handler: Arc<dyn Fn() + Send + Sync>) {
    self.click_handler = Some(handler);
}
```

Пример создания кнопки:
```rust
let on_click = Arc::new(move || {
    // изменить состояние
});
element!(Button { on_click: Some(on_click.clone()) })
```

---

## ШАГ 10 — `src/ui/components/image.rs` — компонент изображения

```rust
use std::sync::Arc;

use taffy::prelude::*;

use crate::image::BmpImage;
use crate::ui::canvas::{DrawContext, UiRect};
use crate::ui::component::{Component, ComponentUpdater};

// ─── ImageProps ──────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct ImageProps {
    /// Изображение (Arc позволяет шарить между рендерами без копирования)
    pub image: Option<Arc<BmpImage>>,
    /// Переопределить ширину. None = взять из image.width
    pub width: Option<i32>,
    /// Переопределить высоту. None = взять из image.height
    pub height: Option<i32>,
}

// ─── Image ───────────────────────────────────────────────────────────────────

pub struct Image {
    image: Option<Arc<BmpImage>>,
}

impl Component for Image {
    type Props = ImageProps;

    fn new(props: &ImageProps) -> Self {
        Self { image: props.image.clone() }
    }

    fn update(&mut self, props: &ImageProps, updater: &mut ComponentUpdater) {
        self.image = props.image.clone();

        // Размер фиксированный (из изображения или из props)
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
        // Нет детей, нет measure_fn, нет click_handler
    }

    fn draw(&self, rect: UiRect, ctx: &mut DrawContext) {
        if let Some(ref img) = self.image {
            ctx.draw_image(img, rect.x, rect.y);
        }
    }
}
```

### 10.1 Создать `src/ui/components/mod.rs`

```rust
pub mod button;
pub mod image;
pub mod text;
pub mod view;

pub use button::{Button, ButtonProps};
pub use image::{Image, ImageProps};
pub use text::{Text, TextAlign, TextProps};
pub use view::{
    AlignItems, Border, Direction, EdgeInsets, JustifyContent, SizeValue, View, ViewProps,
};
```

---

## ШАГ 11 — `src/ui/mod.rs` — UiApp

UiApp — публичный фасад всей системы. Хранит layout engine, event dispatcher,
и корневой InstantiatedComponent между рендерами.

```rust
// src/ui/mod.rs (полная версия)

pub mod canvas;
pub mod component;
pub mod components;
pub mod element;
pub mod events;
pub mod layout;

pub use canvas::{Color, DrawContext, FontSize, TextStyle, UiRect};
pub use components::view::{AlignItems, Border, Direction, EdgeInsets, JustifyContent, SizeValue};
pub use events::UiEvent;

use embedded_graphics::pixelcolor::Gray4;

use crate::display::EmbeddedDisplay;
use crate::ui::component::InstantiatedComponent;
use crate::ui::element::AnyElement;
use crate::ui::events::EventDispatcher;
use crate::ui::layout::LayoutEngine;

// ─── UiApp ───────────────────────────────────────────────────────────────────

/// Главный объект UI-системы.
///
/// Создаётся один раз в начале программы и живёт всё время работы устройства.
///
/// Пример использования:
/// ```rust
/// let display = EmbeddedDisplay::new().unwrap();
/// let mut app = UiApp::new(display);
///
/// loop {
///     let ui = build_ui(&state);
///     app.render(ui);
///
///     if let Some(touch) = gt911.read_touch()? {
///         app.handle_event(UiEvent::Tap(touch));
///     }
///
///     FreeRtos::delay_ms(50);
/// }
/// ```
pub struct UiApp {
    engine: LayoutEngine,
    dispatcher: EventDispatcher,
    display: EmbeddedDisplay,
    /// Корень дерева компонентов.
    /// None до первого render(), Some после.
    root: Option<InstantiatedComponent>,
    /// Корневой NodeId в taffy (обёртка вокруг root).
    root_node: Option<taffy::NodeId>,
}

impl UiApp {
    pub fn new(display: EmbeddedDisplay) -> Self {
        Self {
            engine: LayoutEngine::new(),
            dispatcher: EventDispatcher::new(),
            display,
            root: None,
            root_node: None,
        }
    }

    /// Выполнить полный цикл рендера:
    /// 1. Reconcile дерево компонентов
    /// 2. Рассчитать layout через taffy
    /// 3. Нарисовать компоненты + собрать HitTarget-ы
    /// 4. Отправить буфер на дисплей
    pub fn render(&mut self, element: AnyElement) {
        // ── PHASE 1: UPDATE (reconciliation) ──────────────────────────────────

        // Первый рендер: создать корневой компонент
        if self.root.is_none() {
            let mut root = InstantiatedComponent::create_root(&element, &mut self.engine);
            root.update(&element, &mut self.engine);
            self.root_node = Some(root.node_id);
            self.root = Some(root);
        } else {
            // Последующие рендеры: обновить существующий корень
            let root = self.root.as_mut().unwrap();

            // Проверяем что тип корневого элемента не изменился
            // (смена типа корня — редкий edge case, проще запретить)
            debug_assert_eq!(
                root.type_id(),
                element.type_id(),
                "Тип корневого элемента не должен меняться между рендерами"
            );

            root.update(&element, &mut self.engine);
        }

        let root_node = self.root_node.unwrap();

        // ── PHASE 2: LAYOUT ───────────────────────────────────────────────────

        self.engine.compute(
            root_node,
            self.display.width() as f32,
            self.display.height() as f32,
        );

        // ── PHASE 3: DRAW + COLLECT HITS ──────────────────────────────────────

        // Очистить экран белым
        self.display.clear(Gray4::WHITE);

        // Очистить старые HitTarget-ы
        self.dispatcher.clear();

        // Рекурсивный обход: рисуем + собираем HitTargets
        {
            let mut ctx = DrawContext::new(&mut self.display);
            if let Some(ref root) = self.root {
                root.draw_and_collect(&self.engine, 0, 0, &mut ctx, &mut self.dispatcher);
            }
        }

        // ── PHASE 4: FLUSH ────────────────────────────────────────────────────

        self.display.flush().ok();
    }

    /// Передать touch-событие в систему.
    ///
    /// Возвращает `true` если событие было обработано каким-либо компонентом.
    ///
    /// Вызывать ПОСЛЕ render(), т.к. HitTarget-ы обновляются во время рендера.
    pub fn handle_event(&mut self, event: UiEvent) -> bool {
        self.dispatcher.dispatch(&event)
    }
}
```

### 11.1 Добавить вспомогательный метод в `InstantiatedComponent`

В `src/ui/component.rs` добавить:

```rust
impl InstantiatedComponent {
    // ... существующие методы ...

    /// Создать корневой компонент (для UiApp::render первого вызова).
    pub(crate) fn create_root(element: &AnyElement, engine: &mut LayoutEngine) -> Self {
        Self::create(element, engine)
    }

    /// Публичный доступ к type_id для проверки в UiApp.
    pub(crate) fn type_id(&self) -> TypeId {
        self.type_id
    }

    // node_id уже pub(crate) через поле
}
```

---

## ШАГ 12 — `src/lib.rs` — подключить модуль

В `src/lib.rs` добавить:

```rust
mod ui;

pub use ui::UiApp;
pub use ui::canvas::{Color, FontSize, TextStyle, UiRect};
pub use ui::components::view::{
    AlignItems, Border, Direction, EdgeInsets, JustifyContent, SizeValue, View, ViewProps,
};
pub use ui::components::text::{Text, TextAlign, TextProps};
pub use ui::components::button::{Button, ButtonProps};
pub use ui::components::image::{Image, ImageProps};
pub use ui::events::UiEvent;
```

---

## ШАГ 13 — Интеграция в `src/bin/ai-papers3.rs`

Это финальный шаг: заменяем текущий рисующий код на UI-систему.

### 13.1 Минимальный smoke-тест (нарисовать "Hello World")

```rust
// src/bin/ai-papers3.rs — добавить в начало main после инициализации display

use ai_papers3::{Color, Direction, EdgeInsets, FontSize, SizeValue, UiApp};
use ai_papers3::{Button, ButtonProps, Text, TextProps, View, ViewProps};
use ai_papers3::element; // наш macro

let mut app = UiApp::new(display); // display: EmbeddedDisplay

// Нарисовать простой экран
app.render(element!(View {
    width: SizeValue::Fixed(960),
    height: SizeValue::Fixed(540),
    background: Some(Color::WHITE),
    direction: Direction::Column,
    padding: EdgeInsets::all(20),
    gap: 16,
    children: vec![
        element!(Text {
            content: "Hello, PaperS3!".to_string(),
            font_size: FontSize::Large,
            color: Color::BLACK,
        }),
        element!(Text {
            content: "UI system works!".to_string(),
            font_size: FontSize::Medium,
            color: Color::DARK_GRAY,
        }),
    ],
}));
```

### 13.2 Тест кнопки

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

let button_pressed = Arc::new(AtomicBool::new(false));

loop {
    let pressed = button_pressed.load(Ordering::Relaxed);
    let button_pressed_clone = Arc::clone(&button_pressed);

    app.render(element!(View {
        width: SizeValue::Fixed(960),
        height: SizeValue::Fixed(540),
        background: Some(Color::WHITE),
        direction: Direction::Column,
        padding: EdgeInsets::all(40),
        gap: 20,
        children: vec![
            element!(Text {
                content: if pressed { "Button WAS pressed!" } else { "Press the button" }
                    .to_string(),
                font_size: FontSize::Large,
                color: Color::BLACK,
            }),
            element!(Button {
                background: Some(Color::LIGHT_GRAY),
                pressed_background: Some(Color::GRAY),
                border: Some(Border { color: Color::DARK_GRAY, thickness: 2 }),
                padding: EdgeInsets::all(16),
                on_click: Some(Arc::new(move || {
                    button_pressed_clone.store(true, Ordering::Relaxed);
                })),
                children: vec![
                    element!(Text {
                        content: "Click me".to_string(),
                        font_size: FontSize::Medium,
                        color: Color::BLACK,
                    })
                ],
            }),
        ],
    }));

    // Обработать touch
    if let Ok(Some(touch_point)) = gt911.read_touch() {
        app.handle_event(UiEvent::Tap(touch_point));
    }

    FreeRtos::delay_ms(100);
}
```

---

## Порядок выполнения и зависимости

```
ШАГ 1   Cargo.toml: добавить taffy
  │
  ▼
ШАГ 2   canvas.rs: Color, UiRect, DrawContext, FontSize, TextStyle
  │
  ▼
ШАГ 3   layout.rs: LayoutEngine, NodeContext, px/auto/length_px
  │
  ▼
ШАГ 4   element.rs: AnyElement, AnyComponent, ComponentHelper, element! macro
  │
  ▼
ШАГ 5   component.rs: Component trait, ComponentUpdater, InstantiatedComponent
  │
  ▼
ШАГ 6   events.rs: UiEvent, HitTarget, EventDispatcher
  │
  ├──────────────────────────────────────────┐
  ▼                                          ▼
ШАГ 7   view.rs                         ШАГ 8  text.rs
  │                                          │
  ▼                                          ▼
ШАГ 9   button.rs                       ШАГ 10 image.rs
  │
  ▼
ШАГ 10  components/mod.rs (re-exports)
  │
  ▼
ШАГ 11  ui/mod.rs: UiApp
  │
  ▼
ШАГ 12  lib.rs: pub use
  │
  ▼
ШАГ 13  bin/ai-papers3.rs: интеграция
```

---

## Типичные ошибки компилятора и как их исправить

### Ошибка: `the trait bound Box<dyn Fn()>: Clone is not satisfied`
**Причина:** `ViewProps::children: Vec<AnyElement>` пытается вызвать `.clone()`.
**Решение:** Не клонировать children. Создавать `ViewProps` заново в каждом `build_ui()`.

### Ошибка: `cannot move out of *props because it is behind a shared reference`
**Причина:** В `Component::update` Props передаётся как `&Self::Props`, а мы пытаемся
взять `on_click: Box<dyn Fn()>` из него.
**Решение:** Изменить `on_click` на `Option<Arc<dyn Fn() + Send + Sync>>` — Arc реализует Clone.

### Ошибка: `type annotations needed` в measure_fn
**Причина:** Rust не может вывести тип замыкания.
**Решение:** Явно указать тип: `updater.set_measure_fn(move |w: f32| -> (f32, f32) { ... })`

### Ошибка: `taffy: set_children failed` в runtime
**Причина:** Передаём NodeId который уже удалён из taffy или не принадлежит этому дереву.
**Решение:** Убедиться что `reconcile_children` удаляет старые NodeId ПОСЛЕ создания новых.

### Ошибка: Компоненты рисуются в (0, 0) вместо нужной позиции
**Причина:** Забыли прибавлять `parent_x` / `parent_y` к relative layout из taffy.
**Решение:** Проверить `draw_and_collect` — `abs_rect.x = parent_x + rel.x`.

---

## Чеклист проверки перед финальным тестом на железе

- [ ] `cargo build --release` проходит без ошибок
- [ ] `cargo clippy --all-targets -- -D warnings` без предупреждений
- [ ] Smoke-тест: "Hello World" появляется на экране
- [ ] View с `background` правильно закрашивает область
- [ ] Text с разными `FontSize` рисует текст разного размера
- [ ] Button отображает рамку и фон
- [ ] Touch-событие в область Button вызывает on_click
- [ ] Touch вне области Button ничего не вызывает
- [ ] Image рисует BmpImage в правильной позиции
- [ ] Вложенные View (Column → Row → Text) правильно позиционируются
- [ ] flex_grow: 1.0 растягивает View до края экрана
- [ ] gap между детьми View работает
- [ ] padding внутри View сдвигает детей
