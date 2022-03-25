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

    pub fn try_peek(&self) -> Result<T, ()> {
        if !self.empty {
            Ok(self.buffer[self.first])
        } else {
            Err(())
        }
    }

    pub fn try_push(&mut self, data: T) -> Result<(), ()> {
        if self.empty || self.first != self.last {
            self.buffer[self.last] = data;
            self.last += 1;

            if self.last >= self.size {
                self.last = 0;
            }

            self.empty = false;
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn try_pop(&mut self) -> Result<T, ()> {
        if !self.empty {
            let value = self.buffer[self.first];
            self.first += 1;

            if self.first >= self.size {
                self.first = 0;
            }
            if self.first == self.last {
                self.empty = true;
            }

            Ok(value)
        } else {
            Err(())
        }
    }
}
