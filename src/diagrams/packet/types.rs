//! Packet diagram types
//!
//! Packet diagrams show bit-level packet/protocol structure with labeled fields.

/// A block in a packet diagram
#[derive(Debug, Clone, PartialEq)]
pub struct PacketBlock {
    /// Start bit position (0-indexed)
    pub start: usize,
    /// End bit position (inclusive)
    pub end: usize,
    /// Number of bits in this block
    pub bits: usize,
    /// Label for this block
    pub label: String,
}

/// A row of packet blocks (max 32 bits per row by default)
pub type PacketWord = Vec<PacketBlock>;

/// Error types for packet diagram operations
#[derive(Debug, Clone, PartialEq)]
pub enum PacketError {
    /// Block is not contiguous with previous block
    NotContiguous {
        start: usize,
        end: usize,
        expected: usize,
    },
    /// End bit is less than start bit
    InvalidRange { start: usize, end: usize },
    /// Bit count is zero
    ZeroBitField { count: usize },
}

impl std::fmt::Display for PacketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PacketError::NotContiguous {
                start,
                end,
                expected,
            } => {
                write!(
                    f,
                    "Packet block {} - {} is not contiguous. It should start from {}.",
                    start, end, expected
                )
            }
            PacketError::InvalidRange { start, end } => {
                write!(
                    f,
                    "Packet block {} - {} is invalid. End must be greater than start.",
                    start, end
                )
            }
            PacketError::ZeroBitField { count } => {
                write!(
                    f,
                    "Packet block {} is invalid. Cannot have a zero bit field.",
                    count
                )
            }
        }
    }
}

impl std::error::Error for PacketError {}

/// The Packet diagram database
#[derive(Debug, Clone, Default)]
pub struct PacketDb {
    /// Title of the diagram
    title: String,
    /// Accessibility title
    acc_title: String,
    /// Accessibility description
    acc_description: String,
    /// Packet rows (each row contains blocks that fit within bits_per_row)
    packet: Vec<PacketWord>,
    /// Number of bits per row (default 32)
    bits_per_row: usize,
    /// Current bit position for tracking contiguity
    current_bit: usize,
}

impl PacketDb {
    /// Create a new empty PacketDb
    pub fn new() -> Self {
        Self {
            title: String::new(),
            acc_title: String::new(),
            acc_description: String::new(),
            packet: Vec::new(),
            bits_per_row: 32,
            current_bit: 0,
        }
    }

    /// Clear all data
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// Set the diagram title
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    /// Get the diagram title
    pub fn get_title(&self) -> &str {
        &self.title
    }

    /// Set accessibility title
    pub fn set_acc_title(&mut self, title: &str) {
        self.acc_title = title.to_string();
    }

    /// Get accessibility title
    pub fn get_acc_title(&self) -> &str {
        &self.acc_title
    }

    /// Set accessibility description
    pub fn set_acc_description(&mut self, desc: &str) {
        self.acc_description = desc.to_string();
    }

    /// Get accessibility description
    pub fn get_acc_description(&self) -> &str {
        &self.acc_description
    }

    /// Set bits per row
    pub fn set_bits_per_row(&mut self, bits: usize) {
        self.bits_per_row = bits;
    }

    /// Add a block with explicit start and end positions
    pub fn add_block(&mut self, start: usize, end: usize, label: &str) -> Result<(), PacketError> {
        // Validate end >= start
        if end < start {
            return Err(PacketError::InvalidRange { start, end });
        }

        // Validate contiguity
        if start != self.current_bit {
            return Err(PacketError::NotContiguous {
                start,
                end,
                expected: self.current_bit,
            });
        }

        let total_bits = end - start + 1;
        self.add_block_internal(start, end, total_bits, label);
        Ok(())
    }

    /// Add a block with a bit count (e.g., +8 for 8 bits)
    pub fn add_block_by_count(&mut self, bit_count: usize, label: &str) -> Result<(), PacketError> {
        if bit_count == 0 {
            return Err(PacketError::ZeroBitField { count: 0 });
        }

        let start = self.current_bit;
        let end = start + bit_count - 1;
        self.add_block_internal(start, end, bit_count, label);
        Ok(())
    }

    /// Add a single-bit block
    pub fn add_single_bit(&mut self, position: usize, label: &str) -> Result<(), PacketError> {
        self.add_block(position, position, label)
    }

