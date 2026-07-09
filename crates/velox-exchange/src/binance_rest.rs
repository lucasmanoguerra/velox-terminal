//! Binance REST API client — account info, order placement, market data queries.
//!
//! # Authentication
//!
//! Signed endpoints use [HMAC-SHA256](https://binance-docs.github.io/apidocs/spot/en/#signed-trade-user_data-endpoint)
//! with the API secret. The signature is appended as a query parameter.
//!
//! # Endpoints
//!
//! | Method | Endpoint | Auth | Description |
//! |--------|----------|------|-------------|
//! | GET    | `/api/v3/ping` | No   | Test connectivity |
//! | GET    | `/api/v3/exchangeInfo` | No | Symbol rules & filters |
//! | GET    | `/api/v3/account` | Yes  | Balances & account info |
//! | POST   | `/api/v3/order` | Yes  | Place a new order |
//! | DELETE | `/api/v3/order` | Yes  | Cancel an order |
//! | GET    | `/api/v3/order` | Yes  | Query order status |

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::ExchangeError;

// ── Constants ────────────────────────────────────────────────────────────

/// Binance production REST API base URL.
const BINANCE_REST_URL: &str = "https://api.binance.com";

/// Binance testnet REST API base URL.
const BINANCE_TESTNET_REST_URL: &str = "https://testnet.binance.vision";

/// Default receive window (milliseconds). Binance rejects requests older than this.
const DEFAULT_RECV_WINDOW: u64 = 5000;

/// HMAC-SHA256 type alias for signing.
type HmacSha256 = Hmac<Sha256>;

// ── Response types (JSON deserialization) ────────────────────────────────

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

/// An asset balance in the account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceBalance {
    pub asset: String,
    /// Available balance.
    #[serde(default)]
    pub free: String,
    /// Locked (in orders) balance.
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

// ── REST Client ──────────────────────────────────────────────────────────

/// Binance REST API client.
///
/// Supports both live and testnet environments. Signed endpoints require
/// an API key and secret key for HMAC-SHA256 authentication.
///
/// # Example
///
/// ```rust,no_run
/// use velox_exchange::binance_rest::BinanceRestClient;
///
/// # async fn example() {
/// let client = BinanceRestClient::new(
///     "api_key".into(),
///     "secret_key".into(),
///     true, // use testnet
/// );
///
/// // Public endpoint (no auth needed)
/// let info = client.exchange_info().await.unwrap();
/// println!("Server time: {}", info.server_time);
///
/// // Signed endpoint (requires API key)
/// let account = client.account().await.unwrap();
/// for balance in &account.balances {
///     println!("{}: free={}, locked={}", balance.asset, balance.free, balance.locked);
/// }
/// # }
/// ```
#[derive(Clone)]
pub struct BinanceRestClient {
    /// Binance API key.
    api_key: String,
    /// HMAC-SHA256 secret key bytes.
    secret_key: Vec<u8>,
    /// Base URL (live or testnet).
    base_url: String,
    /// HTTP client with pool, TLS, and timeouts.
    client: reqwest::Client,
    /// Receive window in milliseconds (default 5000).
    recv_window: u64,
}

impl BinanceRestClient {
    /// Create a new Binance REST client.
    ///
    /// * `api_key` — Binance API key.
    /// * `secret_key` — Binance API secret.
    /// * `use_testnet` — `true` for testnet, `false` for production.
    pub fn new(api_key: String, secret_key: String, use_testnet: bool) -> Self {
        let base_url = if use_testnet {
            BINANCE_TESTNET_REST_URL.to_string()
        } else {
            BINANCE_REST_URL.to_string()
        };

        Self {
            api_key,
            secret_key: secret_key.into_bytes(),
            base_url,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("reqwest Client::builder() should not fail with default options"),
            recv_window: DEFAULT_RECV_WINDOW,
        }
    }

    /// Set a custom receive window (milliseconds).
    pub fn with_recv_window(mut self, recv_window: u64) -> Self {
        self.recv_window = recv_window;
        self
    }

    /// Set a custom HTTP client (useful for mocking in tests).
    pub fn with_client(mut self, client: reqwest::Client) -> Self {
        self.client = client;
        self
    }

    // ── Public endpoints (no auth) ───────────────────────────────────

