//! Formatting helpers for Binance API request parameters.
//!
//! Binance requires numeric values (quantity, price) as decimal strings
//! with specific precision. These helpers ensure correct formatting
//! without trailing zeros or unnecessary decimal points.

/// Format a quantity as a string with up to 8 decimal places.
///
/// Trailing zeros are stripped to produce concise, Binance-compatible
/// decimal strings:
///
/// ```
/// # use velox_exchange::binance_broker::format::format_quantity;
/// assert_eq!(format_quantity(1.0), "1");
/// assert_eq!(format_quantity(0.01), "0.01");
/// assert_eq!(format_quantity(0.00123456), "0.00123456");
/// ```
pub fn format_quantity(qty: f64) -> String {
    if qty.fract() < 1e-8 {
        format!("{:.0}", qty)
    } else {
        let s = format!("{:.8}", qty);
        s.trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// Format a price as a string with up to 8 decimal places.
///
/// Delegates to [`format_quantity`] since both use the same logic.
pub fn format_price(price: f64) -> String {
    format_quantity(price)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_quantity_integer() {
        assert_eq!(format_quantity(1.0), "1");
        assert_eq!(format_quantity(100.0), "100");
        assert_eq!(format_quantity(0.0), "0");
    }

    #[test]
    fn test_format_quantity_decimal() {
        assert_eq!(format_quantity(0.01), "0.01");
        assert_eq!(format_quantity(0.001), "0.001");
        assert_eq!(format_quantity(1.5), "1.5");
    }

    #[test]
    fn test_format_quantity_precision() {
        let qty = format_quantity(0.00123456);
        assert_eq!(qty, "0.00123456");
    }

    #[test]
    fn test_format_price() {
        assert_eq!(format_price(45000.0), "45000");
        assert_eq!(format_price(45000.12), "45000.12");
        assert_eq!(format_price(0.01), "0.01");
    }
}
