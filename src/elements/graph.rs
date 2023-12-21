use std::collections::VecDeque;

pub struct Graph(VecDeque<f32>);

impl Graph {
    pub fn new(size: usize) -> Self {
        let mut inner = VecDeque::new();
        inner.resize(size, 0.0);
        Self(inner)
    }

    pub fn push(&mut self, value: f32) {
        let Self(inner) = self;
        let size = inner.len();
        inner.truncate(size.saturating_sub(1)); // Truncate to shift values along.
        inner.push_front(value)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &f32> {
        self.0.iter()
    }

    pub fn inner_mut(&mut self) -> &mut VecDeque<f32> {
        &mut self.0
    }

    pub fn min(&self) -> f32 {
        if self.is_empty() {
            return Default::default();
        }
        // TODO: Check out the perf on this .copied()
        self.iter().copied().fold(f32::INFINITY, f32::min)
    }

    pub fn max(&self) -> f32 {
        if self.is_empty() {
            return Default::default();
        }
        self.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    }
}

impl From<VecDeque<f32>> for Graph {
    fn from(deque: VecDeque<f32>) -> Self {
        Self(deque)
    }
}
