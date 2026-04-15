#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, Copy)]
pub enum TouchEvent {
    Touch(TouchPoint),
    Slide { from: TouchPoint, to: TouchPoint },
}

#[derive(Debug, Default)]
pub struct TouchTracker {
    last_point: Option<TouchPoint>,
    min_delta: u16,
}

impl TouchTracker {
    pub fn new(min_delta: u16) -> Self {
        Self {
            last_point: None,
            min_delta,
        }
    }

    pub fn on_sample(&mut self, point: Option<TouchPoint>) -> Option<TouchEvent> {
        match (self.last_point, point) {
            (None, Some(current)) => {
                self.last_point = Some(current);
                Some(TouchEvent::Touch(current))
            }
            (Some(previous), Some(current)) => {
                if !self.moved_enough(previous, current) {
                    return None;
                }
                self.last_point = Some(current);
                Some(TouchEvent::Slide {
                    from: previous,
                    to: current,
                })
            }
            (_, None) => {
                self.last_point = None;
                None
            }
        }
    }

    fn moved_enough(&self, from: TouchPoint, to: TouchPoint) -> bool {
        let dx = from.x.abs_diff(to.x);
        let dy = from.y.abs_diff(to.y);
        dx >= self.min_delta || dy >= self.min_delta
    }
}