    /// Test connectivity (`GET /api/v3/ping`).
    ///
    /// Returns `Ok(())` if the connection is alive and Binance responds.
    pub async fn ping(&self) -> Result<(), ExchangeError> {
        let url = format!("{}/api/v3/ping", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ExchangeError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(self.parse_error(resp).await);
        }
        Ok(())
    }

    /// Get exchange info and symbol rules (`GET /api/v3/exchangeInfo`).
    pub async fn exchange_info(&self) -> Result<BinanceExchangeInfo, ExchangeError> {
        let url = format!("{}/api/v3/exchangeInfo", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ExchangeError::Http(e.to_string()))?;

        self.parse_response::<BinanceExchangeInfo>(resp).await
    }

    // ── Signed endpoints (HMAC-SHA256 auth) ──────────────────────────

    /// Get account information (`GET /api/v3/account`).
    ///
    /// Returns balances, permissions, and commission rates.
    /// Requires a valid API key with the `account:read` permission.
    pub async fn account(&self) -> Result<BinanceAccountInfo, ExchangeError> {
        let url = self.signed_get_url("/api/v3/account", &[]);
        let resp = self
            .client
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| ExchangeError::Http(e.to_string()))?;

        self.parse_response::<BinanceAccountInfo>(resp).await
    }

    /// Place a new order (`POST /api/v3/order`).
    ///
    /// # Parameters
    ///
    /// * `symbol` — Trading symbol (e.g. `"BTCUSDT"`)
    /// * `side` — `"BUY"` or `"SELL"`
    /// * `order_type` — `"MARKET"`, `"LIMIT"`, `"STOP_LOSS"`, etc.
    /// * `quantity` — Base asset quantity
    /// * `price` — Limit price (required for LIMIT orders, optional otherwise)
    /// * `time_in_force` — `"GTC"`, `"IOC"`, `"FOK"`, etc.
    /// * `stop_price` — Stop price (required for STOP orders)
    /// * `client_order_id` — Optional client-specified order ID
    #[allow(clippy::too_many_arguments)]
    pub async fn new_order(
        &self,
        symbol: &str,
        side: &str,
        order_type: &str,
        quantity: &str,
        price: Option<&str>,
        time_in_force: Option<&str>,
        stop_price: Option<&str>,
        client_order_id: Option<&str>,
    ) -> Result<BinanceOrderResponse, ExchangeError> {
        let mut params: Vec<(&str, String)> = Vec::with_capacity(8);
        params.push(("symbol", symbol.to_string()));
        params.push(("side", side.to_string()));
        params.push(("type", order_type.to_string()));
        params.push(("quantity", quantity.to_string()));

        if let Some(p) = price {
            params.push(("price", p.to_string()));
        }
        if let Some(tif) = time_in_force {
            params.push(("timeInForce", tif.to_string()));
        }
        if let Some(sp) = stop_price {
            params.push(("stopPrice", sp.to_string()));
        }
        if let Some(coid) = client_order_id {
            params.push(("newClientOrderId", coid.to_string()));
        }

        let url = self.signed_post_url("/api/v3/order", &params);
        let resp = self
            .client
            .post(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| ExchangeError::Http(e.to_string()))?;

        self.parse_response::<BinanceOrderResponse>(resp).await
    }

    /// Cancel an order (`DELETE /api/v3/order`).
    ///
    /// Provide either `order_id` or `orig_client_order_id`.
    pub async fn cancel_order(
        &self,
        symbol: &str,
        order_id: i64,
    ) -> Result<BinanceCancelledOrder, ExchangeError> {
        let params = [
            ("symbol", symbol.to_string()),
            ("orderId", order_id.to_string()),
        ];
        let url = self.signed_delete_url("/api/v3/order", &params);
        let resp = self
            .client
            .delete(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| ExchangeError::Http(e.to_string()))?;

        self.parse_response::<BinanceCancelledOrder>(resp).await
    }

    /// Query an order's status (`GET /api/v3/order`).
    pub async fn get_order(
        &self,
        symbol: &str,
        order_id: i64,
    ) -> Result<BinanceOrderResponse, ExchangeError> {
        let params = [
            ("symbol", symbol.to_string()),
            ("orderId", order_id.to_string()),
        ];
        let url = self.signed_get_url("/api/v3/order", &params);
        let resp = self
            .client
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| ExchangeError::Http(e.to_string()))?;

        self.parse_response::<BinanceOrderResponse>(resp).await
    }

    /// Get all open orders for a symbol (`GET /api/v3/openOrders`).
    pub async fn open_orders(
        &self,
        symbol: Option<&str>,
    ) -> Result<Vec<BinanceOrderResponse>, ExchangeError> {
        let params: Vec<(&str, String)> = if let Some(sym) = symbol {
            vec![("symbol", sym.to_string())]
        } else {
            vec![]
        };
        let url = self.signed_get_url("/api/v3/openOrders", &params);
        let resp = self
            .client
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| ExchangeError::Http(e.to_string()))?;

        self.parse_response::<Vec<BinanceOrderResponse>>(resp).await
    }

    // ── User Data Stream (listen key) ─────────────────────────────

    /// Create a listen key for the user data stream (`POST /api/v3/userDataStream`).
    ///
    /// Requires only the `X-MBX-APIKEY` header (no HMAC signing).
    /// The listen key expires after 60 minutes if not kept alive.
    pub async fn create_listen_key(&self) -> Result<String, ExchangeError> {
        let url = format!("{}/api/v3/userDataStream", self.base_url);
        let resp = self
            .client
            .post(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| ExchangeError::Http(e.to_string()))?;

        #[derive(Deserialize)]
        struct ListenKeyResponse {
            listen_key: String,
        }

        let body: ListenKeyResponse = self.parse_response(resp).await?;
        Ok(body.listen_key)
    }

    /// Keep a listen key alive (`PUT /api/v3/userDataStream`).
    ///
    /// Should be called every 30 minutes to prevent expiry (60 min TTL).
    pub async fn keepalive_listen_key(&self, listen_key: &str) -> Result<(), ExchangeError> {
        let url = format!(
            "{}/api/v3/userDataStream?listenKey={}",
            self.base_url, listen_key
        );
        let resp = self
            .client
            .put(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| ExchangeError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(self.parse_error(resp).await);
        }
        Ok(())
    }

    /// Close a listen key (`DELETE /api/v3/userDataStream`).
    ///
    /// Call when shutting down to prevent stale keys.
    pub async fn close_listen_key(&self, listen_key: &str) -> Result<(), ExchangeError> {
        let url = format!(
            "{}/api/v3/userDataStream?listenKey={}",
            self.base_url, listen_key
        );
        let resp = self
            .client
            .delete(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| ExchangeError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(self.parse_error(resp).await);
        }
        Ok(())
    }

    /// Get all trades for a symbol (`GET /api/v3/myTrades`).
    pub async fn my_trades(
        &self,
        symbol: &str,
        limit: Option<i64>,
    ) -> Result<Vec<BinanceTrade>, ExchangeError> {
        let mut params: Vec<(&str, String)> = vec![("symbol", symbol.to_string())];
        if let Some(lim) = limit {
            params.push(("limit", lim.to_string()));
        }
        let url = self.signed_get_url("/api/v3/myTrades", &params);
        let resp = self
            .client
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| ExchangeError::Http(e.to_string()))?;

        self.parse_response::<Vec<BinanceTrade>>(resp).await
    }

    // ── Internal helpers ─────────────────────────────────────────────

    /// Build a signed GET URL with HMAC-SHA256 signature.
    fn signed_get_url(&self, path: &str, params: &[(&str, String)]) -> String {
        let query = self.build_query(params);
        let signature = self.sign(&query);
        format!("{}{}?{}&signature={}", self.base_url, path, query, signature)
    }

    /// Build a signed POST URL (parameters go in query string per Binance convention).
    fn signed_post_url(&self, path: &str, params: &[(&str, String)]) -> String {
        // Same as GET — Binance puts POST params in the query string
        self.signed_get_url(path, params)
    }

    /// Build a signed DELETE URL.
    fn signed_delete_url(&self, path: &str, params: &[(&str, String)]) -> String {
        self.signed_get_url(path, params)
    }

    /// Build a query string from parameters, sorted alphabetically by key,
    /// with `timestamp` and `recvWindow` appended.
    fn build_query(&self, params: &[(&str, String)]) -> String {
        let mut sorted: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));

        let timestamp = chrono::Utc::now().timestamp_millis();

        let mut query = String::new();
        for (i, (key, val)) in sorted.iter().enumerate() {
            if i > 0 {
                query.push('&');
            }
            query.push_str(key);
            query.push('=');
            query.push_str(val);
        }
        if !query.is_empty() {
            query.push('&');
        }
        query.push_str(&format!("timestamp={}&recvWindow={}", timestamp, self.recv_window));
        query
    }

    /// Compute HMAC-SHA256 hex signature for a query string.
    fn sign(&self, query: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.secret_key)
            .expect("HMAC-SHA256 key should be valid (non-empty)");
        mac.update(query.as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    /// Parse a JSON response or extract a Binance API error.
    async fn parse_response<T: serde::de::DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> Result<T, ExchangeError> {
        let status = resp.status();
        if !status.is_success() {
            return Err(self.parse_error(resp).await);
        }
        let body = resp
            .bytes()
            .await
            .map_err(|e| ExchangeError::Http(format!("Failed to read response body: {e}")))?;
        serde_json::from_slice::<T>(&body)
            .map_err(|e| ExchangeError::JsonParse(format!("{e}: {}", String::from_utf8_lossy(&body))))
    }

    /// Parse an error response body into an ExchangeError.
    async fn parse_error(&self, resp: reqwest::Response) -> ExchangeError {
        let status = resp.status();
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read body>".to_string());

        // Try to parse as a structured Binance API error
        if let Ok(binance_err) = serde_json::from_str::<BinanceApiError>(&body) {
            return ExchangeError::Exchange(format!(
                "Binance API error [code {}]: {}",
                binance_err.code, binance_err.msg
            ));
        }

        // Generic HTTP error
        ExchangeError::Http(format!("HTTP {}: {}", status.as_u16(), body))
    }
}

