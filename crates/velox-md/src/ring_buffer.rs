//! Lock-free ring buffer for market data events.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use velox_core::Tick;
use velox_core::Quote;

/// A tick or quote event from the market data feed.
#[derive(Debug, Clone)]
pub enum MarketEvent {
    Tick(Tick),
    Quote(Quote),
}

/// Lock-free single-producer single-consumer ring buffer.
pub struct RingBuffer {
    buffer: Box<[std::cell::UnsafeCell<Option<MarketEvent>>]>,
    capacity: usize,
    write_index: AtomicU64,
    read_index: AtomicU64,
}

// SAFETY: SPSC design guarantees no concurrent access to the same slot.
unsafe impl Sync for RingBuffer {}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.next_power_of_two();
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(std::cell::UnsafeCell::new(None));
        }
        Self {
            buffer: buffer.into_boxed_slice(),
            capacity,
            write_index: AtomicU64::new(0),
            read_index: AtomicU64::new(0),
        }
    }

    /// Push an event into the ring buffer. Returns false if full.
    pub fn push(&self, event: MarketEvent) -> bool {
        let write = self.write_index.load(Ordering::Acquire);
        let read = self.read_index.load(Ordering::Acquire);

        if write - read >= self.capacity as u64 {
            return false; // buffer full
        }

        let idx = (write & (self.capacity as u64 - 1)) as usize;
        unsafe {
            *self.buffer[idx].get() = Some(event);
        }
        self.write_index.store(write + 1, Ordering::Release);
        true
    }

    /// Pop an event from the ring buffer. Returns None if empty.
    pub fn pop(&self) -> Option<MarketEvent> {
        let read = self.read_index.load(Ordering::Acquire);
        let write = self.write_index.load(Ordering::Acquire);

        if read == write {
            return None; // empty
        }

        let idx = (read & (self.capacity as u64 - 1)) as usize;
        let event = unsafe { (*self.buffer[idx].get()).take() };
        self.read_index.store(read + 1, Ordering::Release);
        event
    }

    /// Current length of the buffer.
    pub fn len(&self) -> usize {
        (self.write_index.load(Ordering::Acquire) - self.read_index.load(Ordering::Acquire)) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_push_pop() {
        let buf = RingBuffer::new(1024);
        let tick = Tick {
            symbol: *b"ES      ",
            price: 4500.25,
            volume: 100.0,
            timestamp: Utc::now(),
            conditions: 0,
        };
        assert!(buf.push(MarketEvent::Tick(tick)));
        assert!(!buf.is_empty());
        assert_eq!(buf.len(), 1);

        let popped = buf.pop();
        assert!(popped.is_some());
        assert!(buf.is_empty());
    }

    #[test]
    fn test_buffer_full() {
        let buf = RingBuffer::new(4);
        let tick = Tick {
            symbol: *b"ES      ",
            price: 4500.0,
            volume: 1.0,
            timestamp: Utc::now(),
            conditions: 0,
        };
        assert!(buf.push(MarketEvent::Tick(tick.clone())));
        assert!(buf.push(MarketEvent::Tick(tick.clone())));
        assert!(buf.push(MarketEvent::Tick(tick.clone())));
        assert!(buf.push(MarketEvent::Tick(tick.clone())));
        assert!(!buf.push(MarketEvent::Tick(tick.clone()))); // 5th should fail (buffer full)
    }
}
