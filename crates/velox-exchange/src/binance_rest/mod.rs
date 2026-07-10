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
//!
//! # Re-exports
//!
//! All response types are available from [`types`] and re-exported at
//! the crate level for convenience.

pub mod types;

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::fmt;

use crate::ExchangeError;

type HmacSha256 = Hmac<Sha256>;

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
            types::BINANCE_TESTNET_REST_URL.to_string()
        } else {
            types::BINANCE_REST_URL.to_string()
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
            recv_window: types::DEFAULT_RECV_WINDOW,
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
    pub async fn exchange_info(&self) -> Result<types::BinanceExchangeInfo, ExchangeError> {
        let url = format!("{}/api/v3/exchangeInfo", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ExchangeError::Http(e.to_string()))?;

        self.parse_response::<types::BinanceExchangeInfo>(resp).await
    }

    // ── Signed endpoints (HMAC-SHA256 auth) ──────────────────────────

    /// Get account information (`GET /api/v3/account`).
    pub async fn account(&self) -> Result<types::BinanceAccountInfo, ExchangeError> {
        let url = self.signed_get_url("/api/v3/account", &[]);
        let resp = self
            .client
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| ExchangeError::Http(e.to_string()))?;

        self.parse_response::<types::BinanceAccountInfo>(resp).await
    }

    /// Place a new order (`POST /api/v3/order`).
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
    ) -> Result<types::BinanceOrderResponse, ExchangeError> {
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

        self.parse_response::<types::BinanceOrderResponse>(resp).await
    }

    /// Cancel an order (`DELETE /api/v3/order`).
    pub async fn cancel_order(
        &self,
        symbol: &str,
        order_id: i64,
    ) -> Result<types::BinanceCancelledOrder, ExchangeError> {
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

        self.parse_response::<types::BinanceCancelledOrder>(resp).await
    }

    /// Query an order's status (`GET /api/v3/order`).
    pub async fn get_order(
        &self,
        symbol: &str,
        order_id: i64,
    ) -> Result<types::BinanceOrderResponse, ExchangeError> {
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

        self.parse_response::<types::BinanceOrderResponse>(resp).await
    }

    /// Get all open orders for a symbol (`GET /api/v3/openOrders`).
    pub async fn open_orders(
        &self,
        symbol: Option<&str>,
    ) -> Result<Vec<types::BinanceOrderResponse>, ExchangeError> {
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

        self.parse_response::<Vec<types::BinanceOrderResponse>>(resp).await
    }

    // ── User Data Stream (listen key) ─────────────────────────────

    /// Create a listen key (`POST /api/v3/userDataStream`).
    pub async fn create_listen_key(&self) -> Result<String, ExchangeError> {
        let url = format!("{}/api/v3/userDataStream", self.base_url);
        let resp = self
            .client
            .post(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| ExchangeError::Http(e.to_string()))?;

        #[derive(serde::Deserialize)]
        struct ListenKeyResponse {
            listen_key: String,
        }

        let body: ListenKeyResponse = self.parse_response(resp).await?;
        Ok(body.listen_key)
    }

    /// Keep a listen key alive (`PUT /api/v3/userDataStream`).
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
    ) -> Result<Vec<types::BinanceTrade>, ExchangeError> {
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

        self.parse_response::<Vec<types::BinanceTrade>>(resp).await
    }

    // ── Internal helpers ─────────────────────────────────────────────

    /// Build a signed GET URL with HMAC-SHA256 signature.
    pub(crate) fn signed_get_url(&self, path: &str, params: &[(&str, String)]) -> String {
        let query = self.build_query(params);
        let signature = self.sign(&query);
        format!("{}{}?{}&signature={}", self.base_url, path, query, signature)
    }

    /// Build a signed POST URL.
    pub(crate) fn signed_post_url(&self, path: &str, params: &[(&str, String)]) -> String {
        self.signed_get_url(path, params)
    }

    /// Build a signed DELETE URL.
    pub(crate) fn signed_delete_url(&self, path: &str, params: &[(&str, String)]) -> String {
        self.signed_get_url(path, params)
    }

    /// Build a query string from parameters, sorted alphabetically by key,
    /// with `timestamp` and `recvWindow` appended.
    pub(crate) fn build_query(&self, params: &[(&str, String)]) -> String {
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
    pub(crate) fn sign(&self, query: &str) -> String {
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

        if let Ok(binance_err) = serde_json::from_str::<types::BinanceApiError>(&body) {
            return ExchangeError::Exchange(format!(
                "Binance API error [code {}]: {}",
                binance_err.code, binance_err.msg
            ));
        }

        ExchangeError::Http(format!("HTTP {}: {}", status.as_u16(), body))
    }
}

impl fmt::Debug for BinanceRestClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let masked = format!("{}****", &self.api_key[..4.min(self.api_key.len())]);
        f.debug_struct("BinanceRestClient")
            .field("base_url", &self.base_url)
            .field("recv_window", &self.recv_window)
            .field("api_key (masked)", &masked)
            .finish()
    }
}

// ── Test module ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
