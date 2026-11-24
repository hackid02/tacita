//! move hidden (stealth) TARI into the account's revealed balance.
//!
//! campaign funding and fees come from the revealed balance, while claimed burns arrive as hidden
//! outputs. this asks the wallet for a stealth transfer to the account's own address that spends
//! only hidden funds and pays the whole amount out as revealed.

use serde_json::{json, Value};

/// TARI's resource address on ootle (the stealth TARI resource).
pub const TARI_RESOURCE: &str = "resource_0101010101010101010101010101010101010101010101010101010101010101";

pub fn request(account: &str, own_address: &str, micro: u64, max_fee: u64) -> Value {
    json!({
        "owner_account": { "ComponentAddress": account },
        "fee_params": { "input_selection": "ConfidentialOnly", "pay_fee_with_swap": null },
        "input_selection": "ConfidentialOnly",
        "resource_address": TARI_RESOURCE,
        "transfers": [{
            "destination_address": own_address,
            "blinded_output_amount": 0,
            "revealed_output_amount": micro.to_string(),
            "pay_to": "StealthPublicKey",
            "attach_sender_address": false
        }],
        "max_fee": max_fee,
        "dry_run": false
    })
}

#[cfg(test)]
mod tests {
    use ootle_rs::{key_provider::PrivateKeyProvider, transaction::TransactionSigner, Network};
    use tari_ootle_walletd_client::types::StealthTransferRequest;

    use super::*;

    #[test]
    fn the_wallet_accepts_this_request_shape() {
        let me = PrivateKeyProvider::random(Network::MainNet).address().to_string();
        let v = request("component_5286446acff808c0fcad494ce7b74e19238b446b02394242328d974261d3fe36", &me, 1_500_000, 50_000);
        let parsed: StealthTransferRequest = serde_json::from_value(v.clone()).unwrap_or_else(|e| panic!("{e}\n{v:#}"));
        let back = serde_json::to_value(&parsed).unwrap();
        assert_eq!(back["transfers"][0]["revealed_output_amount"], json!("1500000"), "{back:#}");
        assert_eq!(back["resource_address"], json!(TARI_RESOURCE), "{back:#}");
    }
}
