use std::any::{Any, TypeId};
use std::sync::Arc;

use taffy::Style;

use crate::ui::canvas::{DrawContext, UiRect};
use crate::ui::element::{AnyComponent, AnyElement, ComponentHelper};
use crate::ui::events::EventDispatcher;
use crate::ui::layout::LayoutEngine;

// ─── Component trait ──────────────────────────────────────────────────────────

pub trait Component: Any + Sized + 'static {
    type Props: Default + 'static;

    fn new(props: &Self::Props) -> Self;

    /// Обновить состояние и зарегистрировать layout в taffy.
    /// Props передаётся как &mut чтобы можно было забрать children через mem::take.
    fn update(&mut self, props: &mut Self::Props, updater: &mut ComponentUpdater);

    fn draw(&self, rect: UiRect, ctx: &mut DrawContext) {
        let _ = (rect, ctx);
    }

    /// Создать Props с дефолтными значениями. Используется в макросе `element!`
    /// вместо struct-литерала через ассоциированный тип (stable Rust совместимость).
    fn default_props() -> Self::Props {
        Self::Props::default()
    }
}

impl<C: Component + Any> AnyComponent for C {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ─── ComponentUpdater ─────────────────────────────────────────────────────────

pub struct ComponentUpdater<'a> {
    pub(crate) node_id: taffy::NodeId,
    pub(crate) engine: &'a mut LayoutEngine,
    pub(crate) pending_children: Vec<AnyElement>,
    pub(crate) click_handler: Option<Arc<dyn Fn() + Send + Sync>>,
    has_measure_fn: bool,
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

    /// Деструктурировать updater, вернуть (children, handler, engine).
    pub(crate) fn finish(
        self,
    ) -> (
        Vec<AnyElement>,
        Option<Arc<dyn Fn() + Send + Sync>>,
        &'a mut LayoutEngine,
    ) {
        (self.pending_children, self.click_handler, self.engine)
    }

    pub fn set_layout_style(&mut self, style: Style) {
        self.engine.set_style(self.node_id, style);
        if !self.has_measure_fn {
            self.engine.clear_measure_fn(self.node_id);
        }
    }

    pub fn set_children(&mut self, children: Vec<AnyElement>) {
        self.pending_children = children;
    }

    pub fn set_measure_fn(&mut self, f: impl Fn(f32) -> (f32, f32) + Send + 'static) {
        self.has_measure_fn = true;
        self.engine.set_measure_fn(self.node_id, f);
    }

    pub fn register_click_handler(&mut self, handler: Arc<dyn Fn() + Send + Sync>) {
        self.click_handler = Some(handler);
    }
}

// ─── InstantiatedComponent ────────────────────────────────────────────────────

pub struct InstantiatedComponent {
    pub(crate) node_id: taffy::NodeId,
    pub(crate) type_id: TypeId,
    component: Box<dyn AnyComponent>,
    pub(crate) children: Vec<InstantiatedComponent>,
    helper: &'static dyn ComponentHelper,
    click_handler: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl InstantiatedComponent {
    pub(crate) fn create(element: &AnyElement, engine: &mut LayoutEngine) -> Self {
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

    pub fn update(&mut self, element: &mut AnyElement, engine: &mut LayoutEngine) {
        let mut updater = ComponentUpdater::new(self.node_id, engine);
        self.helper
            .update(&mut *self.component, element.props_mut(), &mut updater);

        // Деструктурируем updater целиком — избегаем частичного перемещения полей
        let (incoming, click_handler, engine) = updater.finish();
        self.click_handler = click_handler;
        self.reconcile_children(incoming, engine);
    }

    /// Reconciliation по индексу + TypeId.
    fn reconcile_children(&mut self, incoming: Vec<AnyElement>, engine: &mut LayoutEngine) {
        let old_len = self.children.len();
        let new_len = incoming.len();

        // Переводим incoming во временный Vec<Option<AnyElement>> чтобы
        // брать элементы по одному без двойного заимствования
        let mut incoming: Vec<Option<AnyElement>> = incoming.into_iter().map(Some).collect();

        // Шаг 1: удалить лишние старые дети с конца
        while self.children.len() > new_len {
            let removed = self.children.pop().unwrap();
            removed.remove_from_engine(engine);
        }

        // Шаг 2: обработать позиции 0..old_len (существующие дети)
        for i in 0..old_len.min(new_len) {
            let mut child_element = incoming[i].take().unwrap();
            if self.children[i].type_id != child_element.type_id() {
                // Тип изменился — заменяем
                let new_child = InstantiatedComponent::create(&child_element, engine);
                let old = std::mem::replace(&mut self.children[i], new_child);
                old.remove_from_engine(engine);
            }
            self.children[i].update(&mut child_element, engine);
        }

        // Шаг 3: добавить новые дети (позиции old_len..new_len)
        for i in old_len..new_len {
            let mut child_element = incoming[i].take().unwrap();
            let mut new_child = InstantiatedComponent::create(&child_element, engine);
            new_child.update(&mut child_element, engine);
            self.children.push(new_child);
        }

        // Шаг 4: обновить список детей в taffy
        let child_ids: Vec<taffy::NodeId> = self.children.iter().map(|c| c.node_id).collect();
        engine.set_children(self.node_id, &child_ids);
    }

    pub fn draw_and_collect(
        &self,
        engine: &LayoutEngine,
        parent_x: i32,
        parent_y: i32,
        ctx: &mut DrawContext,
        dispatcher: &mut EventDispatcher,
    ) {
        let rel = engine.layout_relative(self.node_id);
        let abs = UiRect {
            x: parent_x + rel.x,
            y: parent_y + rel.y,
            width: rel.width,
            height: rel.height,
        };

        self.helper.draw(&*self.component, abs, ctx);

        if let Some(ref handler) = self.click_handler {
            let h = Arc::clone(handler);
            dispatcher.register(abs, Box::new(move || h()));
        }

        for child in &self.children {
            child.draw_and_collect(engine, abs.x, abs.y, ctx, dispatcher);
        }
    }

    fn remove_from_engine(self, engine: &mut LayoutEngine) {
        for child in self.children {
            child.remove_from_engine(engine);
        }
        engine.remove_node(self.node_id);
    }
}