    /// Internal method to add blocks, handling row splitting
    fn add_block_internal(&mut self, start: usize, end: usize, total_bits: usize, label: &str) {
        let mut current_start = start;
        let mut remaining_bits = total_bits;

        while remaining_bits > 0 {
            let row_index = current_start / self.bits_per_row;
            let position_in_row = current_start % self.bits_per_row;
            let bits_until_row_end = self.bits_per_row - position_in_row;
            let bits_in_this_row = remaining_bits.min(bits_until_row_end);
            let current_end = current_start + bits_in_this_row - 1;

            // Ensure we have enough rows
            while self.packet.len() <= row_index {
                self.packet.push(Vec::new());
            }

            let block = PacketBlock {
                start: current_start,
                end: current_end,
                bits: bits_in_this_row,
                label: label.to_string(),
            };

            self.packet[row_index].push(block);

            current_start += bits_in_this_row;
            remaining_bits -= bits_in_this_row;
        }

        self.current_bit = end + 1;
    }

    /// Push a complete word (row) of blocks
    pub fn push_word(&mut self, word: PacketWord) {
        if !word.is_empty() {
            self.packet.push(word);
        }
    }

    /// Get all packet rows
    pub fn get_packet(&self) -> &[PacketWord] {
        &self.packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_packet() {
        let db = PacketDb::new();
        assert!(db.get_packet().is_empty());
    }

    #[test]
    fn test_set_title() {
        let mut db = PacketDb::new();
        db.set_title("Packet diagram");
        assert_eq!(db.get_title(), "Packet diagram");
    }

    #[test]
    fn test_set_acc_title() {
        let mut db = PacketDb::new();
        db.set_acc_title("Packet accTitle");
        assert_eq!(db.get_acc_title(), "Packet accTitle");
    }

    #[test]
    fn test_set_acc_description() {
        let mut db = PacketDb::new();
        db.set_acc_description("Packet accDescription");
        assert_eq!(db.get_acc_description(), "Packet accDescription");
    }

    #[test]
    fn test_add_block_range() {
        let mut db = PacketDb::new();
        db.add_block(0, 10, "test").unwrap();

        let packet = db.get_packet();
        assert_eq!(packet.len(), 1);
        assert_eq!(packet[0].len(), 1);
        assert_eq!(packet[0][0].start, 0);
        assert_eq!(packet[0][0].end, 10);
        assert_eq!(packet[0][0].bits, 11);
        assert_eq!(packet[0][0].label, "test");
    }

    #[test]
    fn test_single_bit() {
        let mut db = PacketDb::new();
        db.add_block(0, 10, "test").unwrap();
        db.add_single_bit(11, "single").unwrap();

        let packet = db.get_packet();
        assert_eq!(packet[0].len(), 2);
        assert_eq!(packet[0][1].start, 11);
        assert_eq!(packet[0][1].end, 11);
        assert_eq!(packet[0][1].bits, 1);
        assert_eq!(packet[0][1].label, "single");
    }

    #[test]
    fn test_bit_count() {
        let mut db = PacketDb::new();
        db.add_block_by_count(8, "byte").unwrap();
        db.add_block_by_count(16, "word").unwrap();

        let packet = db.get_packet();
        assert_eq!(packet.len(), 1);
        assert_eq!(packet[0].len(), 2);

        assert_eq!(packet[0][0].start, 0);
        assert_eq!(packet[0][0].end, 7);
        assert_eq!(packet[0][0].bits, 8);
        assert_eq!(packet[0][0].label, "byte");

        assert_eq!(packet[0][1].start, 8);
        assert_eq!(packet[0][1].end, 23);
        assert_eq!(packet[0][1].bits, 16);
        assert_eq!(packet[0][1].label, "word");
    }

    #[test]
    fn test_split_into_multiple_rows() {
        let mut db = PacketDb::new();
        db.add_block(0, 10, "test").unwrap();
        db.add_block(11, 90, "multiple").unwrap();

        let packet = db.get_packet();
        // Row 0: bits 0-31 (blocks: 0-10 test, 11-31 multiple)
        // Row 1: bits 32-63 (blocks: 32-63 multiple)
        // Row 2: bits 64-90 (blocks: 64-90 multiple)
        assert_eq!(packet.len(), 3);

        // First row
        assert_eq!(packet[0][0].start, 0);
        assert_eq!(packet[0][0].end, 10);
        assert_eq!(packet[0][0].bits, 11);
        assert_eq!(packet[0][0].label, "test");

        assert_eq!(packet[0][1].start, 11);
        assert_eq!(packet[0][1].end, 31);
        assert_eq!(packet[0][1].bits, 21); // 31-11+1 = 21 bits
        assert_eq!(packet[0][1].label, "multiple");

        // Second row
        assert_eq!(packet[1][0].start, 32);
        assert_eq!(packet[1][0].end, 63);
        assert_eq!(packet[1][0].bits, 32); // 63-32+1 = 32 bits (full row)
        assert_eq!(packet[1][0].label, "multiple");

        // Third row
        assert_eq!(packet[2][0].start, 64);
        assert_eq!(packet[2][0].end, 90);
        assert_eq!(packet[2][0].bits, 27); // 90-64+1 = 27 bits
        assert_eq!(packet[2][0].label, "multiple");
    }

    #[test]
    fn test_split_at_exact_row_boundary() {
        let mut db = PacketDb::new();
        db.add_block(0, 16, "test").unwrap();
        db.add_block(17, 63, "multiple").unwrap();

        let packet = db.get_packet();
        assert_eq!(packet.len(), 2);

        // First row
        assert_eq!(packet[0][0].start, 0);
        assert_eq!(packet[0][0].end, 16);
        assert_eq!(packet[0][0].bits, 17);

        assert_eq!(packet[0][1].start, 17);
        assert_eq!(packet[0][1].end, 31);
        assert_eq!(packet[0][1].bits, 15); // 31-17+1 = 15 bits
        assert_eq!(packet[0][1].label, "multiple");

        // Second row (exactly fills the row)
        assert_eq!(packet[1][0].start, 32);
        assert_eq!(packet[1][0].end, 63);
        assert_eq!(packet[1][0].bits, 32); // 63-32+1 = 32 bits (full row)
        assert_eq!(packet[1][0].label, "multiple");
    }

    #[test]
    fn test_not_contiguous_error() {
        let mut db = PacketDb::new();
        db.add_block(0, 16, "test").unwrap();
        let result = db.add_block(18, 20, "error");

        assert!(result.is_err());
        if let Err(PacketError::NotContiguous {
            start,
            end,
            expected,
        }) = result
        {
            assert_eq!(start, 18);
            assert_eq!(end, 20);
            assert_eq!(expected, 17);
        } else {
            panic!("Expected NotContiguous error");
        }
    }

    #[test]
    fn test_not_contiguous_with_bit_count() {
        let mut db = PacketDb::new();
        db.add_block_by_count(16, "test").unwrap();
        let result = db.add_block(18, 20, "error");

        assert!(result.is_err());
        if let Err(PacketError::NotContiguous {
            start,
            end,
            expected,
        }) = result
        {
            assert_eq!(start, 18);
            assert_eq!(end, 20);
            assert_eq!(expected, 16);
        } else {
            panic!("Expected NotContiguous error");
        }
    }

    #[test]
    fn test_not_contiguous_single_bit() {
        let mut db = PacketDb::new();
        db.add_block(0, 16, "test").unwrap();
        let result = db.add_single_bit(18, "error");

        assert!(result.is_err());
        if let Err(PacketError::NotContiguous {
            start,
            end,
            expected,
        }) = result
        {
            assert_eq!(start, 18);
            assert_eq!(end, 18);
            assert_eq!(expected, 17);
        } else {
            panic!("Expected NotContiguous error");
        }
    }

    #[test]
    fn test_invalid_range_error() {
        let mut db = PacketDb::new();
        db.add_block(0, 16, "test").unwrap();
        let result = db.add_block(25, 20, "error");

        assert!(result.is_err());
        if let Err(PacketError::InvalidRange { start, end }) = result {
            assert_eq!(start, 25);
            assert_eq!(end, 20);
        } else {
            panic!("Expected InvalidRange error");
        }
    }

    #[test]
    fn test_zero_bit_count_error() {
        let mut db = PacketDb::new();
        let result = db.add_block_by_count(0, "test");

        assert!(result.is_err());
        if let Err(PacketError::ZeroBitField { count }) = result {
            assert_eq!(count, 0);
        } else {
            panic!("Expected ZeroBitField error");
        }
    }

    #[test]
    fn test_clear() {
        let mut db = PacketDb::new();
        db.set_title("Test");
        db.add_block(0, 10, "test").unwrap();
        db.clear();

        assert_eq!(db.get_title(), "");
        assert!(db.get_packet().is_empty());
    }

    #[test]
    fn test_error_display() {
        let err = PacketError::NotContiguous {
            start: 18,
            end: 20,
            expected: 17,
        };
        assert_eq!(
            err.to_string(),
            "Packet block 18 - 20 is not contiguous. It should start from 17."
        );

        let err = PacketError::InvalidRange { start: 25, end: 20 };
        assert_eq!(
            err.to_string(),
            "Packet block 25 - 20 is invalid. End must be greater than start."
        );

        let err = PacketError::ZeroBitField { count: 0 };
        assert_eq!(
            err.to_string(),
            "Packet block 0 is invalid. Cannot have a zero bit field."
        );
    }
}
