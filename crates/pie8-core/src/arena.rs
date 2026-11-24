//! Monotonic, alignment-aware storage for sample data.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArenaError {
    InvalidAlignment,
    Exhausted,
}

pub struct Arena<'a> {
    memory: &'a mut [u8],
    used: usize,
}

impl<'a> Arena<'a> {
    pub fn new(memory: &'a mut [u8]) -> Self {
        Self { memory, used: 0 }
    }
    pub fn capacity(&self) -> usize {
        self.memory.len()
    }
    pub fn used(&self) -> usize {
        self.used
    }
    pub fn remaining(&self) -> usize {
        self.capacity() - self.used
    }
    pub(crate) fn allocate_base(&self) -> *const u8 {
        self.memory.as_ptr()
    }
    pub(crate) fn as_slice(&self) -> &[u8] {
        self.memory
    }
    pub fn reset(&mut self) {
        self.memory[..self.used].fill(0);
        self.used = 0;
    }
    pub fn allocate(&mut self, size: usize, alignment: usize) -> Result<&mut [u8], ArenaError> {
        if alignment == 0 || !alignment.is_power_of_two() {
            return Err(ArenaError::InvalidAlignment);
        }
        let address = self.memory.as_ptr() as usize + self.used;
        let padding = address.wrapping_neg() & (alignment - 1);
        let start = self
            .used
            .checked_add(padding)
            .ok_or(ArenaError::Exhausted)?;
        let end = start.checked_add(size).ok_or(ArenaError::Exhausted)?;
        if end > self.memory.len() {
            return Err(ArenaError::Exhausted);
        }
        self.used = end;
        Ok(&mut self.memory[start..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alignment_exhaustion_and_reset() {
        let mut memory = [0xaa; 33];
        let mut arena = Arena::new(&mut memory);
        let item = arena.allocate(8, 16).unwrap();
        assert_eq!((item.as_ptr() as usize) & 15, 0);
        assert_eq!(arena.allocate(64, 1), Err(ArenaError::Exhausted));
        assert_eq!(arena.allocate(1, 3), Err(ArenaError::InvalidAlignment));
        arena.reset();
        assert_eq!(arena.used(), 0);
        assert!(arena.allocate(33, 1).is_ok());
    }
    #[test]
    fn zero_length_and_overflow_are_bounded() {
        let mut memory = [0; 8];
        let mut arena = Arena::new(&mut memory);
        assert!(arena.allocate(0, 8).is_ok());
        assert_eq!(arena.allocate(usize::MAX, 1), Err(ArenaError::Exhausted));
    }
}
