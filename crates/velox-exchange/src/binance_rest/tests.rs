//! Tests for Binance REST client — query building, HMAC signing, and HTTP mocking.

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::binance_rest::BinanceRestClient;

type HmacSha256 = Hmac<Sha256>;

/// Helper: create a test client with mock HTTP backend.
fn test_client() -> BinanceRestClient {
    BinanceRestClient::new("test_api_key".into(), "test_secret_key".into(), false)
}

// ── Build query / sign tests ─────────────────────────────────────

#[test]
fn test_build_query_empty_params() {
    let client = test_client();
    let query = client.build_query(&[]);
    assert!(query.contains("timestamp="), "query={query}");
    assert!(query.contains("recvWindow="), "query={query}");
    assert!(
        query.starts_with("timestamp=") || query.contains("&timestamp="),
        "query should contain timestamp, got: {query}"
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
    let parts: Vec<&str> = query.split('&').collect();
    let param_part: Vec<&&str> = parts.iter()
        .filter(|p| !p.starts_with("timestamp=") && !p.starts_with("recvWindow="))
        .collect();
    assert_eq!(param_part.len(), 3);
    assert!(param_part[0].starts_with("side="));
    assert!(param_part[1].starts_with("symbol="));
    assert!(param_part[2].starts_with("type="));
}

#[test]
fn test_sign_consistency() {
    let client = test_client();
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

// ── HMAC signing verification ───────────────────────────────────

#[test]
fn test_hmac_known_vector() {
    let client = BinanceRestClient::new("api_key".into(), "secret".into(), false);
    let query = "hello=world";
    let sig = client.sign(query);
    let mut mac = HmacSha256::new_from_slice(b"secret").unwrap();
    mac.update(b"hello=world");
    let expected = hex::encode(mac.finalize().into_bytes());
    assert_eq!(sig, expected, "HMAC signature should match known vector");
}

// ── Signed URL format ───────────────────────────────────────────

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

// ── HTTP mocking ────────────────────────────────────────────────

#[tokio::test]
async fn test_ping_success() {
    let mock_server = httpmock::MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::GET).path("/api/v3/ping");
        then.status(200).body("{}");
    });

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
        when.method(httpmock::Method::GET).path("/api/v3/exchangeInfo");
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
        when.method(httpmock::Method::GET).path("/api/v3/account");
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
    mock.assert();
}

#[tokio::test]
async fn test_api_error_parsing() {
    let mock_server = httpmock::MockServer::start();
    let error_json = serde_json::json!({
        "code": -2014,
        "msg": "API-key format invalid."
    });

    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::GET).path("/api/v3/account");
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
        when.method(httpmock::Method::POST).path("/api/v3/order");
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
    assert!(response.fills.is_some());
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
        when.method(httpmock::Method::DELETE).path("/api/v3/order");
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
        when.method(httpmock::Method::GET).path("/api/v3/ping");
        then.status(503).body("Service Temporarily Unavailable");
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
    let client = BinanceRestClient::new("ABCD1234Secret".into(), "supersecret".into(), false);
    let debug_str = format!("{client:?}");
    assert!(debug_str.contains("ABCD****"), "Should show first 4 chars of API key");
    assert!(!debug_str.contains("supersecret"), "Should NOT contain the full secret key");
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
        when.method(httpmock::Method::GET).path("/api/v3/openOrders");
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
        when.method(httpmock::Method::GET).path("/api/v3/myTrades");
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
