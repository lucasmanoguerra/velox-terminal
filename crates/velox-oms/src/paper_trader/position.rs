//! Position tracking and P&L calculation for the paper trading engine.

use std::collections::HashMap;
use velox_core::{Position, Side};

use super::PaperTrader;

impl PaperTrader {
    /// Compute positions per symbol from fill history.
    ///
    /// Uses weighted-average cost basis and tracks realized P&L per symbol.
    pub fn positions(&self) -> Vec<Position> {
        let mut pos_map: HashMap<String, (f64, f64, f64)> = HashMap::new();

        for fill in self.order_manager.all_fills() {
            let entry = pos_map.entry(fill.symbol.clone()).or_insert((0.0, 0.0, 0.0));
            let (qty, avg, realized) = *entry;

            match fill.side {
                Side::Buy => {
                    if qty >= 0.0 {
                        let new_qty = qty + fill.quantity;
                        let new_avg = if qty > 0.0 {
                            ((avg * qty) + (fill.price * fill.quantity)) / new_qty
                        } else {
                            fill.price
                        };
                        *entry = (new_qty, new_avg, realized);
                    } else {
                        let abs_short = -qty;
                        let reduce = fill.quantity.min(abs_short);
                        let new_realized = realized + (reduce * (avg - fill.price));
                        let remaining = fill.quantity - reduce;
                        if remaining > 0.0 {
                            *entry = (remaining, fill.price, new_realized);
                        } else {
                            *entry = (qty + fill.quantity, avg, new_realized);
                        }
                    }
                }
                Side::Sell => {
                    if qty <= 0.0 {
                        let new_qty = qty - fill.quantity;
                        let new_avg = if qty < 0.0 {
                            ((avg * (-qty)) + (fill.price * fill.quantity)) / (-new_qty)
                        } else {
                            fill.price
                        };
                        *entry = (new_qty, new_avg, realized);
                    } else {
                        let reduce = fill.quantity.min(qty);
                        let new_realized = realized + (reduce * (fill.price - avg));
                        let remaining = fill.quantity - reduce;
                        if remaining > 0.0 {
                            *entry = (-remaining, fill.price, new_realized);
                        } else {
                            *entry = (qty - fill.quantity, avg, new_realized);
                        }
                    }
                }
            }
        }

        let current_price = |sym: &str| -> f64 {
            self.last_prices.get(sym).copied().unwrap_or(0.0)
        };

        pos_map
            .into_iter()
            .map(|(symbol, (qty, avg_entry, realized_pnl))| {
                let cp = current_price(&symbol);
                let unrealized = if qty > 0.0 {
                    qty * (cp - avg_entry)
                } else if qty < 0.0 {
                    (-qty) * (avg_entry - cp)
                } else {
                    0.0
                };

                Position {
                    symbol,
                    quantity: qty,
                    avg_entry_price: avg_entry,
                    current_price: cp,
                    unrealized_pnl: unrealized,
                    realized_pnl,
                }
            })
            .collect()
    }

    /// Account snapshot.
    pub fn account(&self) -> &velox_core::AccountInfo {
        &self.account
    }

    /// Recompute equity / buying power / P&L from current positions.
    pub fn update_account(&mut self) {
        let positions = self.positions();

        let total_unrealized: f64 = positions.iter().map(|p| p.unrealized_pnl).sum();
        let total_realized: f64 = positions.iter().map(|p| p.realized_pnl).sum();

        self.account.unrealized_pnl = total_unrealized;
        self.account.realized_pnl = total_realized;
        self.account.equity = (self.account.cash + total_unrealized + total_realized).max(0.0);
        self.account.buying_power = self.account.equity * 2.0;
    }
}
