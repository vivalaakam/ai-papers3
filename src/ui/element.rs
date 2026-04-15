use std::any::{Any, TypeId};
use std::marker::PhantomData;

use crate::ui::canvas::{DrawContext, UiRect};
use crate::ui::component::ComponentUpdater;

// ─── AnyComponent ─────────────────────────────────────────────────────────────

/// Трейт-объект для Component. Позволяет хранить Box<dyn AnyComponent>
/// без знания конкретного типа.
pub trait AnyComponent: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// ─── ComponentHelper ─────────────────────────────────────────────────────────

/// Статический vtable для операций над конкретным типом компонента.
/// Один экземпляр на каждый уникальный тип (создаётся через Box::leak).
pub trait ComponentHelper: Send + Sync + 'static {
    fn component_type_id(&self) -> TypeId;

    /// Создать новый экземпляр компонента из props.
    fn create(&self, props: &dyn Any) -> Box<dyn AnyComponent>;

    /// Обновить существующий компонент.
    fn update(
        &self,
        component: &mut dyn AnyComponent,
        props: &mut dyn Any,
        updater: &mut ComponentUpdater,
    );

    /// Нарисовать компонент.
    fn draw(&self, component: &dyn AnyComponent, rect: UiRect, ctx: &mut DrawContext);
}

// ─── ComponentHelperImpl<C> ───────────────────────────────────────────────────

pub(crate) struct ComponentHelperImpl<C: crate::ui::component::Component + 'static>(
    pub PhantomData<C>,
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
            .expect("ComponentHelperImpl::create: wrong props type");
        Box::new(C::new(props))
    }

    fn update(
        &self,
        component: &mut dyn AnyComponent,
        props: &mut dyn Any,
        updater: &mut ComponentUpdater,
    ) {
        let component = component
            .as_any_mut()
            .downcast_mut::<C>()
            .expect("ComponentHelperImpl::update: wrong component type");
        let props = props
            .downcast_mut::<C::Props>()
            .expect("ComponentHelperImpl::update: wrong props type");
        component.update(props, updater);
    }

    fn draw(&self, component: &dyn AnyComponent, rect: UiRect, ctx: &mut DrawContext) {
        let component = component
            .as_any()
            .downcast_ref::<C>()
            .expect("ComponentHelperImpl::draw: wrong component type");
        component.draw(rect, ctx);
    }
}

// ─── AnyElement ───────────────────────────────────────────────────────────────

/// Type-erased элемент (описание того, что нужно отрисовать).
pub struct AnyElement {
    type_id: TypeId,
    props: Box<dyn Any>,
    pub(crate) helper: &'static dyn ComponentHelper,
}

impl AnyElement {
    /// Создать AnyElement для компонента типа C.
    pub fn new<C: crate::ui::component::Component + 'static>(props: C::Props) -> Self {
        // Box::leak создаёт статическую ссылку — вызывается один раз
        // на каждый уникальный тип C (у нас 4-5 типов = пренебрежимо мало).
        let helper: &'static dyn ComponentHelper =
            Box::leak(Box::new(ComponentHelperImpl::<C>(PhantomData)));
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

    pub fn props_mut(&mut self) -> &mut dyn Any {
        self.props.as_mut()
    }
}

// ─── element! macro ───────────────────────────────────────────────────────────

/// Создать AnyElement с удобным синтаксисом.
///
/// ```rust
/// element!(View { direction: Direction::Column, background: Some(Color::WHITE) })
/// element!(Text { content: "Hello".into(), font_size: FontSize::Large })
/// ```
///
/// Поля, не указанные явно, берутся из Default::default().
///
/// Реализация через последовательное присвоение полей избегает экспериментальной
/// фичи `more_qualified_paths` (struct-литерал через ассоциированный тип).
#[macro_export]
macro_rules! element {
    ($type:ty { $($field:ident: $value:expr),* $(,)? }) => {{
        let mut _props = <$type as $crate::ui::component::Component>::default_props();
        $( _props.$field = $value; )*
        $crate::ui::element::AnyElement::new::<$type>(_props)
    }};
}
