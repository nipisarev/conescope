use std::collections::VecDeque;

const MAX_CHUNKS: usize = 500;

/// Ring buffer of raw terminal output chunks.
#[derive(Debug, Clone)]
pub struct TerminalHistory {
    chunks: VecDeque<Vec<u8>>,
}

impl Default for TerminalHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalHistory {
    #[must_use]
    pub fn new() -> Self {
        Self {
            chunks: VecDeque::with_capacity(MAX_CHUNKS),
        }
    }

    pub fn push(&mut self, data: Vec<u8>) {
        if self.chunks.len() >= MAX_CHUNKS {
            self.chunks.pop_front();
        }
        self.chunks.push_back(data);
    }

    #[must_use]
    pub fn chunks(&self) -> &VecDeque<Vec<u8>> {
        &self.chunks
    }

    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let total: usize = self.chunks.iter().map(Vec::len).sum();
        let mut buf = Vec::with_capacity(total);
        for chunk in &self.chunks {
            buf.extend_from_slice(chunk);
        }
        buf
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_retrieve() {
        let mut h = TerminalHistory::new();
        h.push(b"hello ".to_vec());
        h.push(b"world".to_vec());
        assert_eq!(h.len(), 2);
        assert_eq!(h.to_bytes(), b"hello world");
    }

    #[test]
    fn eviction_at_capacity() {
        let mut h = TerminalHistory::new();
        for i in 0..600_u32 {
            h.push(i.to_le_bytes().to_vec());
        }
        assert_eq!(h.len(), MAX_CHUNKS);
        // First chunk should be 100 (600 - 500)
        let first = u32::from_le_bytes(h.chunks()[0].as_slice().try_into().unwrap());
        assert_eq!(first, 100);
    }

    #[test]
    fn clear_empties() {
        let mut h = TerminalHistory::new();
        h.push(b"data".to_vec());
        h.clear();
        assert!(h.is_empty());
        assert!(h.to_bytes().is_empty());
    }
}
