use std::collections::VecDeque;

pub struct RingBuffer<T> {
    data: VecDeque<T>,
}

impl<T> RingBuffer<T> {
    pub fn with_size(size: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(size),
        }
    }

    pub fn push(&mut self, val: T) {
        if self.data.len() + 1 >= self.data.capacity() {
            self.data.pop_back();
        }
        self.data.push_front(val);
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        self.data.get(idx)
    }
}
