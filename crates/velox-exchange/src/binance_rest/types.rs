//! Binance REST API response types.
//!
//! These types define the JSON response structures from Binance's REST API
//! endpoints. All types use `#[serde(rename_all = "camelCase")]` to match
//! Binance's JSON field naming convention.

use serde::{Deserialize, Serialize};

/// Binance API base URL (production).
pub const BINANCE_REST_URL: &str = "https://api.binance.com";

/// Binance API base URL (testnet).
pub const BINANCE_TESTNET_REST_URL: &str = "https://testnet.binance.vision";

/// Default receive window in milliseconds.
pub const DEFAULT_RECV_WINDOW: u64 = 5000;

/// A Binance API error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceApiError {
    /// Error code.
    pub code: i64,
    /// Error message.
    pub msg: String,
}

/// Response from `GET /api/v3/account`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceAccountInfo {
    #[serde(default)]
    pub maker_commission: i64,
    #[serde(default)]
    pub taker_commission: i64,
    #[serde(default)]
    pub buyer_commission: i64,
    #[serde(default)]
    pub seller_commission: i64,
    #[serde(default)]
    pub can_trade: bool,
    #[serde(default)]
    pub can_withdraw: bool,
    #[serde(default)]
    pub can_deposit: bool,
    #[serde(default)]
    pub account_type: Option<String>,
    #[serde(default)]
    pub balances: Vec<BinanceBalance>,
    #[serde(default)]
    pub permissions: Vec<String>,
}

/// An asset balance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceBalance {
    pub asset: String,
    #[serde(default)]
    pub free: String,
    #[serde(default)]
    pub locked: String,
}

/// Response from `GET /api/v3/exchangeInfo`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceExchangeInfo {
    #[serde(default)]
    pub timezone: String,
    #[serde(default)]
    pub server_time: i64,
    #[serde(default)]
    pub symbols: Vec<BinanceSymbolInfo>,
}

/// Symbol info from exchange info.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceSymbolInfo {
    pub symbol: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub base_asset: String,
    #[serde(default)]
    pub quote_asset: String,
    #[serde(default)]
    pub base_asset_precision: i64,
    #[serde(default)]
    pub quote_precision: i64,
    #[serde(default)]
    pub order_types: Vec<String>,
    #[serde(default)]
    pub iceberg_allowed: bool,
    #[serde(default)]
    pub is_spot_trading_allowed: bool,
    #[serde(default)]
    pub filters: Vec<serde_json::Value>,
}

/// Response from `POST /api/v3/order`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceOrderResponse {
    pub symbol: String,
    pub order_id: i64,
    pub client_order_id: Option<String>,
    pub transact_time: Option<i64>,
    pub price: String,
    pub orig_qty: String,
    pub executed_qty: String,
    pub cummulative_quote_qty: String,
    pub status: String,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub side: String,
    pub fills: Option<Vec<BinanceOrderFill>>,
}

/// A fill within a trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceOrderFill {
    pub price: String,
    pub qty: String,
    pub commission: String,
    pub commission_asset: String,
    pub trade_id: i64,
}

/// Response from `DELETE /api/v3/order` (cancel).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceCancelledOrder {
    pub symbol: String,
    pub orig_client_order_id: String,
    pub order_id: i64,
    pub client_order_id: String,
    pub price: String,
    pub orig_qty: String,
    pub executed_qty: String,
    pub cummulative_quote_qty: String,
    pub status: String,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub side: String,
}

/// A trade from `GET /api/v3/myTrades`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceTrade {
    pub symbol: String,
    pub id: i64,
    pub order_id: i64,
    pub price: String,
    pub qty: String,
    pub commission: String,
    pub commission_asset: String,
    pub time: i64,
    pub is_buyer: bool,
    pub is_maker: bool,
    pub is_best_match: bool,
}
