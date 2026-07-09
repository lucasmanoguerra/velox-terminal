//! Lock-free ring buffer for market data events.

use std::sync::atomic::{AtomicU64, Ordering};

use velox_core::Quote;
use velox_core::Tick;

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

    /// Pop up to `max` events from the ring buffer into `buf`.
    ///
    /// Returns the number of events popped. This is more efficient than calling
    /// [`pop`](Self::pop) in a loop because it issues only two atomic loads and one
    /// atomic store for the entire batch, rather than 3 atomic ops per event.
    ///
    /// # SPSC Safety
    ///
    /// The producer cannot overwrite slots we are reading because it checks
    /// `write - read < capacity` before writing, which prevents wrap-around
    /// past our observed `read` position.
    pub fn pop_n(&self, buf: &mut Vec<MarketEvent>, max: usize) -> usize {
        let read = self.read_index.load(Ordering::Acquire);
        let write = self.write_index.load(Ordering::Acquire);

        let available = (write - read) as usize;
        if available == 0 || max == 0 {
            return 0;
        }

        let count = available.min(max);
        let mask = self.capacity as u64 - 1;

        // Read each slot. SAFETY: SPSC design guarantees no concurrent write to
        // slots in the range [read, read + count) — the producer's write position
        // is ahead of this range and won't wrap around until write - read < capacity.
        for i in 0..count {
            let idx = ((read + i as u64) & mask) as usize;
            // SAFETY: We are the sole consumer, no other thread reads this slot.
            let slot = unsafe { &mut *self.buffer[idx].get() };
            if let Some(event) = slot.take() {
                buf.push(event);
            }
        }

        // Release: make our reads visible to the producer so it knows it can
        // reuse these slots.
        self.read_index.store(read + count as u64, Ordering::Release);
        count
    }

    /// Current length of the buffer.
    pub fn len(&self) -> usize {
        (self.write_index.load(Ordering::Acquire) - self.read_index.load(Ordering::Acquire))
            as usize
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
        assert!(buf.push(MarketEvent::Tick(tick)));
        assert!(buf.push(MarketEvent::Tick(tick)));
        assert!(buf.push(MarketEvent::Tick(tick)));
        assert!(buf.push(MarketEvent::Tick(tick)));
        assert!(!buf.push(MarketEvent::Tick(tick))); // 5th should fail (buffer full)
    }

    // ── pop_n tests ───────────────────────────────────────────────

    fn make_tick(price: f64) -> Tick {
        Tick {
            symbol: *b"BTCUSD\0\0",
            price,
            volume: 1.0,
            timestamp: Utc::now(),
            conditions: 0,
        }
    }

    #[test]
    fn test_pop_n_returns_up_to_max() {
        let buf = RingBuffer::new(1024);
        for i in 0..10 {
            assert!(buf.push(MarketEvent::Tick(make_tick(100.0 + i as f64))));
        }
        let mut batch = Vec::new();
        let count = buf.pop_n(&mut batch, 5);
        assert_eq!(count, 5);
        assert_eq!(batch.len(), 5);
        assert_eq!(buf.len(), 5);
    }

    #[test]
    fn test_pop_n_all_events() {
        let buf = RingBuffer::new(1024);
        for i in 0..5 {
            assert!(buf.push(MarketEvent::Tick(make_tick(100.0 + i as f64))));
        }
        let mut batch = Vec::new();
        let count = buf.pop_n(&mut batch, 100); // max > available
        assert_eq!(count, 5);
        assert_eq!(batch.len(), 5);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_pop_n_preserves_order() {
        let buf = RingBuffer::new(1024);
        for i in 0..5 {
            assert!(buf.push(MarketEvent::Tick(make_tick(100.0 + i as f64))));
        }
        let mut batch = Vec::new();
        buf.pop_n(&mut batch, 5);
        for (i, event) in batch.iter().enumerate() {
            if let MarketEvent::Tick(tick) = event {
                assert!((tick.price - (100.0 + i as f64)).abs() < 1e-6);
            } else {
                panic!("Expected Tick");
            }
        }
    }

    #[test]
    fn test_pop_n_empty_buffer() {
        let buf = RingBuffer::new(1024);
        let mut batch = Vec::new();
        let count = buf.pop_n(&mut batch, 10);
        assert_eq!(count, 0);
        assert!(batch.is_empty());
    }

    #[test]
    fn test_pop_n_exact_wrap_around() {
        // Use a small buffer and push to force wraparound
        let buf = RingBuffer::new(4);
        // Fill buffer
        for i in 0..4 {
            assert!(buf.push(MarketEvent::Tick(make_tick(100.0 + i as f64))));
        }
        // Pop 3 → read_index = 3
        let mut batch = Vec::new();
        assert_eq!(buf.pop_n(&mut batch, 3), 3);
        assert_eq!(buf.len(), 1);
        // Push 3 more — these wrap around
        for i in 0..3 {
            assert!(buf.push(MarketEvent::Tick(make_tick(200.0 + i as f64))));
        }
        // Pop 4 (all remaining) — reads from indices 3, 0, 1, 2
        batch.clear();
        assert_eq!(buf.pop_n(&mut batch, 4), 4);
        assert_eq!(batch.len(), 4);
        // First event should be the one that was at index 3 (old data)
        if let MarketEvent::Tick(tick) = &batch[0] {
            assert!((tick.price - 103.0).abs() < 1e-6);
        } else {
            panic!("Expected Tick");
        }
        // The next three are the wrapped-around ones
        for (i, event) in batch.iter().enumerate().skip(1) {
            if let MarketEvent::Tick(tick) = event {
                let expected_price = 199.0 + i as f64;
                assert!((tick.price - expected_price).abs() < 1e-6);
            } else {
                panic!("Expected Tick");
            }
        }
        assert!(buf.is_empty());
    }

    #[test]
    fn test_pop_n_zero_max() {
        let buf = RingBuffer::new(1024);
        assert!(buf.push(MarketEvent::Tick(make_tick(100.0))));
        let mut batch = Vec::new();
        let count = buf.pop_n(&mut batch, 0);
        assert_eq!(count, 0);
        assert!(batch.is_empty());
        assert_eq!(buf.len(), 1);
    }
}
