pub use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
pub struct Ring<T> {
    values: Vec<T>,
    next: AtomicUsize,
}

impl<T> Ring<T> {
    pub fn new(values: Vec<T>) -> Self {
        assert!(values.len() > 0, "Ring<T> doesn't work with empty Vec<T>");
        Self {
            values,
            next: AtomicUsize::new(0),
        }
    }
}

impl<T> Ring<T> {
    #[inline]
    fn next_index(&self) -> usize {
        if self.values.len() == 1 {
            0
        } else {
            self.next.fetch_add(1, Ordering::Relaxed) % self.values.len()
        }
    }

    #[inline]
    pub fn next_as_ref(&self) -> &T {
        &self.values[self.next_index()]
    }
}

impl<T: Copy> Ring<T> {
    #[inline]
    pub fn next_as_owned(&self) -> T {
        *self.next_as_ref()
    }
}

impl<T: Clone> Ring<T> {
    #[inline]
    pub fn next_as_cloned(&self) -> T {
        self.next_as_ref().clone()
    }
}
