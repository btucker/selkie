//! Packet diagram support
//!
//! This module provides data structures for packet diagrams.
//! Packet diagrams show bit-level packet/protocol structure with labeled fields.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::{PacketBlock, PacketDb, PacketError, PacketWord};
