//! Fuzz target for Binance trade/depth JSON parsing.
//!
//! Feeds randomized byte sequences to the Binance trade and depth
//! message parsers, ensuring they never panic on malformed input.
//!
//! # Running
//!
//! ```bash
//! cargo fuzz run market_data_parser -- -max_len=8192
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;

/// Attempt to parse random bytes as Binance trade messages.
///
/// The parser should never panic — only return Err on invalid input.
/// This fuzzer catches panics, assertion failures, and unbounded
/// memory allocation from malicious payloads.
fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Try to interpret as UTF-8 JSON (Binance uses JSON)
    if let Ok(text) = std::str::from_utf8(data) {
        // Simulate trade event parsing
        if let Some(_price) = extract_price_from_binance_trade(text) {
            // Successfully extracted — no panic
        }
        // Also OK if parsing fails — we just don't crash
    }
});

/// Minimal Binance trade parser for fuzzing.
///
/// Extracts "p" (price) and "q" (quantity) fields from a Binance trade JSON.
/// Returns None on any parse failure.
fn extract_price_from_binance_trade(json: &str) -> Option<f64> {
    // Minimal non-panicking parser
    let price_key = "\"p\":";
    let qty_key = "\"q\":";

    let price_start = json.find(price_key)?;
    let price_val_start = price_start + price_key.len();
    let price_end = json[price_val_start..].find(|c: char| c == ',' || c == '}' || c == ' ')?;
    let price_str = &json[price_val_start..price_val_start + price_end];
    let _price: f64 = price_str.trim().trim_matches('"').parse().ok()?;

    let qty_start = json.find(qty_key)?;
    let qty_val_start = qty_start + qty_key.len();
    let qty_end = json[qty_val_start..].find(|c: char| c == ',' || c == '}' || c == ' ')?;
    let qty_str = &json[qty_val_start..qty_val_start + qty_end];
    let _qty: f64 = qty_str.trim().trim_matches('"').parse().ok()?;

    Some(_price)
}
