//! atlas_toon — TOON (Token-Oriented Object Notation) renderer.
//!
//! TOON uses indentation + table headers instead of JSON braces/brackets.
//! It achieves ~40% fewer tokens than JSON while preserving the same data model.
//!
//! Syntax recap:
//!   key: value              -- scalar field
//!   key:                    -- object starts (indented block follows)
//!   items[N]{col1,col2}:   -- array table with N rows; rows are CSV lines
//!   items[N]:               -- simple string array

pub mod table;
pub mod render;

pub use render::{render_txfacts, render_tx_history, render_wallet_profile};

/// Content-type header value for TOON responses.
pub const CONTENT_TYPE: &str = "text/toon; charset=utf-8";

/// Trait implemented by types that can render themselves as TOON.
pub trait ToonRender {
    fn to_toon(&self) -> String;
}
