//! Ring buffer for orderbook history (time-travel feature)
//!
//! Enables the Track 2 visualizer to replay orderbook states.

use crate::orderbook::OrderbookSnapshot;
use std::collections::VecDeque;

/// Ring buffer for storing orderbook snapshots
///
/// Used by the WASM visualizer to enable time-travel/replay functionality.
#[derive(Debug, Clone)]
pub struct HistoryBuffer {
    /// Stored snapshots
    snapshots: VecDeque<TimestampedSnapshot>,
    /// Maximum number of snapshots to retain
    max_size: usize,
    /// Next sequence number
    next_sequence: u64,
}

/// Snapshot with sequence number for ordering
#[derive(Debug, Clone)]
pub struct TimestampedSnapshot {
    /// The orderbook snapshot
    pub snapshot: OrderbookSnapshot,
    /// Sequence number (monotonically increasing)
    pub sequence: u64,
    /// Optional timestamp in milliseconds (if provided by caller)
    pub timestamp_ms: Option<u64>,
}

impl HistoryBuffer {
    /// Create a new history buffer with specified capacity
    pub fn new(max_size: usize) -> Self {
        Self {
            snapshots: VecDeque::with_capacity(max_size.min(1024)),
            max_size,
            next_sequence: 0,
        }
    }

    /// Push a snapshot to the buffer
    ///
    /// If the buffer is full, the oldest snapshot is removed.
    pub fn push(&mut self, snapshot: OrderbookSnapshot) {
        self.push_with_timestamp(snapshot, None);
    }

    /// Push a snapshot with an optional timestamp
    pub fn push_with_timestamp(&mut self, snapshot: OrderbookSnapshot, timestamp_ms: Option<u64>) {
        let entry = TimestampedSnapshot {
            snapshot,
            sequence: self.next_sequence,
            timestamp_ms,
        };
        self.next_sequence += 1;

        if self.snapshots.len() >= self.max_size {
            self.snapshots.pop_front();
        }
        self.snapshots.push_back(entry);
    }

    /// Get the number of stored snapshots
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// Get the maximum capacity
    pub fn capacity(&self) -> usize {
        self.max_size
    }

    /// Get a snapshot by index (0 = oldest)
    pub fn get(&self, index: usize) -> Option<&TimestampedSnapshot> {
        self.snapshots.get(index)
    }

    /// Get the most recent snapshot
    pub fn latest(&self) -> Option<&TimestampedSnapshot> {
        self.snapshots.back()
    }

    /// Get the oldest snapshot
    pub fn oldest(&self) -> Option<&TimestampedSnapshot> {
        self.snapshots.front()
    }

    /// Get a snapshot by sequence number
    pub fn get_by_sequence(&self, sequence: u64) -> Option<&TimestampedSnapshot> {
        // Binary search since sequences are monotonically increasing
        self.snapshots
            .iter()
            .find(|s| s.sequence == sequence)
    }

    /// Get snapshots in a sequence range (inclusive)
    pub fn range(&self, start_seq: u64, end_seq: u64) -> Vec<&TimestampedSnapshot> {
        self.snapshots
            .iter()
            .filter(|s| s.sequence >= start_seq && s.sequence <= end_seq)
            .collect()
    }

    /// Get the current sequence number (next to be assigned)
    pub fn current_sequence(&self) -> u64 {
        self.next_sequence
    }

    /// Get the first sequence number in the buffer
    pub fn first_sequence(&self) -> Option<u64> {
        self.oldest().map(|s| s.sequence)
    }

    /// Get the last sequence number in the buffer
    pub fn last_sequence(&self) -> Option<u64> {
        self.latest().map(|s| s.sequence)
    }

    /// Clear all snapshots
    pub fn clear(&mut self) {
        self.snapshots.clear();
        // Don't reset sequence to maintain monotonicity
    }

    /// Iterator over all snapshots (oldest first)
    pub fn iter(&self) -> impl Iterator<Item = &TimestampedSnapshot> {
        self.snapshots.iter()
    }
}

impl Default for HistoryBuffer {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_types::Level;
    use rust_decimal_macros::dec;

    fn make_snapshot(bid: f64, ask: f64) -> OrderbookSnapshot {
        OrderbookSnapshot {
            symbol: "BTC/USD".to_string(),
            bids: vec![Level::from_f64(bid, 1.0)],
            asks: vec![Level::from_f64(ask, 1.0)],
            checksum: 0,
            state: crate::orderbook::OrderbookState::Synced,
        }
    }

    #[test]
    fn test_push_and_get() {
        let mut buffer = HistoryBuffer::new(10);
        assert!(buffer.is_empty());

        buffer.push(make_snapshot(100.0, 101.0));
        assert_eq!(buffer.len(), 1);

        let entry = buffer.get(0).unwrap();
        assert_eq!(entry.sequence, 0);
        assert_eq!(entry.snapshot.bids[0].price, dec!(100));
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let mut buffer = HistoryBuffer::new(3);

        buffer.push(make_snapshot(100.0, 101.0));
        buffer.push(make_snapshot(101.0, 102.0));
        buffer.push(make_snapshot(102.0, 103.0));
        assert_eq!(buffer.len(), 3);

        // This should evict the first snapshot
        buffer.push(make_snapshot(103.0, 104.0));
        assert_eq!(buffer.len(), 3);

        // First entry should now be sequence 1, not 0
        assert_eq!(buffer.oldest().unwrap().sequence, 1);
        assert_eq!(buffer.latest().unwrap().sequence, 3);
    }

    #[test]
    fn test_get_by_sequence() {
        let mut buffer = HistoryBuffer::new(10);

        buffer.push(make_snapshot(100.0, 101.0));
        buffer.push(make_snapshot(101.0, 102.0));
        buffer.push(make_snapshot(102.0, 103.0));

        let entry = buffer.get_by_sequence(1).unwrap();
        assert_eq!(entry.snapshot.bids[0].price, dec!(101));

        assert!(buffer.get_by_sequence(100).is_none());
    }

    #[test]
    fn test_range() {
        let mut buffer = HistoryBuffer::new(10);

        for i in 0..5 {
            buffer.push(make_snapshot(100.0 + i as f64, 101.0 + i as f64));
        }

        let range = buffer.range(1, 3);
        assert_eq!(range.len(), 3);
        assert_eq!(range[0].sequence, 1);
        assert_eq!(range[2].sequence, 3);
    }

    #[test]
    fn test_clear_preserves_sequence() {
        let mut buffer = HistoryBuffer::new(10);

        buffer.push(make_snapshot(100.0, 101.0));
        buffer.push(make_snapshot(101.0, 102.0));
        assert_eq!(buffer.current_sequence(), 2);

        buffer.clear();
        assert!(buffer.is_empty());

        // Sequence should continue from where it left off
        buffer.push(make_snapshot(102.0, 103.0));
        assert_eq!(buffer.latest().unwrap().sequence, 2);
    }
}
