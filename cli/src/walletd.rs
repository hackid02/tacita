//! Minimal JSON-RPC client for the Tari Ootle wallet daemon (tari_ootle_walletd).
//!
//! Auth uses an API key created in the wallet's web UI ("API Keys" page). The key is sent as
//! `Authorization: Bearer <key>` on every call.

use serde_json::{json, Value};

pub struct Walletd {
    url: String,
    api_key: String,
}

impl Walletd {
    pub fn new(url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self { url: url.into(), api_key: api_key.into() }
    }

    pub fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        let resp = ureq::post(&self.url)
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .send_json(body);
        let resp = match resp {
            Ok(r) => r,
            Err(ureq::Error::Status(code, r)) => {
                let text = r.into_string().unwrap_or_default();
                return Err(format!("wallet answered HTTP {code}: {text}"));
            },
            Err(e) => return Err(format!("could not reach the wallet at {}: {e}", self.url)),
        };
        let v: Value = resp.into_json().map_err(|e| format!("bad JSON from wallet: {e}"))?;
        if let Some(err) = v.get("error") {
            return Err(format!("wallet error in {method}: {err}"));
        }
        v.get("result").cloned().ok_or_else(|| format!("no result in {method} response"))
    }
}
