use core::ops::{Index, IndexMut};

pub struct RingBuffer<'a, T: Copy> {
    buffer: &'a mut [T],
    first: usize,
    last: usize,
    size: usize,
    empty: bool,
}

impl<'a, T: Copy> RingBuffer<'a, T> {
    pub fn new(buffer: &'a mut [T]) -> Self {
        let size = buffer.len();
        Self {
            buffer: buffer,
            first: 0,
            last: 0,
            size,
            empty: true
        }
    }

    pub fn len(&self) -> usize {
        if self.empty {
            0
        } else if self.first < self.last {
            self.last - self.first
        } else {
            self.last + self.size - self.first
        }
    }

    pub fn capacity(&self) -> usize {
        self.size
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.wrap_index(index).map(|x| &self.buffer[x])
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.wrap_index(index).map(|x| &mut self.buffer[x])
    }

    fn wrap_index(&self, index: usize) -> Option<usize> {
        let p = self.first + index;
        if index >= self.len() {
            None
        } else if p < self.size {
            Some(p)
        } else {
            Some(p - self.size)
        }
    }

    pub fn try_peek(&self) -> Option<T> {
        if !self.empty {
            Some(self.buffer[self.first])
        } else {
            None
        }
    }

    pub fn peek(&self) -> T {
        self.try_peek().expect("Out of bound access")
    }

    pub fn try_push(&mut self, data: T) -> Option<()> {
        if self.empty || self.first != self.last {
            self.buffer[self.last] = data;
            self.last += 1;

            if self.last >= self.size {
                self.last = 0;
            }

            self.empty = false;
            Some(())
        } else {
            None
        }
    }

    pub fn push(&mut self, data: T) {
        self.try_push(data).expect("Out of bound access")
    }

    pub fn push_force(&mut self, data: T) -> bool {
        let mut forced = false;

        self.buffer[self.last] = data;
        if !self.empty && self.first == self.last {
            forced = true;

            self.first += 1;
            if self.first >= self.size {
                self.first = 0;
            }
        }

        self.last += 1;
        if self.last >= self.size {
            self.last = 0;
        }

        self.empty = false;

        forced
    }

    pub fn insert_force(&mut self, pos: usize, data: T) -> bool {
        let len = self.len();
        if pos > len {
            panic!("Out of bound access")
        }

        if self.empty || pos == len {
            self.push_force(data)
        } else if self.first != self.last {
            self.push(data);
            for i in (pos..len).rev() {
                self[i + 1] = self[i];
            }
            self[pos] = data;
            false
        } else {
            for i in 0..pos {
                self[i] = self[i + 1];
            }
            self[pos] = data;
            true
        }
    }

    pub fn try_pop(&mut self) -> Option<T> {
        if !self.empty {
            let value = self.buffer[self.first];
            self.first += 1;

            if self.first >= self.size {
                self.first = 0;
            }
            if self.first == self.last {
                self.empty = true;
            }

            Some(value)
        } else {
            None
        }
    }

    pub fn pop(&mut self) -> T {
        self.try_pop().expect("Out of bound access")
    }
}

impl<'a, T: Copy> Index<usize> for RingBuffer<'a, T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        self.get(index).expect("Out of bound access")
    }
}

impl<'a, T: Copy> IndexMut<usize> for RingBuffer<'a, T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        self.get_mut(index).expect("Out of bound access")
    }
}