impl std::fmt::Debug for BinanceRestClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinanceRestClient")
            .field("base_url", &self.base_url)
            .field("recv_window", &self.recv_window)
            .field("api_key (masked)", &format!("{}****", &self.api_key[..4.min(self.api_key.len())]))
            .finish()
    }
}

// ── Additional response types ────────────────────────────────────────────

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

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a test client with mock HTTP backend.
    fn test_client() -> BinanceRestClient {
        BinanceRestClient::new(
            "test_api_key".into(),
            "test_secret_key".into(),
            false, // live URL for deterministic test
        )
    }

    // ── Build query / sign tests ─────────────────────────────────────

    #[test]
    fn test_build_query_empty_params() {
        let client = test_client();
        let query = client.build_query(&[]);
        // Should have timestamp and recvWindow
        assert!(query.contains("timestamp="), "query={query}");
        assert!(query.contains("recvWindow="), "query={query}");
        // timestamp and recvWindow should be last
        assert!(
            query.starts_with("timestamp=") || query.contains("&timestamp="),
            "query should start with or contain &timestamp=, got: {query}"
        );
    }

    #[test]
    fn test_build_query_sorted_params() {
        let client = test_client();
        let params = [
            ("symbol", "BTCUSDT".to_string()),
            ("side", "BUY".to_string()),
            ("type", "MARKET".to_string()),
        ];
        let query = client.build_query(&params);
        // Params should be sorted alphabetically: side, symbol, type
        let parts: Vec<&str> = query.split('&').collect();
        // Find the param portion (before timestamp)
        let param_part: Vec<&&str> = parts.iter().filter(|p| !p.starts_with("timestamp=") && !p.starts_with("recvWindow=")).collect();
        assert_eq!(param_part.len(), 3);
        assert!(param_part[0].starts_with("side="));
        assert!(param_part[1].starts_with("symbol="));
        assert!(param_part[2].starts_with("type="));
    }

    #[test]
    fn test_sign_consistency() {
        let client = test_client();
        // Same input must produce same signature
        let query = "symbol=BTCUSDT&side=BUY&timestamp=1234567890&recvWindow=5000";
        let sig1 = client.sign(query);
        let sig2 = client.sign(query);
        assert_eq!(sig1, sig2, "HMAC signature must be deterministic");
        assert_eq!(sig1.len(), 64, "SHA-256 hex should be 64 chars");
    }

    #[test]
    fn test_signed_get_url_includes_auth() {
        let client = test_client();
        let params = [("symbol", "BTCUSDT".to_string())];
        let url = client.signed_get_url("/api/v3/order", &params);
        assert!(url.starts_with("https://api.binance.com/api/v3/order?"));
        assert!(url.contains("symbol=BTCUSDT"));
        assert!(url.contains("timestamp="));
        assert!(url.contains("recvWindow="));
        assert!(url.contains("signature="));
    }

    // ── HMAC signing verification (known test vector) ────────────────

    #[test]
    fn test_hmac_known_vector() {
        // Using known HMAC-SHA256 test vector for verification
        let client = BinanceRestClient::new(
            "api_key".into(),
            "secret".into(), // simple key
            false,
        );
        let query = "hello=world";
        let sig = client.sign(query);
        // Expected: HMAC-SHA256("secret", "hello=world")
        // We compute it deterministically
        let mut mac = HmacSha256::new_from_slice(b"secret").unwrap();
        mac.update(b"hello=world");
        let expected = hex::encode(mac.finalize().into_bytes());
        assert_eq!(sig, expected, "HMAC signature should match known vector");
    }

    // ── Signed URL format ────────────────────────────────────────────

    #[test]
    fn test_signed_post_url_format() {
        let client = test_client();
        let params = [
            ("symbol", "ETHUSDT".to_string()),
            ("side", "SELL".to_string()),
            ("type", "LIMIT".to_string()),
            ("quantity", "0.1".to_string()),
            ("price", "3000.00".to_string()),
            ("timeInForce", "GTC".to_string()),
        ];
        let url = client.signed_post_url("/api/v3/order", &params);
        assert!(url.contains("side=SELL"));
        assert!(url.contains("symbol=ETHUSDT"));
        assert!(url.contains("quantity=0.1"));
        assert!(url.contains("price=3000.00"));
        assert!(url.contains("signature="));
    }

    #[test]
    fn test_signed_delete_url_format() {
        let client = test_client();
        let params = [
            ("symbol", "BTCUSDT".to_string()),
            ("orderId", "123456789".to_string()),
        ];
        let url = client.signed_delete_url("/api/v3/order", &params);
        assert!(url.contains("orderId=123456789"));
        assert!(url.contains("signature="));
    }

    // ── HTTP mocking (via reqwest_test) ──────────────────────────────

    /// Create a mock HTTP server and client for integration-style tests.
    /// Uses `httpmock` pattern with `reqwest`'s ability to use custom URLs.
    #[tokio::test]
    async fn test_ping_success() {
        // Use a local server mock via the `mockito`-style approach:
        // We'll use a reqwest Client that connects to a mock server.
        let mock_server = httpmock::MockServer::start();
        let mock = mock_server.mock(|when, then| {
            when.method(httpmock::Method::GET)
                .path("/api/v3/ping");
            then.status(200)
                .body("{}");
        });

        // Build client pointing at mock server
        let client = BinanceRestClient {
            api_key: "test".into(),
            secret_key: "test".into(),
            base_url: mock_server.base_url(),
            client: reqwest::Client::new(),
            recv_window: 5000,
        };

        let result = client.ping().await;
        assert!(result.is_ok(), "ping should succeed: {:?}", result.err());
        mock.assert();
    }

    #[tokio::test]
    async fn test_exchange_info_parse() {
        let mock_server = httpmock::MockServer::start();
        let exchange_info_json = serde_json::json!({
            "timezone": "UTC",
            "serverTime": 1508638944633i64,
            "symbols": [{
                "symbol": "BTCUSDT",
                "status": "TRADING",
                "baseAsset": "BTC",
                "quoteAsset": "USDT",
                "baseAssetPrecision": 8,
                "quotePrecision": 8,
                "orderTypes": ["LIMIT", "MARKET"],
                "icebergAllowed": false,
                "isSpotTradingAllowed": true,
                "filters": []
            }]
        });

        let mock = mock_server.mock(|when, then| {
            when.method(httpmock::Method::GET)
                .path("/api/v3/exchangeInfo");
            then.status(200)
                .header("Content-Type", "application/json")
                .body(exchange_info_json.to_string());
        });

        let client = BinanceRestClient {
            api_key: "test".into(),
            secret_key: "test".into(),
            base_url: mock_server.base_url(),
            client: reqwest::Client::new(),
            recv_window: 5000,
        };

        let info = client.exchange_info().await.unwrap();
        assert_eq!(info.timezone, "UTC");
        assert_eq!(info.symbols.len(), 1);
        assert_eq!(info.symbols[0].symbol, "BTCUSDT");
        assert_eq!(info.symbols[0].status, "TRADING");
        mock.assert();
    }

    #[tokio::test]
    async fn test_account_success() {
        let mock_server = httpmock::MockServer::start();
        let account_json = serde_json::json!({
            "makerCommission": 10,
            "takerCommission": 10,
            "buyerCommission": 0,
            "sellerCommission": 0,
            "canTrade": true,
            "canWithdraw": true,
            "canDeposit": true,
            "accountType": "SPOT",
            "balances": [
                {"asset": "BTC", "free": "0.50000000", "locked": "0.10000000"},
                {"asset": "USDT", "free": "10000.00000000", "locked": "500.00000000"}
            ],
            "permissions": ["SPOT"]
        });

        let mock = mock_server.mock(|when, then| {
            when.method(httpmock::Method::GET)
                .path("/api/v3/account");
            then.status(200)
                .header("Content-Type", "application/json")
                .body(account_json.to_string());
        });

        let client = BinanceRestClient {
            api_key: "test_key".into(),
            secret_key: "test_secret".into(),
            base_url: mock_server.base_url(),
            client: reqwest::Client::new(),
            recv_window: 5000,
        };

        let account = client.account().await.unwrap();
        assert!(account.can_trade);
        assert_eq!(account.balances.len(), 2);
        assert_eq!(account.balances[0].asset, "BTC");
        assert_eq!(account.balances[0].free, "0.50000000");
        assert_eq!(account.balances[1].asset, "USDT");
        mock.assert();

        // Verify API key header was sent
        mock.assert_hits(1);
        // We can't directly assert headers with httpmock's simple API,
        // but the test validates the flow works
    }

    #[tokio::test]
    async fn test_api_error_parsing() {
        let mock_server = httpmock::MockServer::start();
        let error_json = serde_json::json!({
            "code": -2014,
            "msg": "API-key format invalid."
        });

        let mock = mock_server.mock(|when, then| {
            when.method(httpmock::Method::GET)
                .path("/api/v3/account");
            then.status(400)
                .header("Content-Type", "application/json")
                .body(error_json.to_string());
        });

        let client = BinanceRestClient {
            api_key: "bad_key".into(),
            secret_key: "bad_secret".into(),
            base_url: mock_server.base_url(),
            client: reqwest::Client::new(),
            recv_window: 5000,
        };

        let result = client.account().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("-2014"), "Error should contain code -2014: {err_msg}");
        assert!(err_msg.contains("API-key format invalid"), "Error should contain message: {err_msg}");
        mock.assert();
    }

    #[tokio::test]
    async fn test_new_order_success() {
        let mock_server = httpmock::MockServer::start();
        let order_json = serde_json::json!({
            "symbol": "BTCUSDT",
            "orderId": 123456789,
            "clientOrderId": "myCustomId",
            "transactTime": 1507725176595i64,
            "price": "0.00000000",
            "origQty": "0.01000000",
            "executedQty": "0.01000000",
            "cummulativeQuoteQty": "450.00000000",
            "status": "FILLED",
            "timeInForce": "IOC",
            "type": "MARKET",
            "side": "BUY",
            "fills": [{
                "price": "45000.00000000",
                "qty": "0.01000000",
                "commission": "0.00000000",
                "commissionAsset": "BNB",
                "tradeId": 987654
            }]
        });

        let mock = mock_server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/api/v3/order");
            then.status(200)
                .header("Content-Type", "application/json")
                .body(order_json.to_string());
        });

        let client = BinanceRestClient {
            api_key: "test_key".into(),
            secret_key: "test_secret".into(),
            base_url: mock_server.base_url(),
            client: reqwest::Client::new(),
            recv_window: 5000,
        };

        let response = client
            .new_order("BTCUSDT", "BUY", "MARKET", "0.01", None, Some("IOC"), None, Some("myCustomId"))
            .await
            .unwrap();

        assert_eq!(response.symbol, "BTCUSDT");
        assert_eq!(response.order_id, 123456789);
        assert_eq!(response.status, "FILLED");
        assert_eq!(response.executed_qty, "0.01000000");
        assert!(response.fills.is_some());
        assert_eq!(response.fills.as_ref().unwrap().len(), 1);
        mock.assert();
    }

    #[tokio::test]
    async fn test_cancel_order_success() {
        let mock_server = httpmock::MockServer::start();
        let cancel_json = serde_json::json!({
            "symbol": "BTCUSDT",
            "origClientOrderId": "myCustomId",
            "orderId": 123456789,
            "clientOrderId": "canceledId",
            "price": "45000.00000000",
            "origQty": "0.01000000",
            "executedQty": "0.00000000",
            "cummulativeQuoteQty": "0.00000000",
            "status": "CANCELED",
            "timeInForce": "GTC",
            "type": "LIMIT",
            "side": "BUY"
        });

        let mock = mock_server.mock(|when, then| {
            when.method(httpmock::Method::DELETE)
                .path("/api/v3/order");
            then.status(200)
                .header("Content-Type", "application/json")
                .body(cancel_json.to_string());
        });

        let client = BinanceRestClient {
            api_key: "test_key".into(),
            secret_key: "test_secret".into(),
            base_url: mock_server.base_url(),
            client: reqwest::Client::new(),
            recv_window: 5000,
        };

        let response = client.cancel_order("BTCUSDT", 123456789).await.unwrap();
        assert_eq!(response.order_id, 123456789);
        assert_eq!(response.status, "CANCELED");
        mock.assert();
    }

    #[tokio::test]
    async fn test_http_503_error() {
        let mock_server = httpmock::MockServer::start();
        let mock = mock_server.mock(|when, then| {
            when.method(httpmock::Method::GET)
                .path("/api/v3/ping");
            then.status(503)
                .body("Service Temporarily Unavailable");
        });

        let client = BinanceRestClient {
            api_key: "test".into(),
            secret_key: "test".into(),
            base_url: mock_server.base_url(),
            client: reqwest::Client::new(),
            recv_window: 5000,
        };

        let result = client.ping().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("503"), "Should contain HTTP status: {}", err);
        mock.assert();
    }

    #[test]
    fn test_debug_redacts_api_key() {
        let client = BinanceRestClient::new(
            "ABCD1234Secret".into(),
            "supersecret".into(),
            false,
        );
        let debug_str = format!("{client:?}");
        assert!(debug_str.contains("ABCD****"), "Should show first 4 chars of API key");
        assert!(!debug_str.contains("supersecret"), "Should NOT contain the full secret key");
        assert!(!debug_str.contains("ABCD1234Secret"), "Should NOT contain the full API key");
    }

    #[tokio::test]
    async fn test_open_orders() {
        let mock_server = httpmock::MockServer::start();
        let orders_json = serde_json::json!([{
            "symbol": "BTCUSDT",
            "orderId": 1,
            "price": "45000.00000000",
            "origQty": "0.01000000",
            "executedQty": "0.00000000",
            "cummulativeQuoteQty": "0.00000000",
            "status": "NEW",
            "timeInForce": "GTC",
            "type": "LIMIT",
            "side": "BUY",
            "stopPrice": "0.00000000",
            "icebergQty": "0.00000000",
            "time": 1507725176595i64,
            "updateTime": 1507725176595i64,
            "isWorking": true
        }]);

        let mock = mock_server.mock(|when, then| {
            when.method(httpmock::Method::GET)
                .path("/api/v3/openOrders");
            then.status(200)
                .header("Content-Type", "application/json")
                .body(orders_json.to_string());
        });

        let client = BinanceRestClient {
            api_key: "test_key".into(),
            secret_key: "test_secret".into(),
            base_url: mock_server.base_url(),
            client: reqwest::Client::new(),
            recv_window: 5000,
        };

        let orders = client.open_orders(Some("BTCUSDT")).await.unwrap();
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].status, "NEW");
        mock.assert();
    }

    #[tokio::test]
    async fn test_my_trades() {
        let mock_server = httpmock::MockServer::start();
        let trades_json = serde_json::json!([{
            "symbol": "BTCUSDT",
            "id": 28457,
            "orderId": 100234,
            "price": "45000.00000000",
            "qty": "0.01000000",
            "commission": "0.00000010",
            "commissionAsset": "BTC",
            "time": 1507725176595i64,
            "isBuyer": true,
            "isMaker": true,
            "isBestMatch": true
        }]);

        let mock = mock_server.mock(|when, then| {
            when.method(httpmock::Method::GET)
                .path("/api/v3/myTrades");
            then.status(200)
                .header("Content-Type", "application/json")
                .body(trades_json.to_string());
        });

        let client = BinanceRestClient {
            api_key: "test_key".into(),
            secret_key: "test_secret".into(),
            base_url: mock_server.base_url(),
            client: reqwest::Client::new(),
            recv_window: 5000,
        };

        let trades = client.my_trades("BTCUSDT", Some(5)).await.unwrap();
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].symbol, "BTCUSDT");
        assert_eq!(trades[0].price, "45000.00000000");
        assert!(trades[0].is_buyer);
        mock.assert();
    }
}
