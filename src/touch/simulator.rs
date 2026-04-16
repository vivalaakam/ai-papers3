use alloc::vec::Vec;

use super::{TouchPoint, TouchSource};

pub struct SimulatorTouch {
    pending: Vec<TouchPoint>,
}

impl SimulatorTouch {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    pub fn push(&mut self, point: TouchPoint) {
        self.pending.push(point);
    }

    pub fn extend(&mut self, points: impl IntoIterator<Item = TouchPoint>) {
        self.pending.extend(points);
    }
}

impl TouchSource for SimulatorTouch {
    fn poll(&mut self) -> Option<TouchPoint> {
        self.pending.pop()
    }
}